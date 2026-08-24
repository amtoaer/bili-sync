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

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DanmakuUpdatePolicy {
    pub enabled: bool,
    pub milestones: Vec<DanmakuUpdateMilestone>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DanmakuUpdateMilestone {
    Once { at_days: u32 },
    Periodic { until_days: u32, interval_hours: u32 },
}

impl DanmakuUpdateMilestone {
    pub fn end_days(self) -> i64 {
        i64::from(match self {
            Self::Once { at_days } => at_days,
            Self::Periodic { until_days, .. } => until_days,
        })
    }

    pub fn start_hours(self) -> i64 {
        match self {
            DanmakuUpdateMilestone::Once { at_days } => at_days as i64 * 24,
            DanmakuUpdateMilestone::Periodic {
                until_days,
                interval_hours,
            } => i64::min(until_days as i64 * 24, interval_hours as i64),
        }
    }
}

impl DanmakuUpdatePolicy {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.enabled {
            return Ok(());
        }
        let mut previous_end_days = 0;
        for milestone in &self.milestones {
            if milestone.end_days() <= previous_end_days {
                return Err("弹幕更新的里程碑天数必须大于 0 且严格递增");
            }
            if matches!(milestone, DanmakuUpdateMilestone::Periodic { interval_hours: 0, .. }) {
                return Err("弹幕更新的刷新间隔必须大于 0");
            }
            previous_end_days = milestone.end_days();
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
