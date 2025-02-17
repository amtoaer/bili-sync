use bili_sync_entity::*;
use sea_orm::{DerivePartialModel, FromQueryResult};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct VideoSourcesResponse {
    pub collection: Vec<VideoSource>,
    pub favorite: Vec<VideoSource>,
    pub submission: Vec<VideoSource>,
    pub watch_later: Vec<VideoSource>,
}

#[derive(Serialize, ToSchema)]
pub struct VideosResponse {
    pub videos: Vec<VideoInfo>,
    pub total_count: u64,
}

#[derive(Serialize, ToSchema)]
pub struct VideoResponse {
    pub video: VideoInfo,
    pub pages: Vec<PageInfo>,
}

#[derive(FromQueryResult, Serialize, ToSchema)]
pub struct VideoSource {
    id: i32,
    name: String,
}

#[derive(DerivePartialModel, FromQueryResult, Serialize, ToSchema)]
#[sea_orm(entity = "page::Entity")]
pub struct PageInfo {
    id: i32,
    pid: i32,
    name: String,
}

#[derive(DerivePartialModel, FromQueryResult, Serialize, ToSchema)]
#[sea_orm(entity = "video::Entity")]
pub struct VideoInfo {
    id: i32,
    name: String,
    upper_name: String,
}
