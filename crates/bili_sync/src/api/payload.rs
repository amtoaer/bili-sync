use bili_sync_entity::*;
use serde::{Deserialize, Serialize};

use crate::utils::status::{PageStatus, VideoStatus};

#[derive(Debug, Serialize)]
pub struct VideoInfo {
    id: i32,
    name: String,
    upper_name: String,
    download_status: [u32; 5],
}

impl From<video::Model> for VideoInfo {
    fn from(value: video::Model) -> Self {
        VideoInfo {
            id: value.id,
            name: value.name,
            upper_name: value.upper_name,
            download_status: VideoStatus::from(value.download_status).into(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PageInfo {
    id: i32,
    pid: i32,
    name: String,
    download_status: [u32; 5],
}

impl From<page::Model> for PageInfo {
    fn from(value: page::Model) -> Self {
        PageInfo {
            id: value.id,
            pid: value.pid,
            name: value.name,
            download_status: PageStatus::from(value.download_status).into(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct VideoDetail {
    video: VideoInfo,
    pages: Vec<PageInfo>,
}

impl From<(video::Model, Vec<page::Model>)> for VideoDetail {
    fn from(value: (video::Model, Vec<page::Model>)) -> Self {
        VideoDetail {
            video: VideoInfo::from(value.0),
            pages: value.1.into_iter().map(PageInfo::from).collect(),
        }
    }
}

/// 用于更新单个视频状态的请求结构
#[derive(Debug, Deserialize)]
pub struct UpdateVideoPayload {
    pub download_status: [u32; 5],
}

#[derive(Debug, Deserialize)]
pub struct BulkUpdatePayload {
    pub video_ids: Vec<i32>,
    pub download_status: [u32; 5],
}
