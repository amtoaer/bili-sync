use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use axum::extract::{Extension, Query};
use axum::routing::{get, post};

use crate::api::request::PollQrcodeRequest;
use crate::api::response::{GenerateQrcodeResponse, PollQrcodeResponse};
use crate::api::wrapper::{ApiError, ApiResponse};
use crate::bilibili::{BiliClient, Credential};

pub(super) fn router() -> Router {
    Router::new()
        .route("/login/qrcode/generate", post(generate_qrcode))
        .route("/login/qrcode/poll", get(poll_qrcode))
}

/// 生成扫码登录二维码
pub async fn generate_qrcode(
    Extension(bili_client): Extension<Arc<BiliClient>>,
) -> Result<ApiResponse<GenerateQrcodeResponse>, ApiError> {
    Ok(ApiResponse::ok(Credential::generate_qrcode(&bili_client.client).await?))
}

/// 轮询扫码登录状态
pub async fn poll_qrcode(
    Extension(bili_client): Extension<Arc<BiliClient>>,
    Query(params): Query<PollQrcodeRequest>,
) -> Result<ApiResponse<PollQrcodeResponse>, ApiError> {
    Ok(ApiResponse::ok(
        Credential::poll_qrcode(&bili_client.client, &params.qrcode_key).await?,
    ))
}
