use bili_sync_entity::*;
use sea_orm::{DerivePartialModel, FromQueryResult};
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
    pub video: VideoInfo,
    pub pages: Vec<PageInfo>,
}

#[derive(Serialize, ToSchema)]
pub struct ResetAllVideosResponse {
    pub resetted: bool,
    pub resetted_videos_count: usize,
    pub resetted_pages_count: usize,
}

#[derive(Serialize, ToSchema)]
pub struct UpdateVideoStatusResponse {
    pub success: bool,
    pub video: VideoInfo,
    pub pages: Vec<PageInfo>,
}

#[derive(FromQueryResult, Serialize, ToSchema)]
pub struct VideoSource {
    pub id: i32,
    pub name: String,
}

#[derive(Serialize, ToSchema, DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "video::Entity")]
pub struct VideoInfo {
    pub id: i32,
    pub name: String,
    pub upper_name: String,
    #[schema(value_type = [u32; 5])]
    #[serde(serialize_with = "serde_video_download_status")]
    pub download_status: u32,
}

#[derive(Serialize, ToSchema, DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "page::Entity")]
pub struct PageInfo {
    pub id: i32,
    pub video_id: i32,
    pub pid: i32,
    pub name: String,
    #[schema(value_type = [u32; 5])]
    #[serde(serialize_with = "serde_page_download_status")]
    pub download_status: u32,
}

fn serde_video_download_status<S>(status: &u32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let status: [u32; 5] = VideoStatus::from(*status).into();
    status.serialize(serializer)
}

fn serde_page_download_status<S>(status: &u32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let status: [u32; 5] = PageStatus::from(*status).into();
    status.serialize(serializer)
}

#[derive(Serialize, ToSchema)]
pub struct FavoriteWithSubscriptionStatus {
    pub title: String,
    pub media_count: i64,
    pub fid: i64,
    pub mid: i64,
    pub subscribed: bool,
}

#[derive(Serialize, ToSchema)]
pub struct CollectionWithSubscriptionStatus {
    pub title: String,
    pub sid: i64,
    pub mid: i64,
    pub invalid: bool,
    pub subscribed: bool,
}

#[derive(Serialize, ToSchema)]
pub struct UpperWithSubscriptionStatus {
    pub mid: i64,
    pub uname: String,
    pub face: String,
    pub sign: String,
    pub invalid: bool,
    pub subscribed: bool,
}

#[derive(Serialize, ToSchema)]
pub struct FavoritesResponse {
    pub favorites: Vec<FavoriteWithSubscriptionStatus>,
}

#[derive(Serialize, ToSchema)]
pub struct CollectionsResponse {
    pub collections: Vec<CollectionWithSubscriptionStatus>,
    pub total: i64,
}

#[derive(Serialize, ToSchema)]
pub struct UppersResponse {
    pub uppers: Vec<UpperWithSubscriptionStatus>,
    pub total: i64,
}

#[derive(Serialize, ToSchema)]
pub struct VideoSourcesDetailsResponse {
    pub collections: Vec<VideoSourceDetail>,
    pub favorites: Vec<VideoSourceDetail>,
    pub submissions: Vec<VideoSourceDetail>,
    pub watch_later: Vec<VideoSourceDetail>,
}

#[derive(Serialize, ToSchema, FromQueryResult)]
pub struct VideoSourceDetail {
    pub id: i32,
    pub name: String,
    pub path: String,
    pub enabled: bool,
}
