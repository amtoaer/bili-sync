use std::path::PathBuf;
use std::sync::LazyLock;

use anyhow::{Result, bail};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::bilibili::{Credential, DanmakuOption, FilterOption};
use crate::config::default::{default_auth_token, default_bind_address, default_time_format};
use crate::config::item::{ConcurrentLimit, NFOTimeType, SkipOption};
use crate::utils::model::{load_db_config, save_db_config};

pub static CONFIG_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| dirs::config_dir().expect("No config path found").join("bili-sync"));

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
    pub interval: u64,
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

    #[cfg(test)]
    pub(super) fn test_default() -> Self {
        Self {
            cdn_sorting: true,
            ..Default::default()
        }
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
            interval: 1200,
            upper_path: CONFIG_DIR.join("upper_face"),
            nfo_time_type: NFOTimeType::FavTime,
            concurrent_limit: ConcurrentLimit::default(),
            time_format: default_time_format(),
            cdn_sorting: false,
            version: 0,
        }
    }
}
