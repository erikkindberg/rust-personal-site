use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct PageMeta {
    pub(crate) rel_path: PathBuf,
    pub(crate) section: Option<String>,
    pub(crate) title: String,
}

#[derive(Clone)]
pub(crate) struct BlogPostMeta {
    pub(crate) rel_path: PathBuf,
    pub(crate) title: String,
    pub(crate) subtitle: Option<String>,
    pub(crate) excerpt: String,
}

#[derive(Clone)]
pub(crate) struct Templates {
    pub(crate) base: String,
    pub(crate) blog_index: String,
    pub(crate) blog_post: String,
}

pub(crate) type SiteStructure = BTreeMap<Option<String>, Vec<PageMeta>>;
