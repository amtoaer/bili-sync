use std::path::PathBuf;

use clap::Parser;
use handlebars::handlebars_helper;
use once_cell::sync::Lazy;

use crate::bilibili::VideoCodecs;
use crate::config::Config;
use crate::config::clap::Args;
use crate::config::item::PathSafeTemplate;

/// 全局的 CONFIG，可以从中读取配置信息
pub static CONFIG: Lazy<Config> = Lazy::new(load_config);

/// 全局的 TEMPLATE，用来渲染 video_name 和 page_name 模板
pub static TEMPLATE: Lazy<handlebars::Handlebars> = Lazy::new(|| {
    let mut handlebars = handlebars::Handlebars::new();
    handlebars_helper!(truncate: |s: String, len: usize| {
        if s.chars().count() > len {
            s.chars().take(len).collect::<String>()
        } else {
            s.to_string()
        }
    });
    handlebars.register_helper("truncate", Box::new(truncate));
    handlebars
        .path_safe_register("video", &CONFIG.video_name)
        .expect("failed to register video template");
    handlebars
        .path_safe_register("page", &CONFIG.page_name)
        .expect("failed to register page template");
    handlebars
});

/// 全局的 ARGS，用来解析命令行参数
pub static ARGS: Lazy<Args> = Lazy::new(Args::parse);

/// 全局的 CONFIG_DIR，表示配置文件夹的路径
pub static CONFIG_DIR: Lazy<PathBuf> =
    Lazy::new(|| dirs::config_dir().expect("No config path found").join("bili-sync"));

fn load_config() -> Config {
    info!("开始加载配置文件..");
    let mut config = Config::load().unwrap_or_else(|err| {
        if err
            .downcast_ref::<std::io::Error>()
            .is_none_or(|e| e.kind() != std::io::ErrorKind::NotFound)
        {
            panic!("加载配置文件失败，错误为： {err}");
        }
        warn!("配置文件不存在，使用默认配置..");
        Config::default()
    });
    info!("配置文件加载完毕，覆盖刷新原有配置");
    config.save().expect("保存默认配置时遇到错误");
    info!("检查配置文件..");
    config.check();
    info!("配置文件检查通过");
    if cfg!(test) {
        config.cdn_sorting = true;
        config.filter_option.codecs = vec![VideoCodecs::HEV, VideoCodecs::AVC, VideoCodecs::AV1];
    }
    config
}
