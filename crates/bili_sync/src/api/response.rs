use sea_orm::FromQueryResult;
use serde::Serialize;
use utoipa::ToSchema;

use crate::utils::status::{PageStatus, VideoStatus};

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
    pub video: i32,
    pub pages: Vec<i32>,
}

#[derive(FromQueryResult, Serialize, ToSchema)]
pub struct VideoSource {
    id: i32,
    name: String,
}

#[derive(Serialize, ToSchema)]
pub struct PageInfo {
    pub id: i32,
    pub pid: i32,
    pub name: String,
    pub download_status: [u32; 5],
}

impl From<(i32, i32, String, u32)> for PageInfo {
    fn from((id, pid, name, download_status): (i32, i32, String, u32)) -> Self {
        Self {
            id,
            pid,
            name,
            download_status: PageStatus::from(download_status).into(),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct VideoInfo {
    pub id: i32,
    pub name: String,
    pub upper_name: String,
    pub download_status: [u32; 5],
}

impl From<(i32, String, String, u32)> for VideoInfo {
    fn from((id, name, upper_name, download_status): (i32, String, String, u32)) -> Self {
        Self {
            id,
            name,
            upper_name,
            download_status: VideoStatus::from(download_status).into(),
        }
    }
}
