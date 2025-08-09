use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use axum::extract::Extension;
use axum::routing::get;
use sea_orm::DatabaseConnection;

use crate::api::error::InnerApiError;
use crate::api::wrapper::{ApiError, ApiResponse, ValidatedJson};
use crate::config::{Config, VersionedConfig};
use crate::utils::task_notifier::TASK_STATUS_NOTIFIER;

pub(super) fn router() -> Router {
    Router::new().route("/config", get(get_config).put(update_config))
}

/// 获取全局配置
pub async fn get_config() -> Result<ApiResponse<Arc<Config>>, ApiError> {
    Ok(ApiResponse::ok(VersionedConfig::get().load_full()))
}

/// 更新全局配置
pub async fn update_config(
    Extension(db): Extension<DatabaseConnection>,
    ValidatedJson(config): ValidatedJson<Config>,
) -> Result<ApiResponse<Arc<Config>>, ApiError> {
    let Some(_lock) = TASK_STATUS_NOTIFIER.detect_running() else {
        // 简单避免一下可能的不一致现象
        return Err(InnerApiError::BadRequest("下载任务正在运行，无法修改配置".to_string()).into());
    };
    config.check()?;
    let new_config = VersionedConfig::get().update(config, &db).await?;
    drop(_lock);
    Ok(ApiResponse::ok(new_config))
}
