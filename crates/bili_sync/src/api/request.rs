use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

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

#[derive(Deserialize, ToSchema)]
pub struct StatusUpdate {
    /// 状态位索引 (0-4)
    pub status_index: usize,
    /// 新的状态值 (0-7)
    pub status_value: u32,
}

#[derive(Deserialize, ToSchema)]
pub struct PageStatusUpdate {
    /// 页面ID
    pub page_id: i32,
    /// 状态更新列表
    pub updates: Vec<StatusUpdate>,
}

#[derive(Deserialize, ToSchema)]
pub struct ResetVideoStatusRequest {
    /// 视频状态更新列表
    pub video_updates: Vec<StatusUpdate>,
    /// 页面状态更新列表
    pub page_updates: Vec<PageStatusUpdate>,
}
