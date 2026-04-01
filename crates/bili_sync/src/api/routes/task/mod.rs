use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use axum::extract::Extension;
use axum::routing::post;
use axum::{Json, Router};
use sea_orm::DatabaseConnection;

use crate::api::error::InnerApiError;
use crate::api::request::ManualDownloadRequest;
use crate::api::wrapper::{ApiError, ApiResponse};
use crate::bilibili::BiliClient;
use crate::config::VersionedConfig;
use crate::task::DownloadTaskManager;
use crate::utils::notify::error_and_notify;

pub(super) fn router() -> Router {
    Router::new()
        .route("/task/download", post(new_download_task))
        .route("/task/manual-download", post(new_manual_download_task))
}

pub async fn new_download_task() -> Result<ApiResponse<bool>, ApiError> {
    DownloadTaskManager::get().download_once().await?;
    Ok(ApiResponse::ok(true))
}

pub async fn new_manual_download_task(
    Extension(db): Extension<DatabaseConnection>,
    Extension(bili_client): Extension<Arc<BiliClient>>,
    Json(request): Json<ManualDownloadRequest>,
) -> Result<ApiResponse<bool>, ApiError> {
    let video_url = request.video_url.trim();
    if video_url.is_empty() {
        return Err(InnerApiError::BadRequest("视频链接或 BV 号不能为空".to_string()).into());
    }
    let bvid = crate::task::resolve_bvid(video_url, bili_client.inner_client())
        .await
        .map_err(|e| InnerApiError::BadRequest(format!("{:#}", e)))?;
    let download_path = request
        .download_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned);
    if let Some(path) = download_path.as_deref()
        && !Path::new(path).is_absolute()
    {
        return Err(InnerApiError::BadRequest("下载路径必须是绝对路径".to_string()).into());
    }
    let db = db.clone();
    let bili_client = bili_client.clone();
    tokio::spawn(async move {
        if let Err(e) =
            crate::task::download_video_by_bvid(&db, bili_client.as_ref(), &bvid, download_path.as_deref()).await
        {
            let config = VersionedConfig::get().snapshot();
            error_and_notify(&config, bili_client.as_ref(), format!("手动下载任务失败：{:#}", e));
        }
    });
    Ok(ApiResponse::ok(true))
}
