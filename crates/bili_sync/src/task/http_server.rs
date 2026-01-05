use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::Request;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, ServiceExt};
use reqwest::StatusCode;
use rust_embed_for_web::{EmbedableFile, RustEmbed};
use sea_orm::DatabaseConnection;

use crate::api::{LogHelper, router};
use crate::bilibili::BiliClient;
use crate::config::{VersionedConfig, default_bind_address};

#[derive(RustEmbed)]
#[preserve_source = false]
#[folder = "../../web/build"]
struct Asset;

pub async fn http_server(
    database_connection: DatabaseConnection,
    bili_client: Arc<BiliClient>,
    log_writer: LogHelper,
) -> Result<()> {
    let app = router()
        .fallback_service(get(frontend_files).head(frontend_files))
        .layer(Extension(database_connection))
        .layer(Extension(bili_client))
        .layer(Extension(log_writer));
    let (bind_address, listener) = {
        let bind_address = VersionedConfig::get().read().bind_address.to_owned();
        let listen_res = tokio::net::TcpListener::bind(&bind_address)
            .await
            .context("bind address failed");
        match listen_res {
            Ok(listener) => (bind_address, listener),
            Err(e) => {
                let default_bind_address = default_bind_address();
                if default_bind_address == bind_address {
                    return Err(e);
                }
                warn!(
                    "绑定到地址 {} 失败：{:#}，尝试绑定到默认地址 {}",
                    bind_address, e, default_bind_address
                );
                let listener = tokio::net::TcpListener::bind(&default_bind_address)
                    .await
                    .context("bind default address failed")?;
                (default_bind_address, listener)
            }
        }
    };
    info!("开始运行管理页：http://{}", bind_address);
    Ok(axum::serve(listener, ServiceExt::<Request>::into_make_service(app)).await?)
}

async fn frontend_files(request: Request) -> impl IntoResponse {
    let mut path = request.uri().path().trim_start_matches('/');
    if path.is_empty() || Asset::get(path).is_none() {
        path = "index.html";
    }
    let Some(content) = Asset::get(path) else {
        return (StatusCode::NOT_FOUND, "404 Not Found").into_response();
    };
    let mime_type = content.mime_type();
    let content_type = mime_type.as_deref().unwrap_or("application/octet-stream");
    let default_headers = [
        (header::CONTENT_TYPE, content_type),
        (header::CACHE_CONTROL, "no-cache"),
        (header::ETAG, &content.hash()),
    ];
    if let Some(if_none_match) = request.headers().get(header::IF_NONE_MATCH)
        && let Ok(client_etag) = if_none_match.to_str()
        && client_etag == content.hash()
    {
        return (StatusCode::NOT_MODIFIED, default_headers).into_response();
    }

    if request.method() == axum::http::Method::HEAD {
        return (StatusCode::OK, default_headers).into_response();
    }
    if cfg!(debug_assertions) {
        // safety: `RustEmbed` returns uncompressed files directly from the filesystem in debug mode
        return (StatusCode::OK, default_headers, content.data().unwrap()).into_response();
    }
    let accepted_encodings = request
        .headers()
        .get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').map(str::trim).collect::<HashSet<_>>())
        .unwrap_or_default();
    for (encoding, data) in [("br", content.data_br()), ("gzip", content.data_gzip())] {
        if accepted_encodings.contains(encoding)
            && let Some(data) = data
        {
            return (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, content_type),
                    (header::CACHE_CONTROL, "no-cache"),
                    (header::ETAG, &content.hash()),
                    (header::CONTENT_ENCODING, encoding),
                ],
                data,
            )
                .into_response();
        }
    }
    (
        StatusCode::NOT_ACCEPTABLE,
        "Client must support gzip or brotli compression",
    )
        .into_response()
}
