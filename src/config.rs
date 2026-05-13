use std::path::PathBuf;

use crate::render::normalize_base_url;

const BLOG_POSTS_PER_PAGE_DEFAULT: usize = 5;

#[derive(Clone)]
pub(crate) struct BuildConfig {
    pub(crate) content_dir: PathBuf,
    pub(crate) output_dir: PathBuf,
    pub(crate) template_dir: PathBuf,
    pub(crate) base_url: String,
    pub(crate) noindex: bool,
    pub(crate) robots_meta: String,
    pub(crate) blog_posts_per_page: usize,
}

pub(crate) fn load_config() -> BuildConfig {
    let content_dir = PathBuf::from("content");
    let output_dir = PathBuf::from("public");
    let template_dir = PathBuf::from("templates");
    let base_url = base_url_from_env();
    let noindex = noindex_from_env();
    let robots_meta = robots_meta_tag(noindex);
    let blog_posts_per_page = blog_posts_per_page_from_env();

    BuildConfig {
        content_dir,
        output_dir,
        template_dir,
        base_url,
        noindex,
        robots_meta,
        blog_posts_per_page,
    }
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

fn blog_posts_per_page_from_env() -> usize {
    std::env::var("BLOG_POSTS_PER_PAGE")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(BLOG_POSTS_PER_PAGE_DEFAULT)
}
