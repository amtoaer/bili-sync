use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::Request;
use axum::http::{Uri, header};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::{Extension, Router, ServiceExt, middleware};
use reqwest::StatusCode;
use rust_embed::Embed;
use sea_orm::DatabaseConnection;
use utoipa::OpenApi;
use utoipa_swagger_ui::{Config, SwaggerUi};

use crate::api::auth;
use crate::api::handler::{
    ApiDoc, create_source_collection, delete_source_collection, get_source_collections, get_video, get_video_sources,
    get_videos, reset_video, update_source_collection,
    create_source_favorite, delete_source_favorite, get_source_favorites, update_source_favorite,
};
use crate::config::CONFIG;

#[derive(Embed)]
#[folder = "../../web/build"]
struct Asset;

pub async fn http_server(database_connection: Arc<DatabaseConnection>) -> Result<()> {
    let app = Router::new()
        .route("/api/video-sources", get(get_video_sources))
        .route("/api/videos", get(get_videos))
        .route("/api/videos/{id}", get(get_video))
        .route("/api/videos/{id}/reset", post(reset_video))
        // Route for listing source collections
        .route("/api/source-collections", get(get_source_collections))
        // Route for creating a new source collection
        .route("/api/source-collections", post(create_source_collection))
        // Route for updating a source collection
        .route("/api/source-collections", put(update_source_collection))
        // Route for deleting a source collection
        .route("/api/source-collections/{id}", delete(delete_source_collection))
        .route("/api/source-favorites", post(create_source_favorite))
        .route("/api/source-favorites", get(get_source_favorites))
        .route("/api/source-favorites", put(update_source_favorite))
        .route("/api/source-favorites/{id}", delete(delete_source_favorite))
        // Add the database connection as an extension
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
        .layer(middleware::from_fn(auth::auth));
    let listener = tokio::net::TcpListener::bind(&CONFIG.bind_address)
        .await
        .context("bind address failed")?;
    info!("开始运行管理页: http://{}", CONFIG.bind_address);
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
