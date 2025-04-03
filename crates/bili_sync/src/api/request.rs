use serde::Deserialize;
use utoipa::IntoParams;
use utoipa::ToSchema;

#[derive(Deserialize, IntoParams)]
pub struct VideosRequest {
    pub collection: Option<i32>,
    pub favorite: Option<i32>,
    pub submission: Option<i32>,
    pub watch_later: Option<i32>,
    pub query: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}


// 查询参数结构
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct SourceCollectionsRequest {
    #[param(example = 1)]
    pub s_id: Option<i64>,
    #[param(example = 1)]
    pub m_id: Option<i64>,
    #[param(example = 1)]
    pub r#type: Option<i16>,
    #[param(example = "2023-01-01T00:00:00Z")]
    pub created_after: Option<String>,
    #[param(example = 0)]
    pub enabled: Option<i32>,
    #[param(example = 0)]
    pub page: Option<u64>,
    #[param(example = 10)]
    pub page_size: Option<u64>,
}

// 创建请求体
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSourceCollectionRequest {
    pub s_id: i64,
    pub m_id: i64,
    pub r#type: i16,
    pub path: String,
    pub description: String,
    pub enabled: i32,
}

// 更新请求体
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSourceCollectionRequest {
    pub id: i32,
    pub s_id: Option<i64>,
    pub m_id: Option<i64>,
    pub description: Option<String>,
    pub enabled: Option<i32>,
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct SourceFavoritesRequest {
    #[param(example = 1)]
    pub f_id: Option<i64>,
    #[param(example = "2023-01-01T00:00:00Z")]
    pub created_after: Option<String>,
    #[param(example = 0)]
    pub enabled: Option<i32>,
    #[param(example = 0)]
    pub page: Option<u64>,
    #[param(example = 10)]
    pub page_size: Option<u64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSourceFavoriteRequest {
    pub f_id: i64,
    pub path: String,
    pub description: String,
    pub enabled: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSourceFavoriteRequest {
    pub id: i32,
    pub f_id: Option<i64>,
    pub path: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<i32>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct SourceSubmissionsRequest {
    #[param(example = 0)]
    pub upper_id: Option<i64>,
    #[param(example = "2023-01-01T00:00:00Z")]
    pub created_after: Option<String>,
    #[param(example = 0)]
    pub enabled: Option<i32>,
    #[param(example = 0)]
    pub page: Option<u64>,
    #[param(example = 10)]
    pub page_size: Option<u64>,
}
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSourceSubmissionRequest {
    pub upper_id: i64,
    pub path: String,
    pub description: String,
    pub enabled: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSourceSubmissionRequest {
    pub id: i32,
    pub upper_id: Option<i64>,
    pub path: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<i32>,
}


#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSourceWatchLaterRequest {
    pub path: String,
    pub description: String,
    pub enabled: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSourceWatchLaterRequest {
    pub id: i32,
    pub path: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<i32>,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct SourceWatchLaterRequest {
    #[param(example = 0)]
    pub created_after: Option<String>,
    #[param(example = 0)]
    pub enabled: Option<i32>,
    #[param(example = 0)]
    pub page: Option<u64>,
    #[param(example = 10)]
    pub page_size: Option<u64>,
}