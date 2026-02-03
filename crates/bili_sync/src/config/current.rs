use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use anyhow::{Result, bail};
use croner::parser::CronParser;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::bilibili::{Credential, DanmakuOption, FilterOption};
use crate::config::args::ARGS;
use crate::config::default::{
    default_auth_token, default_bind_address, default_collection_path, default_favorite_path, default_submission_path,
    default_time_format,
};
use crate::config::item::{ConcurrentLimit, NFOTimeType, SkipOption, Trigger};
use crate::notifier::Notifier;
use crate::utils::model::{load_db_config, save_db_config};

pub static CONFIG_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    ARGS.config_dir
        .clone()
        .or_else(|| dirs::config_dir().map(|dir| dir.join("bili-sync")))
        .expect("No config path found")
});

#[derive(Serialize, Deserialize, Validate, Clone)]
pub struct Config {
    pub auth_token: String,
    pub bind_address: String,
    pub credential: Credential,
    pub filter_option: FilterOption,
    pub danmaku_option: DanmakuOption,
    #[serde(default)]
    pub skip_option: SkipOption,
    pub video_name: String,
    pub page_name: String,
    #[serde(default)]
    pub notifiers: Option<Arc<Vec<Notifier>>>,
    #[serde(default = "default_favorite_path")]
    pub favorite_default_path: String,
    #[serde(default = "default_collection_path")]
    pub collection_default_path: String,
    #[serde(default = "default_submission_path")]
    pub submission_default_path: String,
    pub interval: Trigger,
    pub upper_path: PathBuf,
    pub nfo_time_type: NFOTimeType,
    pub concurrent_limit: ConcurrentLimit,
    pub time_format: String,
    pub cdn_sorting: bool,
    pub version: u64,
}

impl Config {
    pub async fn load_from_database(connection: &DatabaseConnection) -> Result<Option<Result<Self>>> {
        load_db_config(connection).await
    }

    pub async fn save_to_database(&self, connection: &DatabaseConnection) -> Result<()> {
        save_db_config(self, connection).await
    }

    pub fn check(&self) -> Result<()> {
        let mut errors = Vec::new();
        if !self.upper_path.is_absolute() {
            errors.push("up 主头像保存的路径应为绝对路径");
        }
        if self.video_name.is_empty() {
            errors.push("未设置 video_name 模板");
        }
        if self.page_name.is_empty() {
            errors.push("未设置 page_name 模板");
        }
        let credential = &self.credential;
        if credential.sessdata.is_empty()
            || credential.bili_jct.is_empty()
            || credential.buvid3.is_empty()
            || credential.dedeuserid.is_empty()
            || credential.ac_time_value.is_empty()
        {
            errors.push("Credential 信息不完整，请确保填写完整");
        }
        if !(self.concurrent_limit.video > 0 && self.concurrent_limit.page > 0) {
            errors.push("video 和 page 允许的并发数必须大于 0");
        }
        match &self.interval {
            Trigger::Interval(secs) => {
                if *secs <= 60 {
                    errors.push("下载任务执行间隔时间必须大于 60 秒");
                }
            }
            Trigger::Cron(cron) => {
                if CronParser::builder()
                    .seconds(croner::parser::Seconds::Required)
                    .dom_and_dow(true)
                    .build()
                    .parse(cron)
                    .is_err()
                {
                    errors.push("Cron 表达式无效，正确格式为“秒 分 时 日 月 周”");
                }
            }
        };
        if !errors.is_empty() {
            bail!(
                errors
                    .into_iter()
                    .map(|e| format!("- {}", e))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auth_token: default_auth_token(),
            bind_address: default_bind_address(),
            credential: Credential::default(),
            filter_option: FilterOption::default(),
            danmaku_option: DanmakuOption::default(),
            skip_option: SkipOption::default(),
            video_name: "{{title}}".to_owned(),
            page_name: "{{bvid}}".to_owned(),
            notifiers: None,
            favorite_default_path: default_favorite_path(),
            collection_default_path: default_collection_path(),
            submission_default_path: default_submission_path(),
            interval: Trigger::default(),
            upper_path: CONFIG_DIR.join("upper_face"),
            nfo_time_type: NFOTimeType::FavTime,
            concurrent_limit: ConcurrentLimit::default(),
            time_format: default_time_format(),
            cdn_sorting: false,
            version: 0,
        }
    }
}
