use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use arc_swap::ArcSwapOption;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::bilibili::{Credential, DanmakuOption, FilterOption};

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let config = Config::load().unwrap_or_else(|err| {
        warn!("Failed loading config: {err}");
        Config::new()
    });
    // 放到外面，确保新的配置项被保存
    config.save().unwrap();
    // 检查配置文件内容
    config.check();
    config
});

pub static CONFIG_DIR: Lazy<PathBuf> =
    Lazy::new(|| dirs::config_dir().expect("No config path found").join("bili-sync"));

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub credential: ArcSwapOption<Credential>,
    pub filter_option: FilterOption,
    #[serde(default)]
    pub danmaku_option: DanmakuOption,
    pub favorite_list: HashMap<String, PathBuf>,
    pub video_name: Cow<'static, str>,
    pub page_name: Cow<'static, str>,
    pub interval: u64,
    pub upper_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    fn new() -> Self {
        Self {
            credential: ArcSwapOption::empty(),
            filter_option: FilterOption::default(),
            danmaku_option: DanmakuOption::default(),
            favorite_list: HashMap::new(),
            video_name: Cow::Borrowed("{{bvid}}"),
            page_name: Cow::Borrowed("{{bvid}}"),
            interval: 1200,
            upper_path: CONFIG_DIR.join("upper_face"),
        }
    }

    /// 简单的预检查
    pub fn check(&self) {
        let mut ok = true;
        if self.favorite_list.is_empty() {
            ok = false;
            error!("No favorite list found, program won't do anything");
        }
        for path in self.favorite_list.values() {
            if !path.is_absolute() {
                ok = false;
                error!("Path in favorite list must be absolute: {}", path.display());
            }
        }
        if !self.upper_path.is_absolute() {
            ok = false;
            error!("Upper face path must be absolute");
        }
        if self.video_name.is_empty() {
            ok = false;
            error!("No video name template found");
        }
        if self.page_name.is_empty() {
            ok = false;
            error!("No page name template found");
        }
        let credential = self.credential.load();
        if let Some(credential) = credential.as_deref() {
            if credential.sessdata.is_empty()
                || credential.bili_jct.is_empty()
                || credential.buvid3.is_empty()
                || credential.dedeuserid.is_empty()
                || credential.ac_time_value.is_empty()
            {
                ok = false;
                error!("Credential is incomplete");
            }
        } else {
            warn!("No credential found, can't access high quality video");
        }

        if !ok {
            panic!("Config in {} is invalid", CONFIG_DIR.join("config.toml").display());
        }
    }

    fn load() -> Result<Self> {
        let config_path = CONFIG_DIR.join("config.toml");
        let config_content = std::fs::read_to_string(config_path)?;
        Ok(toml::from_str(&config_content)?)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = CONFIG_DIR.join("config.toml");
        std::fs::create_dir_all(&*CONFIG_DIR)?;
        std::fs::write(config_path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}
