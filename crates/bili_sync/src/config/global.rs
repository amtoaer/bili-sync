use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwap;
use arc_swap::access::Access;
use clap::Parser;
use handlebars::handlebars_helper;
use once_cell::sync::Lazy;
use sea_orm::DatabaseConnection;
use tokio::sync::OnceCell;

use crate::bilibili::Credential;
use crate::config::clap::Args;
use crate::config::{Config, PathSafeTemplate};

/// 配置和模板的组合，支持原子性更新
pub struct ConfigTemplate {
    pub config: Config,
    pub template: handlebars::Handlebars<'static>,
}

/// 全局状态结构体，包含所有全局配置信息
pub struct GlobalState {
    pub args: Args,
    pub config_template: ArcSwap<ConfigTemplate>,
}

/// 全局状态实例
pub static GLOBAL_STATE: OnceCell<GlobalState> = OnceCell::const_new();

/// 全局的 CONFIG_DIR，表示配置文件夹的路径
pub static CONFIG_DIR: Lazy<PathBuf> =
    Lazy::new(|| dirs::config_dir().expect("No config path found").join("bili-sync"));

impl GlobalState {
    pub async fn init(_database_connection: Arc<DatabaseConnection>) -> Result<()> {
        Ok(GLOBAL_STATE
            .set(GlobalState {
                args: Args::parse(),
                config_template: ArcSwap::from_pointee(ConfigTemplate::new()?),
            })
            .map_err(|_| anyhow::anyhow!("Global state already initialized"))?)
    }

    pub fn get() -> &'static GlobalState {
        GLOBAL_STATE.get().expect("Global state not initialized")
    }

    // 对于 borrowed 的数据结构，.load 方法返回的 Guard 无法跨 await，在需要跨 await 的场景下使用 owned 方法
    pub fn config_borrowed(&self) -> impl Access<Config> + Send + Sync {
        self.config_template
            .map(|config_template: &ConfigTemplate| &config_template.config)
    }

    pub fn template_borrowed(&self) -> impl Access<handlebars::Handlebars<'static>> + Send + Sync {
        self.config_template
            .map(|config_template: &ConfigTemplate| &config_template.template)
    }

    pub fn config_template_borrowed(&self) -> impl Access<ConfigTemplate> + Send + Sync {
        &self.config_template
    }

    pub fn config_template_owned(&self) -> Arc<ConfigTemplate> {
        self.config_template.load_full()
    }
}

impl ConfigTemplate {
    pub fn new() -> Result<Self> {
        let config = Self::load_config()?;
        let template = Self::create_template(&config)?;
        Ok(ConfigTemplate { config, template })
    }

    fn create_template(config: &Config) -> Result<handlebars::Handlebars<'static>> {
        let mut handlebars = handlebars::Handlebars::new();
        handlebars_helper!(truncate: |s: String, len: usize| {
            if s.chars().count() > len {
                s.chars().take(len).collect::<String>()
            } else {
                s.to_string()
            }
        });
        handlebars.register_helper("truncate", Box::new(truncate));
        handlebars.path_safe_register("video", config.video_name.to_owned())?;
        Ok(handlebars)
    }

    fn load_config() -> Result<Config> {
        if cfg!(test) {
            return Ok(Config::load(&CONFIG_DIR.join("test_config.toml")).unwrap_or(Config::test_default()));
        }
        info!("开始加载配置文件..");
        let config = Config::load(&CONFIG_DIR.join("config.toml")).unwrap_or_else(|err| {
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
        Ok(config)
    }
}

pub fn args() -> &'static Args {
    &GlobalState::get().args
}

pub fn config_borrowed() -> impl Access<Config> + Send + Sync {
    GlobalState::get().config_borrowed()
}

pub fn template_borrowed() -> impl Access<handlebars::Handlebars<'static>> + Send + Sync {
    GlobalState::get().template_borrowed()
}

pub fn config_template_borrowed() -> impl Access<ConfigTemplate> + Send + Sync {
    GlobalState::get().config_template_borrowed()
}

pub fn config_template_owned() -> Arc<ConfigTemplate> {
    GlobalState::get().config_template_owned()
}

pub fn credential() -> arc_swap::Guard<Option<Arc<Credential>>> {
    GlobalState::get().config_borrowed().load().credential.load()
}

pub fn set_credential(credential: Option<Arc<Credential>>) {
    GlobalState::get().config_borrowed().load().credential.store(credential);
}
