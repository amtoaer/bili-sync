use std::collections::HashSet;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Extension, Query, Request};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Router, middleware};
use reqwest::{Method, StatusCode, header};

use super::request::ImageProxyParams;
use crate::api::wrapper::ApiResponse;
use crate::bilibili::BiliClient;
use crate::config::VersionedConfig;

mod config;
mod dashboard;
mod me;
mod video_sources;
mod videos;
mod ws;

pub use ws::{LogHelper, MAX_HISTORY_LOGS};

pub fn router() -> Router {
    Router::new().route("/image-proxy", get(image_proxy)).nest(
        "/api",
        config::router()
            .merge(me::router())
            .merge(video_sources::router())
            .merge(videos::router())
            .merge(dashboard::router())
            .merge(ws::router())
            .layer(middleware::from_fn(auth)),
    )
}

/// 中间件：验证请求头中的 Authorization 是否与配置中的 auth_token 匹配
pub async fn auth(headers: HeaderMap, request: Request, next: Next) -> Result<Response, StatusCode> {
    let config = VersionedConfig::get().load();
    let token = config.auth_token.as_str();
    if headers
        .get("Authorization")
        .is_some_and(|v| v.to_str().is_ok_and(|s| s == token))
    {
        return Ok(next.run(request).await);
    }
    Ok(ApiResponse::<()>::unauthorized("auth token does not match").into_response())
}

/// B 站的图片会检查 referer，需要做个转发伪造一下，否则直接返回 403
pub async fn image_proxy(
    Extension(bili_client): Extension<Arc<BiliClient>>,
    Query(params): Query<ImageProxyParams>,
) -> Response {
    let resp = bili_client.client.request(Method::GET, &params.url, None).send().await;
    let whitelist = [
        header::CONTENT_TYPE,
        header::CONTENT_LENGTH,
        header::CACHE_CONTROL,
        header::EXPIRES,
        header::LAST_MODIFIED,
        header::ETAG,
        header::CONTENT_DISPOSITION,
        header::CONTENT_ENCODING,
        header::ACCEPT_RANGES,
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
    ]
    .into_iter()
    .collect::<HashSet<_>>();

    let builder = Response::builder();

    let response = match resp {
        Err(e) => builder.status(StatusCode::BAD_GATEWAY).body(Body::new(e.to_string())),
        Ok(res) => {
            let mut response = builder.status(res.status());
            for (k, v) in res.headers() {
                if whitelist.contains(k) {
                    response = response.header(k, v);
                }
            }
            let streams = res.bytes_stream();
            response.body(Body::from_stream(streams))
        }
    };
    //safety: all previously configured headers are taken from a valid response, ensuring the response is safe to use
    response.unwrap()
}
