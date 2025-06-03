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

#[derive(Serialize, ToSchema)]
pub struct ResetVideoResponse {
    pub resetted: bool,
    pub video: VideoInfo,
    pub pages: Vec<PageInfo>,
}

#[derive(Serialize, ToSchema)]
pub struct ResetAllVideosResponse {
    pub resetted: bool,
    pub resetted_videos_count: usize,
    pub resetted_pages_count: usize,
}

#[derive(FromQueryResult, Serialize, ToSchema)]
pub struct VideoSource {
    id: i32,
    name: String,
}

#[derive(Serialize, ToSchema, DerivePartialModel, FromQueryResult, Clone)]
#[sea_orm(entity = "video::Entity")]
pub struct VideoInfo {
    pub id: i32,
    pub name: String,
    pub upper_name: String,
    pub download_status: u32,
}

#[derive(Serialize, ToSchema, DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "page::Entity")]
pub struct PageInfo {
    pub id: i32,
    pub video_id: i32,
    pub pid: i32,
    pub name: String,
    pub download_status: u32,
}
