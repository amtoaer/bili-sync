use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::utils::filenamify::filenamify;

/// 稍后再看的配置
#[derive(Serialize, Deserialize, Default)]
pub struct WatchLaterConfig {
    pub enabled: bool,
    pub path: PathBuf,
}

/// NFO 文件使用的时间类型
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NFOTimeType {
    #[default]
    FavTime,
    PubTime,
}

/// 并发下载相关的配置
#[derive(Serialize, Deserialize)]
pub struct ConcurrentLimit {
    pub video: usize,
    pub page: usize,
    pub rate_limit: Option<RateLimit>,
    #[serde(default)]
    pub download: ConcurrentDownloadLimit,
}

#[derive(Serialize, Deserialize)]
pub struct ConcurrentDownloadLimit {
    pub enable: bool,
    pub concurrency: usize,
    pub threshold: u64,
}

impl Default for ConcurrentDownloadLimit {
    fn default() -> Self {
        Self {
            enable: true,
            concurrency: 4,
            threshold: 20 * (1 << 20), // 20 MB
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct RateLimit {
    pub limit: usize,
    pub duration: u64,
}

impl Default for ConcurrentLimit {
    fn default() -> Self {
        Self {
            video: 3,
            page: 2,
            // 默认的限速配置，每 250ms 允许请求 4 次
            rate_limit: Some(RateLimit {
                limit: 4,
                duration: 250,
            }),
            download: ConcurrentDownloadLimit::default(),
        }
    }
}

pub trait PathSafeTemplate {
    fn path_safe_register(&mut self, name: &'static str, template: impl Into<String>) -> Result<()>;
    fn path_safe_render(&self, name: &'static str, data: &serde_json::Value) -> Result<String>;
}

/// 通过将模板字符串中的分隔符替换为自定义的字符串，使得模板字符串中的分隔符得以保留
impl PathSafeTemplate for handlebars::Handlebars<'_> {
    fn path_safe_register(&mut self, name: &'static str, template: impl Into<String>) -> Result<()> {
        let template = template.into();
        Ok(self.register_template_string(name, template.replace(std::path::MAIN_SEPARATOR_STR, "__SEP__"))?)
    }

    fn path_safe_render(&self, name: &'static str, data: &serde_json::Value) -> Result<String> {
        Ok(filenamify(&self.render(name, data)?).replace("__SEP__", std::path::MAIN_SEPARATOR_STR))
    }
}
