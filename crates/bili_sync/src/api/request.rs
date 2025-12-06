use bili_sync_entity::rule::Rule;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::bilibili::CollectionType;

#[derive(Deserialize)]
pub struct VideosRequest {
    pub collection: Option<i32>,
    pub favorite: Option<i32>,
    pub submission: Option<i32>,
    pub watch_later: Option<i32>,
    pub query: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Deserialize)]
pub struct ResetVideoStatusRequest {
    #[serde(default)]
    pub force: bool,
}

#[derive(Deserialize)]
pub struct ResetFilteredVideoStatusRequest {
    pub collection: Option<i32>,
    pub favorite: Option<i32>,
    pub submission: Option<i32>,
    pub watch_later: Option<i32>,
    pub query: Option<String>,
    #[serde(default)]
    pub force: bool,
}

#[derive(Deserialize, Validate)]
pub struct StatusUpdate {
    #[validate(range(min = 0, max = 4))]
    pub status_index: usize,
    #[validate(custom(function = "crate::utils::validation::validate_status_value"))]
    pub status_value: u32,
}

#[derive(Deserialize, Validate)]
pub struct PageStatusUpdate {
    pub page_id: i32,
    #[validate(nested)]
    pub updates: Vec<StatusUpdate>,
}

#[derive(Deserialize, Validate)]
pub struct UpdateVideoStatusRequest {
    #[serde(default)]
    #[validate(nested)]
    pub video_updates: Vec<StatusUpdate>,
    #[serde(default)]
    #[validate(nested)]
    pub page_updates: Vec<PageStatusUpdate>,
}

#[derive(Deserialize, Validate)]
pub struct UpdateFilteredVideoStatusRequest {
    pub collection: Option<i32>,
    pub favorite: Option<i32>,
    pub submission: Option<i32>,
    pub watch_later: Option<i32>,
    pub query: Option<String>,
    #[serde(default)]
    #[validate(nested)]
    pub video_updates: Vec<StatusUpdate>,
    #[serde(default)]
    #[validate(nested)]
    pub page_updates: Vec<StatusUpdate>,
}

#[derive(Deserialize)]
pub struct FollowedCollectionsRequest {
    pub page_num: Option<i32>,
    pub page_size: Option<i32>,
}

#[derive(Deserialize)]
pub struct FollowedUppersRequest {
    pub page_num: Option<i32>,
    pub page_size: Option<i32>,
}

#[derive(Deserialize, Validate)]
pub struct InsertFavoriteRequest {
    pub fid: i64,
    #[validate(custom(function = "crate::utils::validation::validate_path"))]
    pub path: String,
}

#[derive(Deserialize, Validate)]
pub struct InsertCollectionRequest {
    pub sid: i64,
    pub mid: i64,
    #[serde(default)]
    pub collection_type: CollectionType,
    #[validate(custom(function = "crate::utils::validation::validate_path"))]
    pub path: String,
}

#[derive(Deserialize, Validate)]
pub struct InsertSubmissionRequest {
    pub upper_id: i64,
    #[validate(custom(function = "crate::utils::validation::validate_path"))]
    pub path: String,
}

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateVideoSourceRequest {
    #[validate(custom(function = "crate::utils::validation::validate_path"))]
    pub path: String,
    pub enabled: bool,
    pub rule: Option<Rule>,
    pub use_dynamic_api: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct DefaultPathRequest {
    pub name: String,
}
