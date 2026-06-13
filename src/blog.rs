use std::fs;
use std::path::Path;
use std::collections::BTreeMap;

use anyhow::{Context, Result};

use crate::render::{escape_html, markdown_to_html, render_page, render_template, with_base_url};
use crate::types::{BlogPostMeta, SiteStructure, Templates};

fn slugify_tag(tag: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for ch in tag.chars().flat_map(|character| character.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}

fn parse_tags_line(line: &str) -> Vec<String> {
    line
        .strip_prefix("tags:")
        .map(|values| {
            values
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn render_tag_links(tags: &[String], base_url: &str, class_name: &str) -> String {
    if tags.is_empty() {
        return String::new();
    }

    let mut output = format!(r#"<div class="{}"><span class="tag-label">tags:</span>"#, class_name);

    for tag in tags {
        let slug = slugify_tag(tag);
        if slug.is_empty() {
            continue;
        }

        let href = blog_tag_page_href(slug.as_str(), 1, base_url);
        output.push_str(&format!(
            r#"<a class="tag-pill" href="{}">{}</a>"#,
            href,
            escape_html(tag)
        ));
    }

    output.push_str("</div>");
    output
}

fn render_blog_filter_links(
    tags: &[(String, String)],
    selected_slug: Option<&str>,
    base_url: &str,
) -> String {
    let mut filters = String::from(r#"<div class="blog-tag-filters"><span>Filter by tag</span>"#);
    let all_active = selected_slug.is_none();
    let all_style = if all_active {
        r#" class="tag-pill is-active""#
    } else {
        r#" class="tag-pill""#
    };

    filters.push_str(&format!(
        r#"<a{} href="{}">All</a>"#,
        all_style,
        with_base_url(base_url, "/blog/index.html")
    ));
    filters.push_str(&format!(
        r#"<a class="tag-pill" href="{}">All tags</a>"#,
        with_base_url(base_url, "/blog/tags/index.html")
    ));

    for (slug, label) in tags {
        let is_active = selected_slug == Some(slug.as_str());
        let style = if is_active {
            r#" class="tag-pill is-active""#
        } else {
            r#" class="tag-pill""#
        };
        let href = with_base_url(base_url, &format!("/blog/tags/{}/index.html", slug));

        filters.push_str(&format!(
            r#"<a{} href="{}">{}</a>"#,
            style,
            href,
            escape_html(label)
        ));
    }

    filters.push_str("</div>");
    filters
}

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

pub(crate) struct BlogParsed {
    pub(crate) title: Option<String>,
    pub(crate) subtitle: Option<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) published: Option<String>,
    pub(crate) edited: Option<String>,
    pub(crate) body: String,
}

pub(crate) fn extract_subheading(markdown: &str) -> Option<String> {
    split_blog_post_markdown(markdown).subtitle
}

pub(crate) fn extract_excerpt(markdown: &str) -> String {
    split_blog_post_markdown(markdown)
        .body
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "Read more…".to_string())
}

pub(crate) fn split_blog_post_markdown(markdown: &str) -> BlogParsed {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut idx = 0usize;
    let mut title = None;
    let mut subtitle = None;
    let mut tags = Vec::new();
    let mut published = None;
    let mut edited = None;

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

    while idx < lines.len() {
        let trimmed = lines[idx].trim();

        if trimmed.is_empty() {
            idx += 1;
            continue;
        }

        if trimmed.starts_with("tags:") {
            tags.extend(parse_tags_line(trimmed));
            idx += 1;
            continue;
        }

        if trimmed.starts_with("published:") {
            published = trimmed
                .strip_prefix("published:")
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToOwned::to_owned);
            idx += 1;
            continue;
        }

        if trimmed.starts_with("edited:") {
            edited = trimmed
                .strip_prefix("edited:")
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToOwned::to_owned);
            idx += 1;
            continue;
        }

        break;
    }

    while idx < lines.len() && lines[idx].trim().is_empty() {
        idx += 1;
    }

    BlogParsed {
        title,
        subtitle,
        tags,
        published,
        edited,
        body: lines[idx..].join("\n"),
    }
}

fn format_date(date_str: &str) -> String {
    const MONTHS: [&str; 12] = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ];
    let parts: Vec<&str> = date_str.splitn(3, '-').collect();
    if parts.len() == 3 {
        if let (Ok(month), Ok(day)) = (parts[1].parse::<usize>(), parts[2].parse::<u32>()) {
            if month >= 1 && month <= 12 {
                return format!("{} {}, {}", MONTHS[month - 1], day, parts[0]);
            }
        }
    }
    date_str.to_string()
}

fn render_timestamps_block(published: Option<&str>, edited: Option<&str>) -> String {
    if published.is_none() && edited.is_none() {
        return String::new();
    }

    let mut html = String::from(r#"<div class="post-timestamps">"#);

    if let Some(date) = published {
        let formatted = format_date(date);
        html.push_str(&format!(
            r#"<span class="post-date">Published: <time datetime="{}">{}</time></span>"#,
            escape_html(date),
            escape_html(&formatted)
        ));
    }

    if let Some(date) = edited {
        let formatted = format_date(date);
        html.push_str(&format!(
            r#"<span class="post-date">Last edited: <time datetime="{}">{}</time></span>"#,
            escape_html(date),
            escape_html(&formatted)
        ));
    }

    html.push_str("</div>");
    html
}

pub(crate) fn render_blog_post_content(
    template: &str,
    title: &str,
    subtitle: Option<&str>,
    tags: &[String],
    published: Option<&str>,
    edited: Option<&str>,
    body_html: &str,
) -> String {
    let subtitle_html = subtitle
        .map(|text| format!("<p class=\"post-subheading\">{}</p>", escape_html(text)))
        .unwrap_or_default();
    let timestamps_html = render_timestamps_block(published, edited);
    let tags_html = render_tag_links(tags, "", "post-tags");

    render_template(
        template,
        &[
            ("post_title", &escape_html(title)),
            ("post_subheading_block", &subtitle_html),
            ("post_timestamps_block", &timestamps_html),
            ("post_tags_block", &tags_html),
            ("post_body", body_html),
        ],
    )
}

fn render_blog_index_content(
    template: &str,
    heading_html: &str,
    intro_html: &str,
    tag_filters_html: &str,
    cards_html: &str,
    pagination_html: &str,
) -> String {
    render_template(
        template,
        &[
            ("blog_heading", heading_html),
            ("blog_intro", intro_html),
            ("blog_tag_filters", tag_filters_html),
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
        let date_html = post
            .published
            .as_ref()
            .map(|d| format!(
                "<p class=\"card-date\"><time datetime=\"{}\">{}</time></p>",
                escape_html(d),
                escape_html(&format_date(d))
            ))
            .unwrap_or_default();
        let tags_html = render_tag_links(&post.tags, base_url, "card-tags");

        cards.push_str(&format!(
            "<article class=\"post-card\"><h2 class=\"card-title\"><a href=\"{}\">{}</a></h2>{}{}{}<p class=\"card-excerpt\">{}</p></article>",
            post_href,
            escape_html(&post.title),
            date_html,
            subtitle,
            tags_html,
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

fn blog_tag_page_href(tag_slug: &str, page_num: usize, base_url: &str) -> String {
    let path = if page_num == 1 {
        format!("/blog/tags/{}/index.html", tag_slug)
    } else {
        format!("/blog/tags/{}/page/{}/index.html", tag_slug, page_num)
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

fn render_tag_breadcrumbs(tag_label: &str, page_num: usize, base_url: &str) -> String {
    let home_url = with_base_url(base_url, "/");
    let blog_url = blog_page_href(1, base_url);
    let tag_label = escape_html(tag_label);

    if page_num == 1 {
        return format!(
            "<div class=\"breadcrumbs\"><a href=\"{}\">Home</a> <a href=\"{}\">Blog</a> <span>Tag: {}</span></div>",
            home_url, blog_url, tag_label
        );
    }

    format!(
        "<div class=\"breadcrumbs\"><a href=\"{}\">Home</a> <a href=\"{}\">Blog</a> <span>Tag: {}</span> <span>Page {}</span></div>",
        home_url, blog_url, tag_label, page_num
    )
}

fn render_tags_index_breadcrumbs(base_url: &str) -> String {
    let home_url = with_base_url(base_url, "/");
    let blog_url = blog_page_href(1, base_url);

    format!(
        "<div class=\"breadcrumbs\"><a href=\"{}\">Home</a> <a href=\"{}\">Blog</a> <span>Tags</span></div>",
        home_url, blog_url
    )
}

fn render_tags_directory(tags: &[(String, String, usize)], base_url: &str) -> String {
    if tags.is_empty() {
        return "<section class=\"blog-list\"><p>No tags yet.</p></section>".to_string();
    }

    let mut output = String::from("<section class=\"blog-list\"><article class=\"post-card\"><h2 class=\"card-title\">Browse by tag</h2><div class=\"card-tags\">");

    for (slug, label, count) in tags {
        let href = blog_tag_page_href(slug, 1, base_url);
        output.push_str(&format!(
            r#"<a class="tag-pill" href="{}">{} ({})</a>"#,
            href,
            escape_html(label),
            count
        ));
    }

    output.push_str("</div></article></section>");
    output
}

fn build_tag_filters(posts: &[BlogPostMeta]) -> Vec<(String, String)> {
    let mut tags = BTreeMap::<String, String>::new();

    for post in posts {
        for tag in &post.tags {
            let slug = slugify_tag(tag);
            if slug.is_empty() {
                continue;
            }

            tags.entry(slug).or_insert_with(|| tag.clone());
        }
    }

    tags.into_iter().collect()
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
    let tag_filters = build_tag_filters(blog_posts);
    let tag_filters_html = render_blog_filter_links(&tag_filters, None, base_url);
    let heading_html = String::new();

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
            &heading_html,
            &intro_html,
            &tag_filters_html,
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

pub(crate) fn generate_blog_tag_archive_pages(
    output_dir: &Path,
    templates: &Templates,
    site_structure: &SiteStructure,
    base_url: &str,
    robots_meta: &str,
    blog_posts: &[BlogPostMeta],
    posts_per_page: usize,
    generated_count: &mut usize,
) -> Result<()> {
    let mut posts_by_tag: BTreeMap<String, Vec<BlogPostMeta>> = BTreeMap::new();
    let mut tag_labels: BTreeMap<String, String> = BTreeMap::new();

    for post in blog_posts {
        for tag in &post.tags {
            let slug = slugify_tag(tag);
            if slug.is_empty() {
                continue;
            }

            posts_by_tag.entry(slug.clone()).or_default().push(post.clone());
            tag_labels.entry(slug).or_insert_with(|| tag.clone());
        }
    }

    let mut tags_directory: Vec<(String, String, usize)> = Vec::new();
    for (slug, label) in &tag_labels {
        let count = posts_by_tag.get(slug).map(|posts| posts.len()).unwrap_or(0);
        tags_directory.push((slug.clone(), label.clone(), count));
    }

    let tags_page_title = "Blog - Tags".to_string();
    let tags_heading_html = "<h1>All tags</h1>".to_string();
    let tags_intro_html = "<p>Browse all tags used in the blog.</p>".to_string();
    let tags_cards_html = render_tags_directory(&tags_directory, base_url);
    let tags_page_content = render_blog_index_content(
        &templates.blog_index,
        &tags_heading_html,
        &tags_intro_html,
        "",
        &tags_cards_html,
        "",
    );
    let blog_section = Some("Blog".to_string());
    let tags_nav_html = crate::render::render_nav(site_structure, &blog_section, base_url);
    let tags_breadcrumbs_html = render_tags_index_breadcrumbs(base_url);
    let tags_page = render_page(
        &templates.base,
        &tags_page_title,
        &tags_page_content,
        &tags_nav_html,
        &tags_breadcrumbs_html,
        robots_meta,
    );
    let tags_target = output_dir.join("blog/tags/index.html");

    if let Some(parent) = tags_target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory: {}", parent.display()))?;
    }

    fs::write(&tags_target, tags_page)
        .with_context(|| format!("failed to write output file: {}", tags_target.display()))?;

    *generated_count += 1;
    println!("generated {}", tags_target.display());

    for (tag_slug, tagged_posts) in posts_by_tag.iter_mut() {
        tagged_posts.sort_by(|a, b| b.rel_path.cmp(&a.rel_path));

        let total_posts = tagged_posts.len();
        let total_pages = if total_posts == 0 {
            1
        } else {
            (total_posts + posts_per_page - 1) / posts_per_page
        };

        let Some(tag_label) = tag_labels.get(tag_slug) else {
            continue;
        };

        let tag_filters = build_tag_filters(blog_posts);
        let tag_filters_html = render_blog_filter_links(&tag_filters, Some(tag_slug.as_str()), base_url);
        let intro_html = format!("<p>Posts tagged {}</p>", escape_html(tag_label));
        let heading_html = format!("<h1>Blog Tag: {}</h1>", escape_html(tag_label));
        let blog_section = Some("Blog".to_string());

        for page_num in 1..=total_pages {
            let page_title = if page_num == 1 {
                format!("Blog - Tag: {}", tag_label)
            } else {
                format!("Blog - Tag: {} - Page {}", tag_label, page_num)
            };

            let cards_html = if total_posts == 0 {
                render_blog_index_cards(&[], base_url)
            } else {
                let start = (page_num - 1) * posts_per_page;
                let end = usize::min(start + posts_per_page, total_posts);
                render_blog_index_cards(&tagged_posts[start..end], base_url)
            };

            let pagination_html = render_blog_pagination(page_num, total_pages, base_url);
            let page_content = render_blog_index_content(
                &templates.blog_index,
                &heading_html,
                &intro_html,
                &tag_filters_html,
                &cards_html,
                &pagination_html,
            );
            let nav_html = crate::render::render_nav(site_structure, &blog_section, base_url);
            let breadcrumbs_html = render_tag_breadcrumbs(tag_label, page_num, base_url);
            let page = render_page(
                &templates.base,
                &page_title,
                &page_content,
                &nav_html,
                &breadcrumbs_html,
                robots_meta,
            );

            let target = if page_num == 1 {
                output_dir.join(format!("blog/tags/{}/index.html", tag_slug))
            } else {
                output_dir.join(format!("blog/tags/{}/page/{}/index.html", tag_slug, page_num))
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
    }

    Ok(())
}
