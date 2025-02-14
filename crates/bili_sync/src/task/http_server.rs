use std::sync::Arc;

use anyhow::{Context, Result};
use axum::routing::get;
use axum::{middleware, Extension, Router};
use sea_orm::DatabaseConnection;

use crate::api::auth;
use crate::api::handler::{bulk_update_videos, get_video, list_videos, update_video};

pub async fn http_server(database_connection: Arc<DatabaseConnection>) -> Result<()> {
    let app = Router::new()
        .route("/api/videos", get(list_videos).post(bulk_update_videos))
        .route("/api/videos/{video_id}/", get(get_video).post(update_video))
        .layer(Extension(database_connection))
        .layer(middleware::from_fn(auth::auth));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:12345")
        .await
        .context("bind address failed")?;
    Ok(axum::serve(listener, app).await?)
}
