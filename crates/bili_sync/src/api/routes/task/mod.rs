use anyhow::Result;
use axum::Router;
use axum::routing::post;

use crate::api::wrapper::{ApiError, ApiResponse};
use crate::task::DownloadTaskManager;

pub(super) fn router() -> Router {
    Router::new().route("/task/download", post(new_download_task))
}

pub async fn new_download_task() -> Result<ApiResponse<bool>, ApiError> {
    DownloadTaskManager::get().oneshot().await?;
    Ok(ApiResponse::ok(true))
}
