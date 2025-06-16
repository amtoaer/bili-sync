use std::path::PathBuf;
use std::sync::LazyLock;

use anyhow::{Result, bail};
use arc_swap::ArcSwap;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

use crate::bilibili::{Credential, DanmakuOption, FilterOption};
use crate::config::LegacyConfig;
use crate::config::default::{default_auth_token, default_bind_address, default_time_format};
use crate::config::item::{ConcurrentLimit, NFOTimeType};
use crate::utils::model::{load_db_config, save_db_config};

pub static CONFIG_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| dirs::config_dir().expect("No config path found").join("bili-sync"));

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_auth_token")]
    pub auth_token: String,
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    pub credential: ArcSwap<Credential>,
    pub filter_option: FilterOption,
    #[serde(default)]
    pub danmaku_option: DanmakuOption,
    pub video_name: String,
    pub page_name: String,
    pub interval: u64,
    pub upper_path: PathBuf,
    #[serde(default)]
    pub nfo_time_type: NFOTimeType,
    #[serde(default)]
    pub concurrent_limit: ConcurrentLimit,
    #[serde(default = "default_time_format")]
    pub time_format: String,
    #[serde(default)]
    pub cdn_sorting: bool,
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
        let credential = self.credential.load();
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
            credential: ArcSwap::from_pointee(Credential::default()),
            filter_option: FilterOption::default(),
            danmaku_option: DanmakuOption::default(),
            video_name: "{{title}}".to_owned(),
            page_name: "{{bvid}}".to_owned(),
            interval: 1200,
            upper_path: CONFIG_DIR.join("upper_face"),
            nfo_time_type: NFOTimeType::FavTime,
            concurrent_limit: ConcurrentLimit::default(),
            time_format: default_time_format(),
            cdn_sorting: false,
        }
    }
}

impl From<LegacyConfig> for Config {
    fn from(legacy: LegacyConfig) -> Self {
        Self {
            auth_token: legacy.auth_token,
            bind_address: legacy.bind_address,
            credential: legacy.credential,
            filter_option: legacy.filter_option,
            danmaku_option: legacy.danmaku_option,
            video_name: legacy.video_name,
            page_name: legacy.page_name,
            interval: legacy.interval,
            upper_path: legacy.upper_path,
            nfo_time_type: legacy.nfo_time_type,
            concurrent_limit: legacy.concurrent_limit,
            time_format: legacy.time_format,
            cdn_sorting: legacy.cdn_sorting,
        }
    }
}
