use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use axum::extract::{Extension, Query};
use axum::routing::{get, post};
use serde::Deserialize;

use crate::api::wrapper::{ApiError, ApiResponse};
use crate::bilibili::{BiliClient, PollStatus, QrcodeLoginResponse};

pub(super) fn router() -> Router {
    Router::new()
        .route("/login/qrcode/generate", post(generate_qrcode))
        .route("/login/qrcode/poll", get(poll_qrcode))
}

/// 生成扫码登录二维码
///
/// # HTTP 端点
///
/// `POST /api/login/qrcode/generate`
///
/// # 响应
///
/// 返回 `QrcodeLoginResponse`，包含：
/// - `url`: 需要转换为二维码的 URL（不是图片链接）
/// - `qrcode_key`: 用于轮询的认证 token
pub async fn generate_qrcode(
    Extension(bili_client): Extension<Arc<BiliClient>>,
) -> Result<ApiResponse<QrcodeLoginResponse>, ApiError> {
    let response = bili_client.client.generate_qrcode().await?;
    Ok(ApiResponse::ok(response))
}

/// 轮询登录状态请求参数
#[derive(Debug, Deserialize)]
pub struct PollQrcodeRequest {
    /// 二维码认证 token（由 generate_qrcode 返回）
    qrcode_key: String,
}

/// 轮询扫码登录状态
///
/// # HTTP 端点
///
/// `GET /api/login/qrcode/poll?qrcode_key={key}`
///
/// # 参数
///
/// - `qrcode_key`: 二维码认证 token
///
/// # 响应
///
/// 返回 `PollStatus` 枚举：
/// - `Success { credential }`: 登录成功，包含完整凭证
/// - `Pending { message, scanned }`: 等待中，scanned 表示是否已扫描
/// - `Expired { message }`: 二维码已过期
pub async fn poll_qrcode(
    Extension(bili_client): Extension<Arc<BiliClient>>,
    Query(params): Query<PollQrcodeRequest>,
) -> Result<ApiResponse<PollStatus>, ApiError> {
    let status = bili_client.client.poll_qrcode(&params.qrcode_key).await?;
    Ok(ApiResponse::ok(status))
}
