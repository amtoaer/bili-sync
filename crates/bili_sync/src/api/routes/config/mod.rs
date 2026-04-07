use std::sync::Arc;

use anyhow::Result;
use axum::extract::Extension;
use axum::routing::{get, post};
use axum::{Json, Router};
use sea_orm::DatabaseConnection;

use crate::api::wrapper::{ApiError, ApiResponse, ValidatedJson};
use crate::bilibili::BiliClient;
use crate::config::{Config, VersionedConfig};
use crate::notifier::{Message, Notifier};

pub(super) fn router() -> Router {
    Router::new()
        .route("/config", get(get_config).put(update_config))
        .route("/config/notifiers/ping", post(ping_notifiers))
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

pub async fn ping_notifiers(
    Extension(bili_client): Extension<Arc<BiliClient>>,
    Json(mut notifier): Json<Notifier>,
) -> Result<ApiResponse<()>, ApiError> {
    // 对于 webhook 类型的通知器测试，设置上 ignore_cache tag 以强制实时渲染
    if let Notifier::Webhook { ignore_cache, .. } = &mut notifier {
        *ignore_cache = Some(());
    }
    notifier
        .notify(bili_client.inner_client(), Message{
            message: "This is a test notification from BiliSync.".into(),
            image_url: Some("https://socialify.git.ci/amtoaer/bili-sync/image?description=1&font=KoHo&issues=1&language=1&logo=https%3A%2F%2Fs2.loli.net%2F2023%2F12%2F02%2F9EwT2yInOu1d3zm.png&name=1&owner=1&pattern=Signal&pulls=1&stargazers=1&theme=Light".to_owned()),
        })
        .await?;
    Ok(ApiResponse::ok(()))
}
