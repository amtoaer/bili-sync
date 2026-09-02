use bili_sync_entity::rule::Rule;
use chrono::NaiveDateTime;
use serde::{Deserialize, Deserializer, Serialize};
use validator::Validate;

use crate::bilibili::{CollectionType, FilterOption};

/// 反序列化三态 Option：serde derive 对 `Option<Option<T>>` 会把显式 null 折叠成 None，
/// 用该辅助函数保留区分——字段缺失 → None，显式 null → Some(None)，有值 → Some(Some(v))
fn deserialize_double_option<'de, D, T>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusFilter {
    Failed,
    Succeeded,
    Waiting,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationFilter {
    Skipped,
    Invalid,
    Normal,
}

#[derive(Deserialize)]
pub struct VideosRequest {
    pub collection: Option<i32>,
    pub favorite: Option<i32>,
    pub submission: Option<i32>,
    pub watch_later: Option<i32>,
    pub query: Option<String>,
    pub status_filter: Option<StatusFilter>,
    pub validation_filter: Option<ValidationFilter>,
    pub created_from: Option<NaiveDateTime>,
    pub created_to: Option<NaiveDateTime>,
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
    pub status_filter: Option<StatusFilter>,
    pub validation_filter: Option<ValidationFilter>,
    pub created_from: Option<NaiveDateTime>,
    pub created_to: Option<NaiveDateTime>,
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
    pub status_filter: Option<StatusFilter>,
    pub validation_filter: Option<ValidationFilter>,
    pub created_from: Option<NaiveDateTime>,
    pub created_to: Option<NaiveDateTime>,
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
    pub name: Option<String>,
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
    /// 外层 Option 区分字段是否提供（缺失不动，显式 null 清空），内层 Option 为实际值
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub rule: Option<Option<Rule>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub filter_option: Option<Option<FilterOption>>,
    pub use_dynamic_api: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct DefaultPathRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct PollQrcodeRequest {
    pub qrcode_key: String,
}

#[derive(Debug, Deserialize)]
pub struct FullSyncVideoSourceRequest {
    pub delete_local: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证 rule/filter_option 的三态语义：字段缺失 → None（不更新），
    /// 显式 null → Some(None)（清空），有值 → Some(Some(v))（写入）
    #[test]
    fn update_video_source_request_rule_semantics() {
        // 字段缺失 → 不更新
        let req: UpdateVideoSourceRequest = serde_json::from_value(serde_json::json!({
            "path": "/tmp",
            "enabled": true
        }))
        .unwrap();
        assert_eq!(req.rule, None);
        assert!(matches!(req.filter_option, None));

        // 显式 null → 清空
        let req: UpdateVideoSourceRequest = serde_json::from_value(serde_json::json!({
            "path": "/tmp",
            "enabled": true,
            "rule": null,
            "filterOption": null
        }))
        .unwrap();
        assert_eq!(req.rule, Some(None));
        assert!(matches!(req.filter_option, Some(None)));

        // 有值 → 写入
        let req: UpdateVideoSourceRequest = serde_json::from_value(serde_json::json!({
            "path": "/tmp",
            "enabled": true,
            "rule": [[{ "field": "title", "rule": { "operator": "equals", "value": "test" } }]]
        }))
        .unwrap();
        assert!(req.rule.as_ref().is_some_and(|rule| rule.is_some()));
    }
}
