use std::sync::LazyLock;

use anyhow::Result;
use handlebars::handlebars_helper;

use crate::config::versioned_cache::VersionedCache;
use crate::config::{Config, PathSafeTemplate};

pub static TEMPLATE: LazyLock<VersionedCache<handlebars::Handlebars<'static>>> =
    LazyLock::new(|| VersionedCache::new(create_template).expect("Failed to create handlebars template"));

fn create_template(config: &Config) -> Result<handlebars::Handlebars<'static>> {
    let mut handlebars = handlebars::Handlebars::new();
    handlebars.register_helper("truncate", Box::new(truncate));
    handlebars.path_safe_register("video", config.video_name.to_owned())?;
    Ok(handlebars)
}

handlebars_helper!(truncate: |s: String, len: usize| {
    if s.chars().count() > len {
        s.chars().take(len).collect::<String>()
    } else {
        s.to_string()
    }
});
