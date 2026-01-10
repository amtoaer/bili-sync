use axum::extract::Request;
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Router, middleware};
use base64::Engine;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use reqwest::StatusCode;

use crate::api::wrapper::ApiResponse;
use crate::config::VersionedConfig;

mod config;
mod dashboard;
mod login;
mod me;
mod task;
mod video_sources;
mod videos;
mod ws;

pub use ws::{LogHelper, MAX_HISTORY_LOGS};

pub fn router() -> Router {
    Router::new().nest(
        "/api",
        config::router()
            .merge(me::router())
            .merge(login::router())
            .merge(video_sources::router())
            .merge(videos::router())
            .merge(dashboard::router())
            .merge(ws::router())
            .merge(task::router())
            .layer(middleware::from_fn(auth)),
    )
}

/// 中间件：使用 auth token 对请求进行身份验证
pub async fn auth(mut headers: HeaderMap, request: Request, next: Next) -> Result<Response, StatusCode> {
    let config = VersionedConfig::get().read();
    let token = config.auth_token.as_str();
    if headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s == token)
    {
        return Ok(next.run(request).await);
    }
    if let Some(protocol) = headers.remove("Sec-WebSocket-Protocol")
        && protocol
            .to_str()
            .ok()
            .and_then(|s| BASE64_URL_SAFE_NO_PAD.decode(s).ok())
            .is_some_and(|s| s == token.as_bytes())
    {
        let mut resp = next.run(request).await;
        resp.headers_mut().insert("Sec-WebSocket-Protocol", protocol);
        return Ok(resp);
    }
    Ok(ApiResponse::<()>::unauthorized("auth token does not match").into_response())
}
