use std::sync::LazyLock;

use anyhow::Result;
use handlebars::handlebars_helper;

use crate::config::versioned_cache::VersionedCache;
use crate::config::{Config, PathSafeTemplate};
use crate::notifier::{Notifier, webhook_template_content, webhook_template_key};

pub static TEMPLATE: LazyLock<VersionedCache<handlebars::Handlebars<'static>>> =
    LazyLock::new(|| VersionedCache::new(create_template).expect("Failed to create handlebars template"));

fn create_template(config: &Config) -> Result<handlebars::Handlebars<'static>> {
    let mut handlebars = handlebars::Handlebars::new();
    handlebars.register_helper("truncate", Box::new(truncate));
    handlebars.path_safe_register("video", config.video_name.clone())?;
    handlebars.path_safe_register("page", config.page_name.clone())?;
    handlebars.path_safe_register("favorite_default_path", config.favorite_default_path.clone())?;
    handlebars.path_safe_register("collection_default_path", config.collection_default_path.clone())?;
    handlebars.path_safe_register("submission_default_path", config.submission_default_path.clone())?;
    if let Some(notifiers) = &config.notifiers {
        for notifier in notifiers.iter() {
            if let Notifier::Webhook { url, template, .. } = notifier {
                handlebars.register_template_string(&webhook_template_key(url), webhook_template_content(template))?;
            }
        }
    }
    Ok(handlebars)
}

handlebars_helper!(truncate: |s: String, len: usize| {
    if s.chars().count() > len {
        s.chars().take(len).collect::<String>()
    } else {
        s.to_string()
    }
});

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_template_usage() {
        let mut template = handlebars::Handlebars::new();
        template.register_helper("truncate", Box::new(truncate));
        let _ = template.path_safe_register("video", "test{{bvid}}test");
        let _ = template.path_safe_register("test_truncate", "哈哈，{{ truncate title 30 }}");
        let _ = template.path_safe_register("test_path_unix", "{{ truncate title 7 }}/test/a");
        let _ = template.path_safe_register("test_path_windows", r"{{ truncate title 7 }}\\test\\a");
        #[cfg(not(windows))]
        {
            assert_eq!(
                template
                    .path_safe_render("test_path_unix", &json!({"title": "关注/永雏塔菲喵"}))
                    .unwrap(),
                "关注_永雏塔菲/test/a"
            );
            assert_eq!(
                template
                    .path_safe_render("test_path_windows", &json!({"title": "关注/永雏塔菲喵"}))
                    .unwrap(),
                "关注_永雏塔菲_test_a"
            );
        }
        #[cfg(windows)]
        {
            assert_eq!(
                template
                    .path_safe_render("test_path_unix", &json!({"title": "关注/永雏塔菲喵"}))
                    .unwrap(),
                "关注_永雏塔菲_test_a"
            );
            assert_eq!(
                template
                    .path_safe_render("test_path_windows", &json!({"title": "关注/永雏塔菲喵"}))
                    .unwrap(),
                r"关注_永雏塔菲\\test\\a"
            );
        }
        assert_eq!(
            template
                .path_safe_render("video", &json!({"bvid": "BV1b5411h7g7"}))
                .unwrap(),
            "testBV1b5411h7g7test"
        );
        assert_eq!(
            template
                .path_safe_render(
                    "test_truncate",
                    &json!({"title": "你说得对，但是 Rust 是由 Mozilla 自主研发的一款全新的编译期格斗游戏。\
                    编译将发生在一个被称作「Cargo」的构建系统中。在这里，被引用的指针将被授予「生命周期」之力，导引对象安全。\
                    你将扮演一位名为「Rustacean」的神秘角色，在与「Rustc」的搏斗中邂逅各种骨骼惊奇的傲娇报错。\
                    征服她们、通过编译同时，逐步发掘「C++」程序崩溃的真相。"})
                )
                .unwrap(),
            "哈哈，你说得对，但是 Rust 是由 Mozilla 自主研发的一"
        );
    }
}
