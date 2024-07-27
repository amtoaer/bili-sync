use std::path::PathBuf;

use clap::Parser;
use handlebars::handlebars_helper;
use once_cell::sync::Lazy;

use crate::config::clap::Args;
use crate::config::Config;

/// 全局的 CONFIG，可以从中读取配置信息
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let config = Config::load().unwrap_or_else(|err| {
        if err
            .downcast_ref::<std::io::Error>()
            .map_or(true, |e| e.kind() != std::io::ErrorKind::NotFound)
        {
            panic!("加载配置文件失败，错误为： {err}");
        }
        warn!("配置文件不存在，使用默认配置...");
        Config::default()
    });
    // 放到外面，确保新的配置项被保存
    info!("配置加载完毕，覆盖刷新原有配置");
    config.save().unwrap();
    // 检查配置文件内容
    info!("校验配置文件内容...");
    config.check();
    config
});

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
        .register_template_string("video", &CONFIG.video_name)
        .unwrap();
    handlebars.register_template_string("page", &CONFIG.page_name).unwrap();
    handlebars
});

/// 全局的 ARGS，用来解析命令行参数
pub static ARGS: Lazy<Args> = Lazy::new(Args::parse);

/// 全局的 CONFIG_DIR，表示配置文件夹的路径
pub static CONFIG_DIR: Lazy<PathBuf> =
    Lazy::new(|| dirs::config_dir().expect("No config path found").join("bili-sync"));
