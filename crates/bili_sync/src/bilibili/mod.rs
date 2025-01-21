use std::sync::Arc;

pub use analyzer::{BestStream, FilterOption};
use anyhow::{bail, ensure, Result};
use arc_swap::ArcSwapOption;
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
pub use client::{BiliClient, Client};
pub use collection::{Collection, CollectionItem, CollectionType};
pub use credential::Credential;
pub use danmaku::DanmakuOption;
pub use error::BiliError;
pub use favorite_list::FavoriteList;
use favorite_list::Upper;
use once_cell::sync::Lazy;
pub use submission::Submission;
pub use video::{Dimension, PageInfo, Video};
pub use watch_later::WatchLater;

mod analyzer;
mod client;
mod collection;
mod credential;
mod danmaku;
mod error;
mod favorite_list;
mod submission;
mod video;
mod watch_later;

static MIXIN_KEY: Lazy<ArcSwapOption<String>> = Lazy::new(Default::default);

#[inline]
pub(crate) fn set_global_mixin_key(key: String) {
    MIXIN_KEY.store(Some(Arc::new(key)));
}

pub(crate) trait Validate {
    type Output;

    fn validate(self) -> Result<Self::Output>;
}

impl Validate for serde_json::Value {
    type Output = serde_json::Value;

    fn validate(self) -> Result<Self::Output> {
        let (code, msg) = match (self["code"].as_i64(), self["message"].as_str()) {
            (Some(code), Some(msg)) => (code, msg),
            _ => bail!("no code or message found"),
        };
        ensure!(code == 0, BiliError::RequestFailed(code, msg.to_owned()));
        Ok(self)
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
/// 注意此处的顺序是有要求的，因为对于 untagged 的 enum 来说，serde 会按照顺序匹配
/// > There is no explicit tag identifying which variant the data contains.
/// > Serde will try to match the data against each variant in order and the first one that deserializes successfully is the one returned.
pub enum VideoInfo {
    /// 从视频详情接口获取的视频信息
    View {
        title: String,
        bvid: String,
        #[serde(rename = "desc")]
        intro: String,
        #[serde(rename = "pic")]
        cover: String,
        #[serde(rename = "owner")]
        upper: Upper<i64>,
        #[serde(with = "ts_seconds")]
        ctime: DateTime<Utc>,
        #[serde(rename = "pubdate", with = "ts_seconds")]
        pubtime: DateTime<Utc>,
        pages: Vec<PageInfo>,
        state: i32,
    },
    /// 从收藏夹中获取的视频信息
    Detail {
        title: String,
        #[serde(rename = "type")]
        vtype: i32,
        bvid: String,
        intro: String,
        cover: String,
        upper: Upper<i64>,
        #[serde(with = "ts_seconds")]
        ctime: DateTime<Utc>,
        #[serde(with = "ts_seconds")]
        fav_time: DateTime<Utc>,
        #[serde(with = "ts_seconds")]
        pubtime: DateTime<Utc>,
        attr: i32,
    },
    /// 从稍后再看中获取的视频信息
    WatchLater {
        title: String,
        bvid: String,
        #[serde(rename = "desc")]
        intro: String,
        #[serde(rename = "pic")]
        cover: String,
        #[serde(rename = "owner")]
        upper: Upper<i64>,
        #[serde(with = "ts_seconds")]
        ctime: DateTime<Utc>,
        #[serde(rename = "add_at", with = "ts_seconds")]
        fav_time: DateTime<Utc>,
        #[serde(rename = "pubdate", with = "ts_seconds")]
        pubtime: DateTime<Utc>,
        state: i32,
    },
    /// 从视频列表中获取的视频信息
    Simple {
        bvid: String,
        #[serde(rename = "pic")]
        cover: String,
        #[serde(with = "ts_seconds")]
        ctime: DateTime<Utc>,
        #[serde(rename = "pubdate", with = "ts_seconds")]
        pubtime: DateTime<Utc>,
    },
    Submission {
        title: String,
        bvid: String,
        #[serde(rename = "description")]
        intro: String,
        #[serde(rename = "pic")]
        cover: String,
        #[serde(rename = "created", with = "ts_seconds")]
        ctime: DateTime<Utc>,
    },
}

#[cfg(test)]
mod tests {
    use futures::{pin_mut, StreamExt};

    use super::*;
    use crate::utils::init_logger;

    #[ignore = "only for manual test"]
    #[tokio::test]
    async fn test_video_info_type() {
        init_logger("None,bili_sync=debug");
        let bili_client = BiliClient::new();
        // 请求 UP 主视频必须要获取 mixin key，使用 key 计算请求参数的签名，否则直接提示权限不足返回空
        let Ok(Some(mixin_key)) = bili_client.wbi_img().await.map(|wbi_img| wbi_img.into()) else {
            panic!("获取 mixin key 失败");
        };
        set_global_mixin_key(mixin_key);
        let video = Video::new(&bili_client, "BV1Z54y1C7ZB".to_string());
        assert!(matches!(video.get_view_info().await, Ok(VideoInfo::View { .. })));
        let collection_item = CollectionItem {
            mid: "521722088".to_string(),
            sid: "387214".to_string(),
            collection_type: CollectionType::Series,
        };
        let collection = Collection::new(&bili_client, &collection_item);
        let stream = collection.into_simple_video_stream();
        pin_mut!(stream);
        assert!(matches!(stream.next().await, Some(VideoInfo::Simple { .. })));
        let favorite = FavoriteList::new(&bili_client, "3084505258".to_string());
        let stream = favorite.into_video_stream();
        pin_mut!(stream);
        assert!(matches!(stream.next().await, Some(VideoInfo::Detail { .. })));
        let watch_later = WatchLater::new(&bili_client);
        let stream = watch_later.into_video_stream();
        pin_mut!(stream);
        assert!(matches!(stream.next().await, Some(VideoInfo::WatchLater { .. })));
        let submission = Submission::new(&bili_client, "956761".to_string());
        let stream = submission.into_video_stream();
        pin_mut!(stream);
        assert!(matches!(stream.next().await, Some(VideoInfo::Submission { .. })));
    }
}
