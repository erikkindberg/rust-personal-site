use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::types::Templates;

pub(crate) fn load_templates(template_dir: &Path) -> Result<Templates> {
    Ok(Templates {
        base: load_template_file(&template_dir.join("page.html"))?,
        blog_index: load_template_file(&template_dir.join("blog_index.html"))?,
        blog_post: load_template_file(&template_dir.join("blog_post.html"))?,
    })
}

fn load_template_file(template_path: &Path) -> Result<String> {
    fs::read_to_string(template_path)
        .with_context(|| format!("failed to read required template: {}", template_path.display()))
}
