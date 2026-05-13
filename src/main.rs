mod blog;
mod config;
mod render;
mod site;
mod template_loader;
mod types;

use anyhow::Result;

fn main() -> Result<()> {
    let config = config::load_config();
    let templates = template_loader::load_templates(&config.template_dir)?;
    site::build_site(&config, &templates)
}
