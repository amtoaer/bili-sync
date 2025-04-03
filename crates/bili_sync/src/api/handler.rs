use std::sync::Arc;

use anyhow::{Result, anyhow};
use axum::extract::{Extension, Path, Query};
use axum::Json;
use bili_sync_entity::*;
use bili_sync_migration::{Expr, OnConflict};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Iterable, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait, Unchanged
};
use utoipa::OpenApi;

use crate::api::auth::OpenAPIAuth;
use crate::api::error::InnerApiError;
use crate::api::request::{CreateSourceCollectionRequest, CreateSourceFavoriteRequest, UpdateSourceCollectionRequest, UpdateSourceFavoriteRequest, VideosRequest};
use crate::api::response::{
    PageInfo, ResetVideoResponse, SourceFavoriteResp, VideoInfo, VideoResponse, VideoSource, VideoSourcesResponse, VideosResponse
};
use crate::api::wrapper::{ApiError, ApiResponse,PaginatedResponse};
use crate::utils::status::{PageStatus, VideoStatus};

use crate::api::request::{SourceCollectionsRequest, SourceFavoritesRequest};
use crate::api::response::SourceCollectionResp;

#[derive(OpenApi)]
#[openapi(
    paths(get_video_sources, get_videos, get_video, reset_video,get_source_collections, create_source_collection, update_source_collection, delete_source_collection ),
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
        (0, 10)
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

/// 获取资源集合列表
#[utoipa::path(
    get,
    path = "/api/source-collections",
    params(
        SourceCollectionsRequest,
    ),
    responses(
        (status = 200, body = ApiResponse<PaginatedResponse<SourceCollectionResp>>),
    )
)]
pub async fn get_source_collections(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Query(params): Query<SourceCollectionsRequest>,
) -> Result<ApiResponse<PaginatedResponse<SourceCollectionResp>>, ApiError> {
    let mut query = source_collection::Entity::find();
    
    if let Some(s_id) = params.s_id {
        query = query.filter(source_collection::Column::SId.eq(s_id));
    }
    if let Some(m_id) = params.m_id {
        query = query.filter(source_collection::Column::MId.eq(m_id));
    }
    if let Some(collection_type) = params.r#type {
        query = query.filter(source_collection::Column::Type.eq(collection_type));
    }
    if let Some(created_after) = params.created_after {
        query = query.filter(source_collection::Column::CreatedAt.gte(created_after));
    }
    let total = query.clone().count(db.as_ref()).await?;

    let (page, page_size) = params
        .page
        .zip(params.page_size)
        .unwrap_or((0, 10));
    let collections=query.clone().order_by_desc(source_collection::Column::Id)
        .select_only()
        .columns(source_collection::Column::iter())
        .paginate(db.as_ref(), page_size)
        .fetch_page(page)
        .await?;

    Ok(ApiResponse::ok(PaginatedResponse {
        data: collections
        .into_iter()
        .map(SourceCollectionResp::from)
        .collect(),
        total,
    }))
}

/// 创建新的资源集合
#[utoipa::path(
    post,
    path = "/api/source-collections",
    request_body = CreateSourceCollectionRequest,
    responses(
        (status = 200, body = ApiResponse<SourceCollectionResp>),
    )
)]
pub async fn create_source_collection(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<CreateSourceCollectionRequest>,
) -> Result<ApiResponse<SourceCollectionResp>, ApiError> {
    let new_collection = source_collection::ActiveModel {
        s_id: Set(payload.s_id),
        m_id: Set(payload.m_id),
        r#type: Set(payload.r#type),
        path: Set(payload.path),
        description: Set(payload.description),
        enabled: Set(payload.enabled),
        ..Default::default()
    };

    let result = source_collection::Entity::insert(new_collection)
        .exec_with_returning(db.as_ref())
        .await?;

    Ok(ApiResponse::ok(SourceCollectionResp::from(result)))
}

/// 更新资源集合
#[utoipa::path(
    put,
    path = "/api/source-collections",
    request_body = UpdateSourceCollectionRequest,
    responses(
        (status = 200, body = ApiResponse<SourceCollectionResp>),
    )
)]
pub async fn update_source_collection(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<UpdateSourceCollectionRequest>,
) -> Result<ApiResponse<SourceCollectionResp>, ApiError> {
    let collection = source_collection::Entity::find_by_id(payload.id)
        .one(db.as_ref())
        .await?
        .ok_or(InnerApiError::NotFound(payload.id))?;

    let mut model: source_collection::ActiveModel = collection.into();
    
    if let Some(s_id) = payload.s_id {
        model.s_id = Set(s_id);
    }
    if let Some(m_id) = payload.m_id {
        model.m_id = Set(m_id);
    }
    if let Some(desc) = payload.description {
        model.description = Set(desc);
    }
    if let Some(enabled) = payload.enabled {
        model.enabled = Set(enabled);
    }
    if let Some(path) = payload.path {
        model.path = Set(path);
    }

    let updated = source_collection::Entity::update(model)
    .exec(db.as_ref())
    .await?;
    Ok(ApiResponse::ok(SourceCollectionResp::from(updated)))
}

#[utoipa::path(
    delete,
    path = "/api/source-collections/{id}",
    params(
        ("id" = i32, Path, description = "资源集合ID")
    ),
    responses(
        (status = 200, body = ApiResponse<bool>, example = json!({ "code": 200, "message": "OK", "data": null })),
    )
)]
pub async fn delete_source_collection(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<ApiResponse<bool>, ApiError> {
    source_collection::Entity::delete_by_id(id)
        .exec(db.as_ref())
        .await?;

    Ok(ApiResponse::ok(true))
}


/// 创建新的资源收藏
#[utoipa::path(
    post,
    path = "/api/source-favorites",
    request_body = CreateSourceFavoriteRequest,
    responses(
        (status = 200, body = ApiResponse<SourceFavoriteResp>),
    )
)]
pub async fn create_source_favorite(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<CreateSourceFavoriteRequest>,
) -> Result<ApiResponse<SourceFavoriteResp>, ApiError> {
    let new_favorite = source_favorite::ActiveModel {
        f_id: Set(payload.f_id),
        path: Set(payload.path),
        description: Set(payload.description),
        enabled: Set(payload.enabled),
        ..Default::default()
    };

    let result = source_favorite::Entity::insert(new_favorite)
        .exec_with_returning(db.as_ref())
        .await?;

    Ok(ApiResponse::ok(SourceFavoriteResp::from(result)))
}

/// 获取资源收藏列表
#[utoipa::path(
    get,
    path = "/api/source-favorites",
    params(
        SourceFavoritesRequest,
    ),
    responses(
        (status = 200, body = ApiResponse<Vec<SourceFavoriteResp>>),
    )
)]
pub async fn get_source_favorites(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Query(params): Query<SourceFavoritesRequest>,
) -> Result<ApiResponse<PaginatedResponse<SourceFavoriteResp>>, ApiError> {
 
    let mut query = source_favorite::Entity::find();

    if let Some(f_id) = params.f_id {
        query = query.filter(source_favorite::Column::FId.eq(f_id));
    }
    if let Some(created_after) = params.created_after {
        query = query.filter(source_favorite::Column::CreatedAt.gte(created_after));
    }

    let total = query.clone().count(db.as_ref()).await?;

    let (page, page_size) = params
        .page
        .zip(params.page_size)
        .unwrap_or((0, 10));

    let favorites = query
        .order_by_desc(source_favorite::Column::Id)
        .paginate(db.as_ref(), page_size)
        .fetch_page(page)
        .await?;

    Ok(ApiResponse::ok(
        PaginatedResponse {
            data:  favorites
            .into_iter()
            .map(SourceFavoriteResp::from)
            .collect(),
            total,
        }
    ))
}

/// 更新资源收藏
#[utoipa::path(
    put,
    path = "/api/source-favorites",
    request_body = UpdateSourceFavoriteRequest,
    responses(
        (status = 200, body = ApiResponse<SourceFavoriteResp>),
    )
)]
pub async fn update_source_favorite(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<UpdateSourceFavoriteRequest>,
) -> Result<ApiResponse<SourceFavoriteResp>, ApiError> {
    let favorite = source_favorite::Entity::find_by_id(payload.id)
        .one(db.as_ref())
        .await?
        .ok_or(InnerApiError::NotFound(payload.id))?;

    let mut model: source_favorite::ActiveModel = favorite.into();
    
    if let Some(f_id) = payload.f_id {
        model.f_id = Set(f_id);
    }
    if let Some(path) = payload.path {
        model.path = Set(path);
    }
    if let Some(description) = payload.description {
        model.description = Set(description);
    }
    if let Some(enabled) = payload.enabled {
        model.enabled = Set(enabled);
    }

    let updated = source_favorite::Entity::update(model)
        .exec(db.as_ref())
        .await?;

    Ok(ApiResponse::ok(SourceFavoriteResp::from(updated)))
}

/// 删除资源收藏
#[utoipa::path(
    delete,
    path = "/api/source-favorites/{id}",
    params(
        ("id" = i32, Path, description = "资源收藏ID")
    ),
    responses(
        (status = 200, body = ApiResponse<bool>),
    )
)]
pub async fn delete_source_favorite(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<ApiResponse<bool>, ApiError> {
    source_favorite::Entity::delete_by_id(id)
        .exec(db.as_ref())
        .await?;

    Ok(ApiResponse::ok(true))
}