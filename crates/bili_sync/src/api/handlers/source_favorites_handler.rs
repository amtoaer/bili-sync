use std::sync::Arc;

use axum::extract::{Extension, Path, Query};
use axum::Json;
use bili_sync_entity::*;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set
};
use crate::api::error::InnerApiError;

use crate::api::wrapper::{ApiError, ApiResponse,PaginatedResponse};
use crate::api::request::{ CreateSourceFavoriteRequest, UpdateSourceFavoriteRequest,SourceFavoritesRequest};
use crate::api::response::SourceFavoriteResp;

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
    if let Some(enabled) = params.enabled {
        query = query.filter(source_favorite::Column::Enabled.eq(enabled));
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