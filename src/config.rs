use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use log::warn;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::bilibili::{Credential, FilterOption};
use crate::Result;

pub static CONFIG: Lazy<Mutex<Config>> = Lazy::new(|| {
    let config = Config::new();
    // 保存一次，确保配置文件存在
    config.save().unwrap();
    // 检查配置文件内容
    config.check();
    Mutex::new(Config::new())
});

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub credential: Option<Credential>,
    pub filter_option: FilterOption,
    pub favorite_list: HashMap<String, String>,
    pub video_name: String,
    pub page_name: String,
    pub interval: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            credential: Some(Credential::default()),
            filter_option: FilterOption::default(),
            favorite_list: HashMap::new(),
            video_name: "{{bvid}}".to_string(),
            page_name: "{{bvid}}".to_string(),
            interval: 1200,
        }
    }
}
impl Config {
    fn new() -> Self {
        Config::load().unwrap_or_default()
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
        assert!(!self.video_name.is_empty(), "No video name template found");
        assert!(!self.page_name.is_empty(), "No page name template found");
        match self.credential {
            Some(ref credential) => {
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
            .ok_or("No config path found")?
            .join("bili-sync")
            .join("config.toml");
        let config_content = std::fs::read_to_string(config_path)?;
        Ok(toml::from_str(&config_content)?)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = dirs::config_dir()
            .ok_or("No config path found")?
            .join("bili-sync")
            .join("config.toml");
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(config_path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}
