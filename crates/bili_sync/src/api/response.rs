use sea_orm::FromQueryResult;
use serde::Serialize;
use utoipa::ToSchema;

use crate::utils::status::{PageStatus, VideoStatus};

use bili_sync_entity::source_collection;
use bili_sync_entity::source_favorite;

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

#[derive(Serialize, ToSchema)]
pub struct SourceCollectionResp {
    pub id: i32,
    pub s_id: i64,
    pub m_id: i64,
    pub r#type: i16,
    pub path: String,
    pub description: String,
    pub enabled: i32,
    pub created_at: String,
}

impl From<source_collection::Model> for SourceCollectionResp {
    fn from(model: source_collection::Model) -> Self {
        SourceCollectionResp {
            // Map the fields from `source_collection::Model` to `SourceCollectionsResponse`
            id: model.id,
            s_id: model.s_id,
            m_id: model.m_id,
            r#type: model.r#type,
            path: model.path,
            description: model.description,
            enabled: model.enabled,
            created_at: model.created_at.to_string(),
        }
    }
}


#[derive(Debug, Serialize, ToSchema)]
pub struct SourceFavoriteResp {
    pub id: i32,
    pub f_id: i64,
    pub path: String,
    pub description: String,
    pub enabled: i32,
    pub created_at: String,
}

impl From<source_favorite::Model> for SourceFavoriteResp {
    fn from(model: source_favorite::Model) -> Self {
        SourceFavoriteResp {
            id: model.id,
            f_id: model.f_id,
            path: model.path,
            description: model.description,
            enabled: model.enabled,
            created_at: model.created_at.to_string(),
        }
    }
}

