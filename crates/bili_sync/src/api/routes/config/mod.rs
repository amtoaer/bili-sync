use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use axum::extract::Extension;
use axum::routing::get;
use sea_orm::DatabaseConnection;

use crate::api::wrapper::{ApiError, ApiResponse, ValidatedJson};
use crate::config::{Config, VersionedConfig};

pub(super) fn router() -> Router {
    Router::new().route("/config", get(get_config).put(update_config))
}

/// 获取全局配置
pub async fn get_config() -> Result<ApiResponse<Arc<Config>>, ApiError> {
    Ok(ApiResponse::ok(VersionedConfig::get().snapshot()))
}

/// 更新全局配置
pub async fn update_config(
    Extension(db): Extension<DatabaseConnection>,
    ValidatedJson(config): ValidatedJson<Config>,
) -> Result<ApiResponse<Arc<Config>>, ApiError> {
    config.check()?;
    let new_config = VersionedConfig::get().update(config, &db).await?;
    Ok(ApiResponse::ok(new_config))
}
