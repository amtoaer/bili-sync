use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use arc_swap::ArcSwapOption;
use log::warn;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::bilibili::{Credential, FilterOption};

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let config = Config::load().unwrap_or_else(|err| {
        warn!("Failed loading config: {err}");
        let new_config = Config::new();
        // 保存一次，确保配置文件存在
        new_config.save().unwrap();
        new_config
    });
    // 检查配置文件内容
    config.check();
    config
});

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub credential: ArcSwapOption<Credential>,
    pub filter_option: FilterOption,
    pub favorite_list: HashMap<String, String>,
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
            favorite_list: HashMap::new(),
            video_name: Cow::Borrowed("{{bvid}}"),
            page_name: Cow::Borrowed("{{bvid}}"),
            interval: 1200,
            upper_path: dirs::config_dir().unwrap().join("bili-sync/upper_face"),
        }
    }

    /// 简单的预检查
    pub fn check(&self) {
        assert!(
            !self.favorite_list.is_empty(),
            "No favorite list found, program won't do anything"
        );
        for path in self.favorite_list.values() {
            assert!(Path::new(path).is_absolute(), "Path in favorite list must be absolute");
        }
        assert!(self.upper_path.is_absolute(), "Upper face path must be absolute");
        assert!(!self.video_name.is_empty(), "No video name template found");
        assert!(!self.page_name.is_empty(), "No page name template found");
        let credential = self.credential.load();
        match &*credential {
            Some(credential) => {
                assert!(
                    !(credential.sessdata.is_empty()
                        || credential.bili_jct.is_empty()
                        || credential.buvid3.is_empty()
                        || credential.dedeuserid.is_empty()
                        || credential.ac_time_value.is_empty()),
                    "Credential is incomplete"
                )
            }
            None => {
                warn!("No credential found, can't access high quality video");
            }
        }
    }

    fn load() -> Result<Self> {
        let config_path = dirs::config_dir()
            .ok_or_else(|| anyhow!("No config path found"))?
            .join("bili-sync/config.toml");
        let config_content = std::fs::read_to_string(config_path)?;
        Ok(toml::from_str(&config_content)?)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = dirs::config_dir()
            .ok_or_else(|| anyhow!("No config path found"))?
            .join("bili-sync/config.toml");
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(config_path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}
