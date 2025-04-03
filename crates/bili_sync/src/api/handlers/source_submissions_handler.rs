use std::sync::Arc;

use axum::extract::{Extension, Path, Query};
use axum::Json;
use bili_sync_entity::*;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set
};
use crate::api::error::InnerApiError;
use crate::api::wrapper::{ApiError, ApiResponse,PaginatedResponse};
use crate::api::request::{CreateSourceSubmissionRequest, SourceSubmissionsRequest, UpdateSourceSubmissionRequest};
use crate::api::response::SourceSubmissionResp;

/// 创建资源提交
#[utoipa::path(
    post,
    path = "/api/source-submissions",
    request_body = CreateSourceSubmissionRequest,
    responses(
        (status = 200, body = ApiResponse<SourceSubmissionResp>),
    )
)]
pub async fn create_source_submission(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<CreateSourceSubmissionRequest>,
) -> Result<ApiResponse<SourceSubmissionResp>, ApiError> {
    let new_submission = source_submission::ActiveModel {
        upper_id: Set(payload.upper_id),
        path: Set(payload.path),
        description: Set(payload.description),
        enabled: Set(payload.enabled),
        ..Default::default()
    };

    let result = source_submission::Entity::insert(new_submission)
        .exec_with_returning(db.as_ref())
        .await?;

    Ok(ApiResponse::ok(SourceSubmissionResp::from(result)))
}

/// 获取资源提交
#[utoipa::path(
    get,
    path = "/api/source-submissions",
    params(
        SourceSubmissionsRequest,
    ),
    responses(
        (status = 200, body = ApiResponse<Vec<SourceSubmissionResp>>),
    )
)]
pub async fn get_source_submissions(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Query(params): Query<SourceSubmissionsRequest>,
) -> Result<ApiResponse<PaginatedResponse<SourceSubmissionResp>>, ApiError> {
    let mut query = source_submission::Entity::find();

    if let Some(upper_id) = params.upper_id {
        query = query.filter(source_submission::Column::UpperId.eq(upper_id));
    }
    if let Some(created_after) = params.created_after {
        query = query.filter(source_submission::Column::CreatedAt.gte(created_after));
    }
    if let Some(enabled) = params.enabled {
        query = query.filter(source_submission::Column::Enabled.eq(enabled));
    }  

    let total = query.clone().count(db.as_ref()).await?;

    let (page, page_size) = params
        .page
        .zip(params.page_size)
        .unwrap_or((0, 10));

    let submissions = query
        .order_by_desc(source_submission::Column::Id)
        .paginate(db.as_ref(), page_size)
        .fetch_page(page)
        .await?;

    Ok(ApiResponse::ok(
        PaginatedResponse {
            data:  submissions
            .into_iter()
            .map(SourceSubmissionResp::from)
            .collect(),
            total,
        }
    ))
}


/// 更新资源提交
#[utoipa::path(
    put,
    path = "/api/source-submissions",
    request_body = UpdateSourceSubmissionRequest,
    responses(
        (status = 200, body = ApiResponse<SourceSubmissionResp>),
    )
)]
pub async fn update_source_submission(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(payload): Json<UpdateSourceSubmissionRequest>,
) -> Result<ApiResponse<SourceSubmissionResp>, ApiError> {
    let submission = source_submission::Entity::find_by_id(payload.id)
        .one(db.as_ref())
        .await?
        .ok_or(InnerApiError::NotFound(payload.id))?;

    let mut model: source_submission::ActiveModel = submission.into();
    
    if let Some(upper_id) = payload.upper_id {
        model.upper_id = Set(upper_id);
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

    let updated = source_submission::Entity::update(model)
        .exec(db.as_ref())
        .await?;

    Ok(ApiResponse::ok(SourceSubmissionResp::from(updated)))
}


/// 删除资源提交
#[utoipa::path(
    delete,
    path = "/api/source-submissions/{id}",
    params(
        ("id" = i32, Path, description = "资源提交ID")
    ),
    responses(
        (status = 200, body = ApiResponse<bool>),
    )
)]
pub async fn delete_source_submission(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<ApiResponse<bool>, ApiError> {
    source_submission::Entity::delete_by_id(id)
        .exec(db.as_ref())
        .await?;

    Ok(ApiResponse::ok(true))
}