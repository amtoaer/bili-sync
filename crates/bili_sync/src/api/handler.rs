use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::extract::{Extension, Path, Query};
use axum::Json;
use bili_sync_entity::*;
use bili_sync_migration::Expr;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use utoipa::OpenApi;

use crate::api::auth::OpenAPIAuth;
use crate::api::error::ApiError;
use crate::api::request::VideosRequest;
use crate::api::response::{PageInfo, VideoInfo, VideoResponse, VideoSource, VideoSourcesResponse, VideosResponse};

#[derive(OpenApi)]
#[openapi(
    paths(get_video_sources, get_videos, get_video),
    modifiers(&OpenAPIAuth),
    security(
        ("Token" = []),
    )
)]
pub struct ApiDoc;

/// 列出所有视频来源
#[utoipa::path(
    get,
    path = "/api/video-sources",
    responses(
        (status = 200, body = VideoSourcesResponse),
    )
)]
pub async fn get_video_sources(
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<Json<VideoSourcesResponse>, ApiError> {
    Ok(Json(VideoSourcesResponse {
        collection: collection::Entity::find()
            .select_only()
            .columns([collection::Column::Id, collection::Column::Name])
            .into_model::<VideoSource>()
            .all(db.as_ref())
            .await?,
        favorite: favorite::Entity::find()
            .select_only()
            .columns([favorite::Column::Id, favorite::Column::Name])
            .into_model::<VideoSource>()
            .all(db.as_ref())
            .await?,
        submission: submission::Entity::find()
            .select_only()
            .column(submission::Column::Id)
            .column_as(submission::Column::UpperName, "name")
            .into_model::<VideoSource>()
            .all(db.as_ref())
            .await?,
        watch_later: watch_later::Entity::find()
            .select_only()
            .column(watch_later::Column::Id)
            .column_as(Expr::value("稍后再看"), "name")
            .into_model::<VideoSource>()
            .all(db.as_ref())
            .await?,
    }))
}

/// 列出视频的基本信息，支持根据视频来源筛选、名称查找和分页
#[utoipa::path(
    get,
    path = "/api/videos",
    params(
        VideosRequest,
    ),
    responses(
        (status = 200, body = VideosResponse),
    )
)]
pub async fn get_videos(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Query(params): Query<VideosRequest>,
) -> Result<Json<VideosResponse>, ApiError> {
    let mut query = video::Entity::find();
    for (field, column) in [
        (params.collection, video::Column::CollectionId),
        (params.favorite, video::Column::FavoriteId),
        (params.submission, video::Column::SubmissionId),
        (params.watch_later, video::Column::WatchLaterId),
    ] {
        if let Some(id) = field {
            query = query.filter(column.eq(id));
        }
    }
    if let Some(query_word) = params.query {
        query = query.filter(video::Column::Name.contains(query_word));
    }
    let total_count = query.clone().count(db.as_ref()).await?;
    let (page, page_size) = if let (Some(page), Some(page_size)) = (params.page, params.page_size) {
        (page, page_size)
    } else {
        (1, 10)
    };
    Ok(Json(VideosResponse {
        videos: query
            .order_by_desc(video::Column::Id)
            .into_partial_model::<VideoInfo>()
            .paginate(db.as_ref(), page_size)
            .fetch_page(page)
            .await?,
        total_count,
    }))
}

/// 获取视频详细信息，包括关联的所有 page
#[utoipa::path(
    get,
    path = "/api/videos/{id}",
    responses(
        (status = 200, body = VideoResponse),
    )
)]
pub async fn get_video(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<Json<VideoResponse>, ApiError> {
    let video_info = video::Entity::find_by_id(id)
        .into_partial_model::<VideoInfo>()
        .one(db.as_ref())
        .await?;
    let Some(video_info) = video_info else {
        return Err(anyhow!("视频不存在").into());
    };
    let pages = page::Entity::find()
        .filter(page::Column::VideoId.eq(id))
        .order_by_asc(page::Column::Pid)
        .into_partial_model::<PageInfo>()
        .all(db.as_ref())
        .await?;
    Ok(Json(VideoResponse {
        video: video_info,
        pages,
    }))
}
