use std::sync::Arc;

use anyhow::Result;
use axum::extract::{Extension, Path, Query};
use axum::Json;
use bili_sync_entity::*;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, Iterable, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set
};

use crate::api::error::InnerApiError;
use crate::api::wrapper::{ApiError, ApiResponse,PaginatedResponse};
use crate::api::request::{CreateSourceCollectionRequest, SourceCollectionsRequest, UpdateSourceCollectionRequest};
use crate::api::response::SourceCollectionResp;


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
    if let Some(enabled) = params.enabled {
        query = query.filter(source_collection::Column::Enabled.eq(enabled));
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