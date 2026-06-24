use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::utils::filenamify::filenamify;

/// NFO 文件使用的时间类型
#[derive(Serialize, Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum NFOTimeType {
    #[default]
    FavTime,
    PubTime,
}

/// 并发下载相关的配置
#[derive(Serialize, Deserialize, Clone)]
pub struct ConcurrentLimit {
    pub video: usize,
    pub page: usize,
    pub rate_limit: Option<RateLimit>,
    #[serde(default)]
    pub download: ConcurrentDownloadLimit,
}

#[derive(Serialize, Deserialize, Clone)]
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

#[derive(Serialize, Deserialize, Clone)]
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

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SkipOption {
    pub no_poster: bool,
    pub no_video_nfo: bool,
    pub no_upper: bool,
    pub no_danmaku: bool,
    pub no_subtitle: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Trigger {
    Interval(u64),
    Cron(String),
}

impl Default for Trigger {
    fn default() -> Self {
        Trigger::Interval(1200)
    }
}

/// 弹幕增量更新策略。
///
/// 采用三段式模型，符合弹幕密度随发布时间衰减的真实曲线：
/// - 新鲜期：发布后 `fresh_days` 天内，每 `fresh_interval_hours` 小时刷新一次。
/// - 成熟期：新鲜期结束到 `mature_days` 天之间，每 `mature_interval_days` 天刷新一次。
/// - 老化期：成熟期结束到 `cold_days` 天之间，每 `cold_interval_days` 天刷新一次。
/// - 冷冻：超过 `cold_days` 后触发最后一次更新并冻结，之后不再自动刷新（手动触发仍可）。
///
/// 默认关闭，保持向后兼容；启用后首次下载成功即视为第一次同步。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DanmakuUpdatePolicy {
    pub enabled: bool,
    pub fresh_days: u32,
    pub fresh_interval_hours: u32,
    pub mature_days: u32,
    pub mature_interval_days: u32,
    pub cold_days: u32,
    pub cold_interval_days: u32,
}

impl Default for DanmakuUpdatePolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            fresh_days: 3,
            fresh_interval_hours: 6,
            mature_days: 30,
            mature_interval_days: 3,
            cold_days: 180,
            cold_interval_days: 30,
        }
    }
}

impl DanmakuUpdatePolicy {
    /// 校验字段合法性：三段阈值需单调递增，时间间隔必须大于 0。
    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.enabled {
            return Ok(());
        }
        if self.fresh_days > self.mature_days {
            return Err("fresh_days 不能大于 mature_days");
        }
        if self.mature_days > self.cold_days {
            return Err("mature_days 不能大于 cold_days");
        }
        if self.fresh_interval_hours == 0 {
            return Err("fresh_interval_hours 必须大于 0");
        }
        if self.mature_interval_days == 0 {
            return Err("mature_interval_days 必须大于 0");
        }
        if self.cold_interval_days == 0 {
            return Err("cold_interval_days 必须大于 0");
        }
        Ok(())
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
