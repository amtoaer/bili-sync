use bili_sync_entity::*;
use sea_orm::{DerivePartialModel, FromQueryResult};
use serde::Serialize;

#[derive(FromQueryResult, Serialize)]
pub struct VideoListModelItem {
    id: i32,
    name: String,
}

#[derive(Serialize)]
pub struct VideoListModel {
    pub collection: Vec<VideoListModelItem>,
    pub favorite: Vec<VideoListModelItem>,
    pub submission: Vec<VideoListModelItem>,
    pub watch_later: Vec<VideoListModelItem>,
}

#[derive(DerivePartialModel, FromQueryResult, Serialize)]
#[sea_orm(entity = "video::Entity")]
pub struct VideoInfo {
    id: i32,
    name: String,
    upper_name: String,
}

#[derive(Serialize)]
pub struct VideoList {
    pub videos: Vec<VideoInfo>,
    pub total_count: u64,
}

#[derive(DerivePartialModel, FromQueryResult, Serialize)]
#[sea_orm(entity = "page::Entity")]
pub struct PageInfo {
    id: i32,
    pid: i32,
    name: String,
}

#[derive(Serialize)]
pub struct VideoDetail {
    pub video: VideoInfo,
    pub pages: Vec<PageInfo>,
}
