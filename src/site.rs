use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::blog::{
    extract_excerpt,
    generate_blog_tag_archive_pages,
    generate_blog_listing_pages,
    is_blog_index_page,
    is_blog_post_page,
    render_blog_post_content,
    split_blog_post_markdown,
};
use crate::config::BuildConfig;
use crate::render::{
    extract_title,
    fallback_title,
    get_section,
    markdown_to_html,
    render_breadcrumbs,
    render_nav,
    render_page,
};
use crate::types::{BlogPostMeta, PageMeta, SiteStructure, Templates};

pub(crate) fn build_site(config: &BuildConfig, templates: &Templates) -> Result<()> {
    let content_dir = &config.content_dir;
    let output_dir = &config.output_dir;

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

    let mut site_structure: SiteStructure = SiteStructure::new();
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
            let parsed = split_blog_post_markdown(&markdown);
            blog_posts.push(BlogPostMeta {
                rel_path: relative.clone(),
                title,
                subtitle: parsed.subtitle,
                tags: parsed.tags,
                published: parsed.published,
                edited: parsed.edited,
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
            let parsed = split_blog_post_markdown(markdown);
            let body_html = markdown_to_html(&parsed.body);
            render_blog_post_content(
                &templates.blog_post,
                &title,
                parsed.subtitle.as_deref(),
                &parsed.tags,
                parsed.published.as_deref(),
                parsed.edited.as_deref(),
                &body_html,
            )
        } else {
            markdown_to_html(markdown)
        };

        let nav_html = render_nav(&site_structure, &section, &config.base_url);
        let breadcrumbs_html = render_breadcrumbs(&relative, &section, &config.base_url);
        let page = render_page(
            &templates.base,
            &title,
            &html_content,
            &nav_html,
            &breadcrumbs_html,
            &config.robots_meta,
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
        output_dir,
        templates,
        &site_structure,
        &config.base_url,
        &config.robots_meta,
        &blog_intro_markdown,
        &mut blog_posts,
        config.blog_posts_per_page,
        &mut generated_count,
    )?;

    generate_blog_tag_archive_pages(
        output_dir,
        templates,
        &site_structure,
        &config.base_url,
        &config.robots_meta,
        &blog_posts,
        config.blog_posts_per_page,
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

    let robots_txt = robots_txt_content(config.noindex);
    fs::write(output_dir.join("robots.txt"), robots_txt)
        .with_context(|| "failed to write robots.txt".to_string())?;

    println!(
        "done: generated {} page(s), copied {} asset file(s)",
        generated_count, copied_count
    );
    Ok(())
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

fn robots_txt_content(noindex: bool) -> &'static str {
    if noindex {
        "User-agent: *\nDisallow: /\n"
    } else {
        "User-agent: *\nAllow: /\n"
    }
}
