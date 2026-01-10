use bili_sync_entity::rule::Rule;
use bili_sync_entity::*;
use sea_orm::{DerivePartialModel, FromQueryResult};
use serde::Serialize;

use crate::{
    bilibili::{PollStatus, Qrcode},
    utils::status::{PageStatus, VideoStatus},
};

#[derive(Serialize)]
pub struct VideoSourcesResponse {
    pub collection: Vec<VideoSource>,
    pub favorite: Vec<VideoSource>,
    pub submission: Vec<VideoSource>,
    pub watch_later: Vec<VideoSource>,
}

#[derive(Serialize)]
pub struct VideosResponse {
    pub videos: Vec<VideoInfo>,
    pub total_count: u64,
}

#[derive(Serialize)]
pub struct VideoResponse {
    pub video: VideoInfo,
    pub pages: Vec<PageInfo>,
}

#[derive(Serialize)]
pub struct ResetVideoResponse {
    pub resetted: bool,
    pub video: VideoInfo,
    pub pages: Vec<PageInfo>,
}

#[derive(Serialize)]
pub struct ResetFilteredVideosResponse {
    pub resetted: bool,
    pub resetted_videos_count: usize,
    pub resetted_pages_count: usize,
}

#[derive(Serialize)]
pub struct UpdateVideoStatusResponse {
    pub success: bool,
    pub video: VideoInfo,
    pub pages: Vec<PageInfo>,
}

#[derive(Serialize)]
pub struct UpdateFilteredVideoStatusResponse {
    pub success: bool,
    pub updated_videos_count: usize,
    pub updated_pages_count: usize,
}

#[derive(FromQueryResult, Serialize)]
pub struct VideoSource {
    pub id: i32,
    pub name: String,
}

#[derive(Serialize, DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "video::Entity")]
pub struct VideoInfo {
    pub id: i32,
    pub bvid: String,
    pub name: String,
    pub upper_name: String,
    pub should_download: bool,
    #[serde(serialize_with = "serde_video_download_status")]
    pub download_status: u32,
}

#[derive(Serialize, DerivePartialModel, FromQueryResult)]
#[sea_orm(entity = "page::Entity")]
pub struct PageInfo {
    pub id: i32,
    pub video_id: i32,
    pub pid: i32,
    pub name: String,
    #[serde(serialize_with = "serde_page_download_status")]
    pub download_status: u32,
}

#[derive(Serialize, DerivePartialModel, FromQueryResult, Clone, Copy)]
#[sea_orm(entity = "video::Entity")]
pub struct SimpleVideoInfo {
    pub id: i32,
    pub download_status: u32,
}

#[derive(Serialize, DerivePartialModel, FromQueryResult, Clone, Copy)]
#[sea_orm(entity = "page::Entity")]
pub struct SimplePageInfo {
    pub id: i32,
    pub video_id: i32,
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

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Followed {
    Favorite {
        title: String,
        media_count: i64,
        fid: i64,
        mid: i64,
        invalid: bool,
        subscribed: bool,
    },
    Collection {
        title: String,
        sid: i64,
        mid: i64,
        media_count: i64,
        invalid: bool,
        subscribed: bool,
    },
    Upper {
        mid: i64,
        uname: String,
        face: String,
        sign: String,
        invalid: bool,
        subscribed: bool,
    },
}

#[derive(Serialize)]
pub struct FavoritesResponse {
    pub favorites: Vec<Followed>,
}

#[derive(Serialize)]
pub struct CollectionsResponse {
    pub collections: Vec<Followed>,
    pub total: i64,
}

#[derive(Serialize)]
pub struct UppersResponse {
    pub uppers: Vec<Followed>,
    pub total: i64,
}

#[derive(Serialize)]
pub struct VideoSourcesDetailsResponse {
    pub collections: Vec<VideoSourceDetail>,
    pub favorites: Vec<VideoSourceDetail>,
    pub submissions: Vec<VideoSourceDetail>,
    pub watch_later: Vec<VideoSourceDetail>,
}

#[derive(Serialize, FromQueryResult)]
pub struct DayCountPair {
    pub day: String,
    pub cnt: i64,
}

#[derive(Serialize)]
pub struct DashBoardResponse {
    pub enabled_favorites: u64,
    pub enabled_collections: u64,
    pub enabled_submissions: u64,
    pub enable_watch_later: bool,
    pub videos_by_day: Vec<DayCountPair>,
}

#[derive(Serialize, Clone, Copy)]
pub struct SysInfo {
    pub timestamp: i64,
    pub total_memory: u64,
    pub used_memory: u64,
    pub process_memory: u64,
    pub used_cpu: f32,
    pub process_cpu: f32,
    pub total_disk: u64,
    pub available_disk: u64,
}

#[derive(Serialize, FromQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct VideoSourceDetail {
    pub id: i32,
    pub name: String,
    pub path: String,
    pub rule: Option<Rule>,
    #[serde(default)]
    pub rule_display: Option<String>,
    #[serde(default)]
    pub use_dynamic_api: Option<bool>,
    pub enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateVideoSourceResponse {
    pub rule_display: Option<String>,
}

pub type GenerateQrcodeResponse = Qrcode;

pub type PollQrcodeResponse = PollStatus;
