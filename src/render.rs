use std::path::Path;

use pulldown_cmark::{html, Options, Parser};

use crate::types::SiteStructure;

pub(crate) fn normalize_base_url(raw_base_url: &str) -> String {
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

pub(crate) fn with_base_url(base_url: &str, path: &str) -> String {
    if base_url.is_empty() {
        return path.to_string();
    }

    if path == "/" {
        return format!("{}/", base_url);
    }

    format!("{}{}", base_url, path)
}

pub(crate) fn markdown_to_html(markdown: &str) -> String {
    let options = Options::all();
    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

pub(crate) fn extract_title(markdown: &str) -> Option<String> {
    markdown
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|title| !title.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn fallback_title(relative: &Path) -> String {
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

pub(crate) fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

pub(crate) fn render_page(
    template: &str,
    title: &str,
    content: &str,
    nav: &str,
    breadcrumbs: &str,
    robots_meta: &str,
) -> String {
    render_template(
        template,
        &[
            ("title", &escape_html(title)),
            ("robots_meta", robots_meta),
            ("content", content),
            ("nav", nav),
            ("breadcrumbs", breadcrumbs),
        ],
    )
}

pub(crate) fn get_section(rel_path: &Path) -> Option<String> {
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

pub(crate) fn render_nav(site_structure: &SiteStructure, current_section: &Option<String>, base_url: &str) -> String {
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

pub(crate) fn render_breadcrumbs(rel_path: &Path, current_section: &Option<String>, base_url: &str) -> String {
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

pub(crate) fn render_template(template: &str, placeholders: &[(&str, &str)]) -> String {
    let mut output = template.to_string();
    for (placeholder, value) in placeholders {
        output = output.replace(&format!("{{{{{}}}}}", placeholder), value);
    }
    output
}

pub(crate) fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
