use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::render::{escape_html, markdown_to_html, render_page, render_template, with_base_url};
use crate::types::{BlogPostMeta, SiteStructure, Templates};

pub(crate) fn is_blog_index_page(rel_path: &Path) -> bool {
    rel_path == Path::new("blog/index.md")
}

pub(crate) fn is_blog_post_page(rel_path: &Path) -> bool {
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

pub(crate) fn extract_subheading(markdown: &str) -> Option<String> {
    split_blog_post_markdown(markdown).1
}

pub(crate) fn extract_excerpt(markdown: &str) -> String {
    markdown
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "Read more…".to_string())
}

pub(crate) fn split_blog_post_markdown(markdown: &str) -> (Option<String>, Option<String>, String) {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut idx = 0usize;
    let mut title = None;
    let mut subtitle = None;

    while idx < lines.len() && lines[idx].trim().is_empty() {
        idx += 1;
    }

    if idx < lines.len() && lines[idx].trim_start().starts_with("# ") {
        title = lines[idx]
            .trim_start()
            .strip_prefix("# ")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        idx += 1;

        while idx < lines.len() && lines[idx].trim().is_empty() {
            idx += 1;
        }

        if idx < lines.len() && lines[idx].trim_start().starts_with("## ") {
            subtitle = lines[idx]
                .trim_start()
                .strip_prefix("## ")
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);
            idx += 1;

            while idx < lines.len() && lines[idx].trim().is_empty() {
                idx += 1;
            }
        }
    } else {
        idx = 0;
    }

    (title, subtitle, lines[idx..].join("\n"))
}

pub(crate) fn render_blog_post_content(
    template: &str,
    title: &str,
    subtitle: Option<&str>,
    body_html: &str,
) -> String {
    let subtitle_html = subtitle
        .map(|text| format!("<p class=\"post-subheading\">{}</p>", escape_html(text)))
        .unwrap_or_default();

    render_template(
        template,
        &[
            ("post_title", &escape_html(title)),
            ("post_subheading_block", &subtitle_html),
            ("post_body", body_html),
        ],
    )
}

fn render_blog_index_content(
    template: &str,
    intro_html: &str,
    cards_html: &str,
    pagination_html: &str,
) -> String {
    render_template(
        template,
        &[
            ("blog_intro", intro_html),
            ("blog_posts", cards_html),
            ("pagination", pagination_html),
        ],
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

pub(crate) fn generate_blog_listing_pages(
    output_dir: &Path,
    templates: &Templates,
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
        let page_content = render_blog_index_content(
            &templates.blog_index,
            &intro_html,
            &cards_html,
            &pagination_html,
        );

        let nav_html = crate::render::render_nav(site_structure, &blog_section, base_url);
        let breadcrumbs_html = render_blog_breadcrumbs(page_num, base_url);
        let page = render_page(
            &templates.base,
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
