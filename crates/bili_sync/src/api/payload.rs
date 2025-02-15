use bili_sync_entity::*;
use serde::Serialize;

use crate::utils::status::{PageStatus, VideoStatus};

#[derive(Debug, Serialize)]
pub struct VideoListModelItem {
    id: i32,
    name: String,
}

impl From<collection::Model> for VideoListModelItem {
    fn from(value: collection::Model) -> Self {
        VideoListModelItem {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<favorite::Model> for VideoListModelItem {
    fn from(value: favorite::Model) -> Self {
        VideoListModelItem {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<submission::Model> for VideoListModelItem {
    fn from(value: submission::Model) -> Self {
        VideoListModelItem {
            id: value.id,
            name: value.upper_name,
        }
    }
}

impl From<watch_later::Model> for VideoListModelItem {
    fn from(value: watch_later::Model) -> Self {
        VideoListModelItem {
            id: value.id,
            name: "稍后再看".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct VideoListModel {
    pub collection: Vec<VideoListModelItem>,
    pub favorite: Vec<VideoListModelItem>,
    pub submission: Vec<VideoListModelItem>,
    pub watch_later: Vec<VideoListModelItem>,
}

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
