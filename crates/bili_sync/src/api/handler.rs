use std::sync::Arc;

use anyhow::{Result, anyhow};
use axum::extract::{Extension, Path, Query};
use bili_sync_entity::*;
use bili_sync_migration::{Expr, OnConflict};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait, Unchanged,
};
use utoipa::OpenApi;

use crate::api::auth::OpenAPIAuth;
use crate::api::error::InnerApiError;
use crate::api::request::VideosRequest;
use crate::api::response::{
    PageInfo, ResetVideoResponse, VideoInfo, VideoResponse, VideoSource, VideoSourcesResponse, VideosResponse,
};
use crate::api::wrapper::{ApiError, ApiResponse};
use crate::utils::status::{PageStatus, VideoStatus};

#[derive(OpenApi)]
#[openapi(
    paths(get_video_sources, get_videos, get_video, reset_video),
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
        (status = 200, body = ApiResponse<VideoSourcesResponse>),
    )
)]
pub async fn get_video_sources(
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<VideoSourcesResponse>, ApiError> {
    Ok(ApiResponse::ok(VideoSourcesResponse {
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
        (status = 200, body = ApiResponse<VideosResponse>),
    )
)]
pub async fn get_videos(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Query(params): Query<VideosRequest>,
) -> Result<ApiResponse<VideosResponse>, ApiError> {
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
    Ok(ApiResponse::ok(VideosResponse {
        videos: query
            .order_by_desc(video::Column::Id)
            .select_only()
            .columns([
                video::Column::Id,
                video::Column::Name,
                video::Column::UpperName,
                video::Column::DownloadStatus,
            ])
            .into_tuple::<(i32, String, String, u32)>()
            .paginate(db.as_ref(), page_size)
            .fetch_page(page)
            .await?
            .into_iter()
            .map(VideoInfo::from)
            .collect(),
        total_count,
    }))
}

/// 获取视频详细信息，包括关联的所有 page
#[utoipa::path(
    get,
    path = "/api/videos/{id}",
    responses(
        (status = 200, body = ApiResponse<VideoResponse>),
    )
)]
pub async fn get_video(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<VideoResponse>, ApiError> {
    let video_info = video::Entity::find_by_id(id)
        .select_only()
        .columns([
            video::Column::Id,
            video::Column::Name,
            video::Column::UpperName,
            video::Column::DownloadStatus,
        ])
        .into_tuple::<(i32, String, String, u32)>()
        .one(db.as_ref())
        .await?
        .map(VideoInfo::from);
    let Some(video_info) = video_info else {
        return Err(InnerApiError::NotFound(id).into());
    };
    let pages = page::Entity::find()
        .filter(page::Column::VideoId.eq(id))
        .order_by_asc(page::Column::Pid)
        .select_only()
        .columns([
            page::Column::Id,
            page::Column::Pid,
            page::Column::Name,
            page::Column::DownloadStatus,
        ])
        .into_tuple::<(i32, i32, String, u32)>()
        .all(db.as_ref())
        .await?
        .into_iter()
        .map(PageInfo::from)
        .collect();
    Ok(ApiResponse::ok(VideoResponse {
        video: video_info,
        pages,
    }))
}

/// 将某个视频与其所有分页的失败状态清空为未下载状态，这样在下次下载任务中会触发重试
#[utoipa::path(
    post,
    path = "/api/videos/{id}/reset",
    responses(
        (status = 200, body = ApiResponse<ResetVideoResponse> ),
    )
)]
pub async fn reset_video(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<ResetVideoResponse>, ApiError> {
    let txn = db.begin().await?;
    let video_status: Option<u32> = video::Entity::find_by_id(id)
        .select_only()
        .column(video::Column::DownloadStatus)
        .into_tuple()
        .one(&txn)
        .await?;
    let Some(video_status) = video_status else {
        return Err(anyhow!(InnerApiError::NotFound(id)).into());
    };
    let resetted_pages_model: Vec<_> = page::Entity::find()
        .filter(page::Column::VideoId.eq(id))
        .all(&txn)
        .await?
        .into_iter()
        .filter_map(|mut model| {
            let mut page_status = PageStatus::from(model.download_status);
            if page_status.reset_failed() {
                model.download_status = page_status.into();
                Some(model)
            } else {
                None
            }
        })
        .collect();
    let mut video_status = VideoStatus::from(video_status);
    let mut should_update_video = video_status.reset_failed();
    if !resetted_pages_model.is_empty() {
        // 视频状态标志的第 5 位表示是否有分 P 下载失败，如果有需要重置的分页，需要同时重置视频的该状态
        video_status.set(4, 0);
        should_update_video = true;
    }
    if should_update_video {
        video::Entity::update(video::ActiveModel {
            id: Unchanged(id),
            download_status: Set(video_status.into()),
            ..Default::default()
        })
        .exec(&txn)
        .await?;
    }
    let resetted_pages_id: Vec<_> = resetted_pages_model.iter().map(|model| model.id).collect();
    let resetted_pages_model: Vec<page::ActiveModel> = resetted_pages_model
        .into_iter()
        .map(|model| model.into_active_model())
        .collect();
    for page_trunk in resetted_pages_model.chunks(50) {
        page::Entity::insert_many(page_trunk.to_vec())
            .on_conflict(
                OnConflict::column(page::Column::Id)
                    .update_column(page::Column::DownloadStatus)
                    .to_owned(),
            )
            .exec(&txn)
            .await?;
    }
    txn.commit().await?;
    Ok(ApiResponse::ok(ResetVideoResponse {
        resetted: should_update_video,
        video: id,
        pages: resetted_pages_id,
    }))
}
