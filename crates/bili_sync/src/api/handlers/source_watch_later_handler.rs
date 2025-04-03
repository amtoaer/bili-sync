use std::sync::Arc;

use axum::extract::{Extension, Path, Query};
use axum::Json;
use bili_sync_entity::*;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set
};
use crate::api::error::InnerApiError;
use crate::api::wrapper::{ApiError, ApiResponse,PaginatedResponse};
use crate::api::request::{SourceWatchLaterRequest, CreateSourceWatchLaterRequest, UpdateSourceWatchLaterRequest};
use crate::api::response::SourceWatchLaterResp;


/// 创建资源稍后观看
#[utoipa::path(
    post,
    path = "/api/source-watch-later",
    request_body = CreateSourceWatchLaterRequest,
    responses(
        (status = 200, body = ApiResponse<SourceWatchLaterResp>),
    )
)]
pub async fn create_source_watch_later(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<CreateSourceWatchLaterRequest>,
) -> Result<ApiResponse<SourceWatchLaterResp>, ApiError> {
    let new_watch_later = source_watch_later::ActiveModel {
        path: Set(payload.path),
        description: Set(payload.description),
        enabled: Set(payload.enabled),
        ..Default::default()
    };

    let result = source_watch_later::Entity::insert(new_watch_later)
        .exec_with_returning(db.as_ref())
        .await?;

    Ok(ApiResponse::ok(SourceWatchLaterResp::from(result)))
}


/// 获取资源稍后观看
#[utoipa::path(
    get,
    path = "/api/source-watch-later",
    params(
        SourceWatchLaterRequest,
    ),
    responses(
        (status = 200, body = ApiResponse<Vec<SourceWatchLaterResp>>),
    )
)]
pub async fn get_source_watch_later(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Query(params): Query<SourceWatchLaterRequest>,
) -> Result<ApiResponse<PaginatedResponse<SourceWatchLaterResp>>, ApiError> {
    let mut query = source_watch_later::Entity::find();

    if let Some(created_after) = params.created_after {
        query = query.filter(source_watch_later::Column::CreatedAt.gte(created_after));
    }
    if let Some(enabled) = params.enabled {
        query = query.filter(source_watch_later::Column::Enabled.eq(enabled));
    }  

    let total = query.clone().count(db.as_ref()).await?;

    let (page, page_size) = params
        .page
        .zip(params.page_size)
        .unwrap_or((0, 10));

    let watch_later = query
        .order_by_desc(source_watch_later::Column::Id)
        .paginate(db.as_ref(), page_size)
        .fetch_page(page)
        .await?;

    Ok(ApiResponse::ok(
        PaginatedResponse {
            data: watch_later
                .into_iter()
                .map(SourceWatchLaterResp::from)
                .collect(),
            total,
        }
    ))
}


/// 更新资源稍后观看
#[utoipa::path(
    put,
    path = "/api/source-watch-later",
    request_body = UpdateSourceWatchLaterRequest,
    responses(
        (status = 200, body = ApiResponse<SourceWatchLaterResp>),
    )
)]
pub async fn update_source_watch_later(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<UpdateSourceWatchLaterRequest>,
) -> Result<ApiResponse<SourceWatchLaterResp>, ApiError> {
    let watch_later = source_watch_later::Entity::find_by_id(payload.id)
        .one(db.as_ref())
        .await?
        .ok_or(InnerApiError::NotFound(payload.id))?;

    let mut model: source_watch_later::ActiveModel = watch_later.into();
    
    if let Some(path) = payload.path {
        model.path = Set(path);
    }
    if let Some(description) = payload.description {
        model.description = Set(description);
    }
    if let Some(enabled) = payload.enabled {
        model.enabled = Set(enabled);
    }

    let updated = source_watch_later::Entity::update(model)
        .exec(db.as_ref())
        .await?;

    Ok(ApiResponse::ok(SourceWatchLaterResp::from(updated)))
}

/// 删除资源稍后观看
#[utoipa::path(
    delete,
    path = "/api/source-watch-later/{id}",
    params(
        ("id" = i32, Path, description = "资源稍后观看ID")
    ),
    responses(
        (status = 200, body = ApiResponse<bool>),
    )
)]
pub async fn delete_source_watch_later(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<ApiResponse<bool>, ApiError> {
    source_watch_later::Entity::delete_by_id(id)
        .exec(db.as_ref())
        .await?;

    Ok(ApiResponse::ok(true))
}