use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use pulldown_cmark::{html, Options, Parser};
use walkdir::WalkDir;

const DEFAULT_TEMPLATE: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
    {{robots_meta}}
  <title>{{title}}</title>
  <style>
        :root {
            --bg: #0f1117;
            --panel: #141925;
            --panel-soft: #181e2d;
            --text: #d6deeb;
            --dim: #8a94a6;
            --border: #283247;
            --accent: #82aaff;
            --accent-2: #c792ea;
        }
        * { box-sizing: border-box; }
        body {
            max-width: 900px;
            margin: 2.25rem auto;
            padding: 0 1.1rem;
            background: var(--bg);
            color: var(--text);
            font-family: "JetBrains Mono", "SF Mono", Menlo, Monaco, Consolas, monospace;
            line-height: 1.7;
            font-size: 15px;
            -webkit-font-smoothing: antialiased;
            text-rendering: optimizeLegibility;
        }
        img {
            max-width: 100%;
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 0.2rem;
            background: var(--panel);
        }
        pre {
            background: var(--panel);
            border: 1px solid var(--border);
            border-radius: 10px;
            padding: 1rem;
            overflow-x: auto;
        }
        code {
            font-family: inherit;
            color: var(--accent-2);
        }
        h1, h2, h3 {
            color: #e5ecf8;
            line-height: 1.3;
            margin-top: 1.6rem;
        }
        a {
            color: var(--accent);
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
            text-underline-offset: 2px;
        }
        nav {
            margin-bottom: 1.2rem;
            padding: 0.7rem 0.85rem;
            border: 1px solid var(--border);
            border-radius: 10px;
            background: var(--panel-soft);
        }
        nav > a {
            display: inline-block;
            margin-right: 1.15rem;
        }
        .breadcrumbs {
            font-size: 0.9em;
            margin-bottom: 1.2rem;
            color: var(--dim);
            padding: 0.55rem 0.75rem;
            border: 1px solid var(--border);
            border-radius: 8px;
            background: var(--panel);
        }
        .breadcrumbs a { color: var(--text); }
        .breadcrumbs > * + *::before { content: " › "; color: var(--dim); }
        .blog-list {
            display: grid;
            gap: 0.9rem;
            margin-top: 1rem;
        }
        .post-card {
            border: 1px solid var(--border);
            border-radius: 10px;
            background: var(--panel);
            padding: 0.9rem 1rem;
        }
        .card-title {
            margin: 0 0 0.25rem;
            font-size: 1.08rem;
        }
        .card-subheading {
            margin: 0 0 0.4rem;
            color: var(--accent-2);
            font-size: 0.95rem;
        }
        .card-excerpt {
            margin: 0;
            color: var(--dim);
            font-size: 0.95rem;
        }
        .blog-post .post-header {
            border-bottom: 1px solid var(--border);
            margin-bottom: 1rem;
            padding-bottom: 0.8rem;
        }
        .blog-post .post-title {
            margin: 0;
        }
        .blog-post .post-subheading {
            margin: 0.45rem 0 0;
            color: var(--accent-2);
            font-size: 1rem;
        }
        .pagination {
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 0.75rem;
            margin-top: 1rem;
            padding: 0.6rem 0.8rem;
            border: 1px solid var(--border);
            border-radius: 8px;
            background: var(--panel-soft);
        }
        .page-link {
            color: var(--accent);
            white-space: nowrap;
        }
        .page-current {
            color: var(--dim);
            font-size: 0.9rem;
            margin: 0 auto;
        }
  </style>
</head>
<body>
  {{nav}}
  {{breadcrumbs}}
  {{content}}
</body>
</html>
"#;

const BLOG_POSTS_PER_PAGE_DEFAULT: usize = 5;

#[derive(Clone)]
#[allow(dead_code)]
struct PageMeta {
    rel_path: PathBuf,
    section: Option<String>,
    title: String,
}

#[derive(Clone)]
struct BlogPostMeta {
    rel_path: PathBuf,
    title: String,
    subtitle: Option<String>,
    excerpt: String,
}

type SiteStructure = BTreeMap<Option<String>, Vec<PageMeta>>;

fn main() -> Result<()> {
    let content_dir = Path::new("content");
    let output_dir = Path::new("public");
    let template_path = Path::new("templates/page.html");
    let base_url = base_url_from_env();
    let noindex = noindex_from_env();
    let robots_meta = robots_meta_tag(noindex);
    let blog_posts_per_page = blog_posts_per_page_from_env();

    let template = load_template(template_path)?;

    if !content_dir.exists() {
        anyhow::bail!(
            "Content directory not found: {}. Create it and add markdown files.",
            content_dir.display()
        );
    }

    if output_dir.exists() {
        fs::remove_dir_all(output_dir)
            .with_context(|| format!("failed to clean output directory: {}", output_dir.display()))?;
    }
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create output directory: {}", output_dir.display()))?;

    let mut site_structure: SiteStructure = BTreeMap::new();
    let mut page_list: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut markdown_sources: HashMap<PathBuf, String> = HashMap::new();
    let mut blog_posts: Vec<BlogPostMeta> = Vec::new();
    let mut blog_intro_markdown: Option<String> = None;

    for entry in WalkDir::new(content_dir).into_iter().filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if !path.is_file() || !is_markdown_file(path) {
            continue;
        }

        let relative = path
            .strip_prefix(content_dir)
            .with_context(|| format!("failed to resolve relative path for {}", path.display()))?
            .to_path_buf();

        let markdown = fs::read_to_string(path)
            .with_context(|| format!("failed to read markdown file: {}", path.display()))?;
        let title = extract_title(&markdown).unwrap_or_else(|| fallback_title(&relative));
        let section = get_section(&relative);

        site_structure
            .entry(section.clone())
            .or_insert_with(Vec::new)
            .push(PageMeta {
                rel_path: relative.clone(),
                section,
                title: title.clone(),
            });

        if is_blog_index_page(&relative) {
            blog_intro_markdown = Some(markdown.clone());
        }

        if is_blog_post_page(&relative) {
            blog_posts.push(BlogPostMeta {
                rel_path: relative.clone(),
                title,
                subtitle: extract_subheading(&markdown),
                excerpt: extract_excerpt(&markdown),
            });
        }

        if !is_blog_index_page(&relative) {
            let mut target = output_dir.join(&relative);
            target.set_extension("html");
            page_list.push((relative.clone(), target));
        }

        markdown_sources.insert(relative, markdown);
    }

    let mut generated_count = 0usize;
    let mut copied_count = 0usize;

    for (relative, target) in page_list {
        let markdown = markdown_sources
            .get(&relative)
            .with_context(|| format!("missing markdown source for {}", relative.display()))?;

        let title = extract_title(markdown).unwrap_or_else(|| fallback_title(&relative));
        let section = get_section(&relative);

        let html_content = if is_blog_post_page(&relative) {
            let body_markdown = strip_title_and_subheading(markdown);
            let body_html = markdown_to_html(&body_markdown);
            render_blog_post_content(&title, extract_subheading(markdown).as_deref(), &body_html)
        } else {
            markdown_to_html(markdown)
        };

        let nav_html = render_nav(&site_structure, &section, &base_url);
        let breadcrumbs_html = render_breadcrumbs(&relative, &section, &base_url);
        let page = render_page(
            &template,
            &title,
            &html_content,
            &nav_html,
            &breadcrumbs_html,
            &robots_meta,
        );

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory: {}", parent.display()))?;
        }

        fs::write(&target, page)
            .with_context(|| format!("failed to write output file: {}", target.display()))?;

        generated_count += 1;
        println!("generated {}", target.display());
    }

    generate_blog_listing_pages(
        &output_dir,
        &template,
        &site_structure,
        &base_url,
        &robots_meta,
        &blog_intro_markdown,
        &mut blog_posts,
        blog_posts_per_page,
        &mut generated_count,
    )?;

    for entry in WalkDir::new(content_dir).into_iter().filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if !path.is_file() || is_markdown_file(path) || is_hidden_file(path) {
            continue;
        }

        let relative = path
            .strip_prefix(content_dir)
            .with_context(|| format!("failed to resolve relative path for {}", path.display()))?;
        let target = output_dir.join(relative);

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory: {}", parent.display()))?;
        }

        fs::copy(path, &target).with_context(|| {
            format!(
                "failed to copy asset from {} to {}",
                path.display(),
                target.display()
            )
        })?;

        copied_count += 1;
        println!("copied {}", target.display());
    }

    let robots_txt = robots_txt_content(noindex);
    fs::write(output_dir.join("robots.txt"), robots_txt)
        .with_context(|| "failed to write robots.txt".to_string())?;

    println!(
        "done: generated {} page(s), copied {} asset file(s)",
        generated_count, copied_count
    );
    Ok(())
}

fn load_template(template_path: &Path) -> Result<String> {
    if template_path.exists() {
        fs::read_to_string(template_path)
            .with_context(|| format!("failed to read template: {}", template_path.display()))
    } else {
        Ok(DEFAULT_TEMPLATE.to_string())
    }
}

fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown"))
        .unwrap_or(false)
}

fn is_hidden_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

fn base_url_from_env() -> String {
    let raw_base_url = std::env::var("BASE_URL").unwrap_or_default();
    normalize_base_url(&raw_base_url)
}

fn noindex_from_env() -> bool {
    let value = std::env::var("NOINDEX").unwrap_or_else(|_| "0".to_string());
    matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

fn robots_meta_tag(noindex: bool) -> String {
    if noindex {
        r#"<meta name="robots" content="noindex,nofollow,noarchive">"#.to_string()
    } else {
        String::new()
    }
}

fn robots_txt_content(noindex: bool) -> &'static str {
    if noindex {
        "User-agent: *\nDisallow: /\n"
    } else {
        "User-agent: *\nAllow: /\n"
    }
}

fn blog_posts_per_page_from_env() -> usize {
    std::env::var("BLOG_POSTS_PER_PAGE")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(BLOG_POSTS_PER_PAGE_DEFAULT)
}

fn is_blog_index_page(rel_path: &Path) -> bool {
    rel_path == Path::new("blog/index.md")
}

fn is_blog_post_page(rel_path: &Path) -> bool {
    let mut components = rel_path.components();
    let first = components.next().and_then(|c| c.as_os_str().to_str());

    if first != Some("blog") {
        return false;
    }

    rel_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem != "index")
        .unwrap_or(false)
}

fn extract_subheading(markdown: &str) -> Option<String> {
    markdown
        .lines()
        .find_map(|line| line.strip_prefix("## ").map(str::trim))
        .filter(|subtitle| !subtitle.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_excerpt(markdown: &str) -> String {
    markdown
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "Read more…".to_string())
}

fn strip_title_and_subheading(markdown: &str) -> String {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut idx = 0usize;

    while idx < lines.len() && lines[idx].trim().is_empty() {
        idx += 1;
    }

    if idx < lines.len() && lines[idx].trim_start().starts_with("# ") {
        idx += 1;

        while idx < lines.len() && lines[idx].trim().is_empty() {
            idx += 1;
        }

        if idx < lines.len() && lines[idx].trim_start().starts_with("## ") {
            idx += 1;

            while idx < lines.len() && lines[idx].trim().is_empty() {
                idx += 1;
            }
        }
    } else {
        idx = 0;
    }

    lines[idx..].join("\n")
}

fn render_blog_post_content(title: &str, subtitle: Option<&str>, body_html: &str) -> String {
    let subtitle_html = subtitle
        .map(|text| format!("<p class=\"post-subheading\">{}</p>", escape_html(text)))
        .unwrap_or_default();

    format!(
        "<article class=\"blog-post\"><header class=\"post-header\"><h1 class=\"post-title\">{}</h1>{}</header><div class=\"post-body\">{}</div></article>",
        escape_html(title),
        subtitle_html,
        body_html
    )
}

fn render_blog_index_cards(posts: &[BlogPostMeta], base_url: &str) -> String {
    if posts.is_empty() {
        return "<section class=\"blog-list\"><p>No posts yet.</p></section>".to_string();
    }

    let mut cards = String::from("<section class=\"blog-list\">");

    for post in posts {
        let mut post_output = post.rel_path.clone();
        post_output.set_extension("html");
        let mut post_href = String::from("/");
        post_href.push_str(&post_output.to_string_lossy().replace('\\', "/"));
        let post_href = with_base_url(base_url, &post_href);

        let subtitle = post
            .subtitle
            .as_ref()
            .map(|text| format!("<p class=\"card-subheading\">{}</p>", escape_html(text)))
            .unwrap_or_default();

        cards.push_str(&format!(
            "<article class=\"post-card\"><h2 class=\"card-title\"><a href=\"{}\">{}</a></h2>{}<p class=\"card-excerpt\">{}</p></article>",
            post_href,
            escape_html(&post.title),
            subtitle,
            escape_html(&post.excerpt)
        ));
    }

    cards.push_str("</section>");
    cards
}

fn blog_page_href(page_num: usize, base_url: &str) -> String {
    let path = if page_num == 1 {
        "/blog/index.html".to_string()
    } else {
        format!("/blog/page/{}/index.html", page_num)
    };

    with_base_url(base_url, &path)
}

fn render_blog_pagination(current_page: usize, total_pages: usize, base_url: &str) -> String {
    if total_pages <= 1 {
        return String::new();
    }

    let mut pagination = String::from("<nav class=\"pagination\" aria-label=\"Blog pagination\">");

    if current_page > 1 {
        pagination.push_str(&format!(
            "<a class=\"page-link\" href=\"{}\">← Newer</a>",
            blog_page_href(current_page - 1, base_url)
        ));
    }

    pagination.push_str(&format!(
        "<span class=\"page-current\">Page {} of {}</span>",
        current_page, total_pages
    ));

    if current_page < total_pages {
        pagination.push_str(&format!(
            "<a class=\"page-link\" href=\"{}\">Older →</a>",
            blog_page_href(current_page + 1, base_url)
        ));
    }

    pagination.push_str("</nav>");
    pagination
}

fn render_blog_breadcrumbs(page_num: usize, base_url: &str) -> String {
    let home_url = with_base_url(base_url, "/");
    let blog_url = blog_page_href(1, base_url);

    if page_num == 1 {
        return format!(
            "<div class=\"breadcrumbs\"><a href=\"{}\">Home</a> <span>Blog</span></div>",
            home_url
        );
    }

    format!(
        "<div class=\"breadcrumbs\"><a href=\"{}\">Home</a> <a href=\"{}\">Blog</a> <span>Page {}</span></div>",
        home_url,
        blog_url,
        page_num
    )
}

fn generate_blog_listing_pages(
    output_dir: &Path,
    template: &str,
    site_structure: &SiteStructure,
    base_url: &str,
    robots_meta: &str,
    blog_intro_markdown: &Option<String>,
    blog_posts: &mut [BlogPostMeta],
    posts_per_page: usize,
    generated_count: &mut usize,
) -> Result<()> {
    blog_posts.sort_by(|a, b| b.rel_path.cmp(&a.rel_path));

    let total_posts = blog_posts.len();
    let total_pages = if total_posts == 0 {
        1
    } else {
        (total_posts + posts_per_page - 1) / posts_per_page
    };

    let intro_html = blog_intro_markdown
        .as_ref()
        .map(|markdown| markdown_to_html(markdown))
        .unwrap_or_default();
    let blog_section = Some("Blog".to_string());

    for page_num in 1..=total_pages {
        let page_title = if page_num == 1 {
            "Blog".to_string()
        } else {
            format!("Blog - Page {}", page_num)
        };

        let cards_html = if total_posts == 0 {
            render_blog_index_cards(&[], base_url)
        } else {
            let start = (page_num - 1) * posts_per_page;
            let end = usize::min(start + posts_per_page, total_posts);
            render_blog_index_cards(&blog_posts[start..end], base_url)
        };

        let pagination_html = render_blog_pagination(page_num, total_pages, base_url);
        let page_content = format!("{}{}{}", intro_html, cards_html, pagination_html);

        let nav_html = render_nav(site_structure, &blog_section, base_url);
        let breadcrumbs_html = render_blog_breadcrumbs(page_num, base_url);
        let page = render_page(
            template,
            &page_title,
            &page_content,
            &nav_html,
            &breadcrumbs_html,
            robots_meta,
        );

        let target = if page_num == 1 {
            output_dir.join("blog/index.html")
        } else {
            output_dir.join(format!("blog/page/{}/index.html", page_num))
        };

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory: {}", parent.display()))?;
        }

        fs::write(&target, page)
            .with_context(|| format!("failed to write output file: {}", target.display()))?;

        *generated_count += 1;
        println!("generated {}", target.display());
    }

    Ok(())
}

fn normalize_base_url(raw_base_url: &str) -> String {
    let trimmed = raw_base_url.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return String::new();
    }

    let without_trailing_slash = trimmed.trim_end_matches('/');
    if without_trailing_slash.starts_with('/') {
        without_trailing_slash.to_string()
    } else {
        format!("/{}", without_trailing_slash)
    }
}

fn with_base_url(base_url: &str, path: &str) -> String {
    if base_url.is_empty() {
        return path.to_string();
    }

    if path == "/" {
        return format!("{}/", base_url);
    }

    format!("{}{}", base_url, path)
}

fn markdown_to_html(markdown: &str) -> String {
    let options = Options::all();
    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

fn extract_title(markdown: &str) -> Option<String> {
    markdown
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|title| !title.is_empty())
        .map(ToOwned::to_owned)
}

fn fallback_title(relative: &Path) -> String {
    let stem = relative
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Untitled");

    stem.split(['-', '_', ' '])
        .filter(|part| !part.is_empty())
        .map(capitalize)
        .collect::<Vec<_>>()
        .join(" ")
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

fn render_page(
    template: &str,
    title: &str,
    content: &str,
    nav: &str,
    breadcrumbs: &str,
    robots_meta: &str,
) -> String {
    template
        .replace("{{title}}", &escape_html(title))
        .replace("{{robots_meta}}", robots_meta)
        .replace("{{content}}", content)
        .replace("{{nav}}", nav)
        .replace("{{breadcrumbs}}", breadcrumbs)
}

fn get_section(rel_path: &Path) -> Option<String> {
    let components: Vec<_> = rel_path.components().collect();
    
    if components.len() < 2 {
        return None;
    }

    components
        .first()
        .and_then(|comp| {
            if let std::path::Component::Normal(os_str) = comp {
                os_str.to_str()
            } else {
                None
            }
        })
        .map(|s| {
            s.split(['-', '_'])
                .map(capitalize)
                .collect::<Vec<_>>()
                .join(" ")
        })
}

fn render_nav(site_structure: &SiteStructure, current_section: &Option<String>, base_url: &str) -> String {
    let home_url = with_base_url(base_url, "/");
    let mut nav = format!("<nav><a href=\"{}\">Home</a>", home_url);

    for section in site_structure.keys() {
        if section.is_none() {
            continue;
        }

        let section_label = section.as_ref().map(|s| s.as_str()).unwrap_or("Home");
        let section_url = section
            .as_ref()
            .map(|s| format!("/{}/index.html", s.to_lowercase().replace(' ', "-")))
            .unwrap_or_else(|| "/".to_string());
        let section_url = with_base_url(base_url, &section_url);

        let is_current = section == current_section;
        let style = if is_current {
            r#" style="font-weight: bold;""#
        } else {
            ""
        };

        nav.push_str(&format!(r#"<a href="{}"{}>{}</a>"#, section_url, style, escape_html(section_label)));
    }

    nav.push_str("</nav>");
    nav
}

fn render_breadcrumbs(rel_path: &Path, current_section: &Option<String>, base_url: &str) -> String {
    if rel_path == Path::new("index.md") {
        return String::new();
    }

    let home_url = with_base_url(base_url, "/");
    let mut breadcrumbs = format!(
        r#"<div class="breadcrumbs"><a href="{}">Home</a>"#,
        home_url
    );

    let components: Vec<_> = rel_path.components().collect();

    if let Some(section) = current_section {
        let section_url = section.to_lowercase().replace(' ', "-");
        let section_href = with_base_url(base_url, &format!("/{}/index.html", section_url));
        breadcrumbs.push_str(&format!(
            r#" <a href="{}">{}</a>"#,
            section_href,
            escape_html(section)
        ));
    }

    let is_index_page = rel_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem == "index")
        .unwrap_or(false);

    if components.len() > 1 && !is_index_page {
        let title = fallback_title(rel_path);
        breadcrumbs.push_str(&format!(" <span>{}</span>", escape_html(&title)));
    }

    breadcrumbs.push_str("</div>");
    breadcrumbs
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
