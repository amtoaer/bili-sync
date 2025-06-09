use std::sync::Arc;

use anyhow::{Context, Result};
use axum::body::Body;
use axum::extract::Request;
use axum::http::{Response, Uri, header};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Router, ServiceExt, middleware};
use reqwest::StatusCode;
use rust_embed_for_web::{EmbedableFile, RustEmbed};
use sea_orm::DatabaseConnection;
use utoipa::OpenApi;
use utoipa_swagger_ui::{Config, SwaggerUi};

use crate::api::auth;
use crate::api::handler::{ApiDoc, api_router};
use crate::bilibili::BiliClient;
use crate::config::CONFIG;

#[derive(RustEmbed)]
#[preserve_source = false]
#[gzip = false]
#[folder = "../../web/build"]
struct Asset;

pub async fn http_server(database_connection: Arc<DatabaseConnection>, bili_client: Arc<BiliClient>) -> Result<()> {
    let app = Router::new()
        .merge(api_router())
        .merge(
            SwaggerUi::new("/swagger-ui/")
                .url("/api-docs/openapi.json", ApiDoc::openapi())
                .config(
                    Config::default()
                        .try_it_out_enabled(true)
                        .persist_authorization(true)
                        .validator_url("none"),
                ),
        )
        .fallback_service(get(frontend_files))
        .layer(Extension(database_connection))
        .layer(Extension(bili_client))
        .layer(middleware::from_fn(auth::auth));
    let listener = tokio::net::TcpListener::bind(&CONFIG.bind_address)
        .await
        .context("bind address failed")?;
    info!("开始运行管理页: http://{}", CONFIG.bind_address);
    Ok(axum::serve(listener, ServiceExt::<Request>::into_make_service(app)).await?)
}

async fn frontend_files(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/');
    if path.is_empty() || Asset::get(path).is_none() {
        path = "index.html";
    }
    let Some(content) = Asset::get(path) else {
        return (StatusCode::NOT_FOUND, "404 Not Found").into_response();
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            content.mime_type().as_deref().unwrap_or("application/octet-stream"),
        )
        .header(header::CONTENT_ENCODING, "br")
        // safety: `RustEmbed` will always generate br-compressed files if the feature is enabled
        .body(Body::from(content.data_br().unwrap()))
        .unwrap_or_else(|_| {
            return (StatusCode::INTERNAL_SERVER_ERROR, "500 Internal Server Error").into_response();
        })
}
