use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::Request;
use axum::routing::get;
use axum::{middleware, Extension, Router, ServiceExt};
use sea_orm::DatabaseConnection;
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;

use crate::api::auth;
use crate::api::handler::{bulk_update_videos, get_video, list_videos, update_video};
use crate::config::CONFIG;

pub async fn http_server(database_connection: Arc<DatabaseConnection>) -> Result<()> {
    let app = Router::new()
        .route("/api/videos/", get(list_videos).post(bulk_update_videos))
        .route("/api/videos/{video_id}/", get(get_video).post(update_video))
        .layer(Extension(database_connection))
        .layer(middleware::from_fn(auth::auth));
    let app = NormalizePathLayer::trim_trailing_slash().layer(app);
    let listener = tokio::net::TcpListener::bind(&CONFIG.bind_address)
        .await
        .context("bind address failed")?;
    Ok(axum::serve(listener, ServiceExt::<Request>::into_make_service(app)).await?)
}
