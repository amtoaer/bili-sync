use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwapOption;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

mod clap;
mod global;
mod item;

use crate::adapter::Args;
use crate::bilibili::{CollectionItem, Credential, DanmakuOption, FilterOption};
pub use crate::config::clap::version;
pub use crate::config::global::{ARGS, CONFIG, CONFIG_DIR, TEMPLATE};
use crate::config::item::{ConcurrentLimit, deserialize_collection_list, serialize_collection_list};
pub use crate::config::item::{NFOTimeType, PathSafeTemplate, RateLimit, WatchLaterConfig};

fn default_time_format() -> String {
    "%Y-%m-%d".to_string()
}

/// 默认的 auth_token 实现，生成随机 16 位字符串
fn default_auth_token() -> Option<String> {
    let byte_choices = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()_+-=";
    let mut rng = rand::thread_rng();
    Some(
        (0..16)
            .map(|_| *(byte_choices.choose(&mut rng).expect("choose byte failed")) as char)
            .collect(),
    )
}

fn default_bind_address() -> String {
    "0.0.0.0:12345".to_string()
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_auth_token")]
    pub auth_token: Option<String>,
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    pub credential: ArcSwapOption<Credential>,
    pub filter_option: FilterOption,
    #[serde(default)]
    pub danmaku_option: DanmakuOption,
    pub favorite_list: HashMap<String, PathBuf>,
    #[serde(
        default,
        serialize_with = "serialize_collection_list",
        deserialize_with = "deserialize_collection_list"
    )]
    pub collection_list: HashMap<CollectionItem, PathBuf>,
    #[serde(default)]
    pub submission_list: HashMap<String, PathBuf>,
    #[serde(default)]
    pub watch_later: WatchLaterConfig,
    pub video_name: Cow<'static, str>,
    pub page_name: Cow<'static, str>,
    pub interval: u64,
    pub upper_path: PathBuf,
    #[serde(default)]
    pub nfo_time_type: NFOTimeType,
    #[serde(default)]
    pub concurrent_limit: ConcurrentLimit,
    #[serde(default = "default_time_format")]
    pub time_format: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auth_token: default_auth_token(),
            bind_address: default_bind_address(),
            credential: ArcSwapOption::from(Some(Arc::new(Credential::default()))),
            filter_option: FilterOption::default(),
            danmaku_option: DanmakuOption::default(),
            favorite_list: HashMap::new(),
            collection_list: HashMap::new(),
            submission_list: HashMap::new(),
            watch_later: Default::default(),
            video_name: Cow::Borrowed("{{title}}"),
            page_name: Cow::Borrowed("{{bvid}}"),
            interval: 1200,
            upper_path: CONFIG_DIR.join("upper_face"),
            nfo_time_type: NFOTimeType::FavTime,
            concurrent_limit: ConcurrentLimit::default(),
            time_format: default_time_format(),
        }
    }
}

impl Config {
    pub fn save(&self) -> Result<()> {
        let config_path = CONFIG_DIR.join("config.toml");
        std::fs::create_dir_all(&*CONFIG_DIR)?;
        std::fs::write(config_path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    #[cfg(not(test))]
    fn load() -> Result<Self> {
        let config_path = CONFIG_DIR.join("config.toml");
        let config_content = std::fs::read_to_string(config_path)?;
        Ok(toml::from_str(&config_content)?)
    }

    pub fn as_video_sources(&self) -> Vec<(Args<'_>, &PathBuf)> {
        let mut params = Vec::new();
        self.favorite_list
            .iter()
            .for_each(|(fid, path)| params.push((Args::Favorite { fid }, path)));
        self.collection_list
            .iter()
            .for_each(|(collection_item, path)| params.push((Args::Collection { collection_item }, path)));
        if self.watch_later.enabled {
            params.push((Args::WatchLater, &self.watch_later.path));
        }
        self.submission_list
            .iter()
            .for_each(|(upper_id, path)| params.push((Args::Submission { upper_id }, path)));
        params
    }

    #[cfg(not(test))]
    pub fn check(&self) {
        let mut ok = true;
        let video_sources = self.as_video_sources();
        if video_sources.is_empty() {
            ok = false;
            error!("没有配置任何需要扫描的内容，程序空转没有意义");
        }
        for (args, path) in video_sources {
            if !path.is_absolute() {
                ok = false;
                error!("{:?} 保存的路径应为绝对路径，检测到: {}", args, path.display());
            }
        }
        if !self.upper_path.is_absolute() {
            ok = false;
            error!("up 主头像保存的路径应为绝对路径");
        }
        if self.video_name.is_empty() {
            ok = false;
            error!("未设置 video_name 模板");
        }
        if self.page_name.is_empty() {
            ok = false;
            error!("未设置 page_name 模板");
        }
        let credential = self.credential.load();
        match credential.as_deref() {
            Some(credential) => {
                if credential.sessdata.is_empty()
                    || credential.bili_jct.is_empty()
                    || credential.buvid3.is_empty()
                    || credential.dedeuserid.is_empty()
                    || credential.ac_time_value.is_empty()
                {
                    ok = false;
                    error!("Credential 信息不完整，请确保填写完整");
                }
            }
            None => {
                ok = false;
                error!("未设置 Credential 信息");
            }
        }
        if !(self.concurrent_limit.video > 0 && self.concurrent_limit.page > 0) {
            ok = false;
            error!("video 和 page 允许的并发数必须大于 0");
        }
        if !ok {
            panic!(
                "位于 {} 的配置文件不合法，请参考提示信息修复后继续运行",
                CONFIG_DIR.join("config.toml").display()
            );
        }
    }
}
