pub use analyzer::{BestStream, FilterOption};
use anyhow::{bail, Result};
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
pub use client::{BiliClient, Client};
pub use collection::{CollectionItem, CollectionType};
pub use credential::Credential;
pub use danmaku::DanmakuOption;
pub use error::BiliError;
use favorite_list::Upper;
pub use favorite_list::{FavoriteList, FavoriteListInfo};
pub use video::{Dimension, PageInfo, Video};

mod analyzer;
mod client;
mod collection;
mod credential;
mod danmaku;
mod error;
mod favorite_list;
mod video;

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
pub enum VideoInfo {
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
}
