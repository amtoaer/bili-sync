use axum::extract::Request;
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use reqwest::StatusCode;

use crate::api::wrapper::ApiResponse;
use crate::config::VersionedConfig;

pub async fn auth(headers: HeaderMap, request: Request, next: Next) -> Result<Response, StatusCode> {
    if request.uri().path().starts_with("/api/")
        && get_token(&headers).is_none_or(|token| token != VersionedConfig::get().load().auth_token)
    {
        return Ok(ApiResponse::<()>::unauthorized("auth token does not match").into_response());
    }
    Ok(next.run(request).await)
}

fn get_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .map(Into::into)
}
