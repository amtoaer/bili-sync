pub use analyzer::{BestStream, FilterOption};
use anyhow::{bail, Result};
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
pub use client::{BiliClient, Client};
pub use collection::{Collection, CollectionItem, CollectionType};
pub use credential::{get_mixin_key, Credential};
pub use danmaku::DanmakuOption;
pub use error::BiliError;
pub use favorite_list::FavoriteList;
use favorite_list::Upper;
pub use video::{Dimension, PageInfo, Video};
pub use watch_later::WatchLater;

mod analyzer;
mod client;
mod collection;
mod credential;
mod danmaku;
mod error;
mod favorite_list;
mod video;
mod watch_later;

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
        if code != 0 {
            bail!(BiliError::RequestFailed(code, msg.to_owned()));
        }
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
        upper: Upper,
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
        upper: Upper,
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
        upper: Upper,
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
}

#[cfg(test)]
mod tests {
    use futures::{pin_mut, StreamExt};

    use super::*;

    #[ignore = "only for manual test"]
    #[tokio::test]
    async fn assert_video_info_type() {
        let bili_client = BiliClient::new();
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
    }
}
