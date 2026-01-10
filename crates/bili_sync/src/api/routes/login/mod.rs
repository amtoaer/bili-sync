use std::sync::Arc;

use anyhow::Result;
use axum::extract::{Extension, Query};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;

use crate::api::wrapper::{ApiError, ApiResponse};
use crate::bilibili::{BiliClient, PollStatus, QrcodeLoginResponse};

pub(super) fn router() -> Router {
    Router::new()
        .route("/login/qrcode/generate", post(generate_qrcode))
        .route("/login/qrcode/poll", get(poll_qrcode))
}

/// 生成二维码
pub async fn generate_qrcode(
    Extension(bili_client): Extension<Arc<BiliClient>>,
) -> Result<ApiResponse<QrcodeLoginResponse>, ApiError> {
    let response = bili_client.client.generate_qrcode().await?;
    Ok(ApiResponse::ok(response))
}

#[derive(Debug, Deserialize)]
pub struct PollQrcodeRequest {
    qrcode_key: String,
}

/// 轮询登录状态
pub async fn poll_qrcode(
    Extension(bili_client): Extension<Arc<BiliClient>>,
    Query(params): Query<PollQrcodeRequest>,
) -> Result<ApiResponse<PollStatus>, ApiError> {
    let status = bili_client.client.poll_qrcode(&params.qrcode_key).await?;
    Ok(ApiResponse::ok(status))
}
