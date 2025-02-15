use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::Request;
use axum::http::{header, Uri};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{middleware, Extension, Router, ServiceExt};
use reqwest::StatusCode;
use rust_embed::Embed;
use sea_orm::DatabaseConnection;
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;

use crate::api::auth;
use crate::api::handler::{get_video, get_video_list_models, list_videos};
use crate::config::CONFIG;

#[derive(Embed)]
#[folder = "../../web/build"]
struct Asset;

pub async fn http_server(database_connection: Arc<DatabaseConnection>) -> Result<()> {
    let app = Router::new()
        .route("/api/videos", get(list_videos))
        .route("/api/videos/{video_id}", get(get_video))
        .route("/api/video-list-models", get(get_video_list_models))
        .fallback_service(get(frontend_files))
        .layer(Extension(database_connection))
        .layer(middleware::from_fn(auth::auth));
    let app = NormalizePathLayer::trim_trailing_slash().layer(app);
    let listener = tokio::net::TcpListener::bind(&CONFIG.bind_address)
        .await
        .context("bind address failed")?;
    Ok(axum::serve(listener, ServiceExt::<Request>::into_make_service(app)).await?)
}

async fn frontend_files(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        path = "index.html";
    }
    match Asset::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
    }
}
