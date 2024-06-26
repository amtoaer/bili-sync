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
use sea_orm::ActiveValue::Set;
pub use video::{Dimension, PageInfo, Video};

use crate::core::utils::id_time_key;
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
        state: i32,
    },
}

impl VideoInfo {
    pub fn to_model(&self) -> bili_sync_entity::video::ActiveModel {
        match self {
            VideoInfo::Simple {
                bvid,
                cover,
                ctime,
                pubtime,
            } => bili_sync_entity::video::ActiveModel {
                bvid: Set(bvid.clone()),
                cover: Set(cover.clone()),
                ctime: Set(ctime.naive_utc()),
                pubtime: Set(pubtime.naive_utc()),
                ..Default::default()
            },
            VideoInfo::Detail {
                title,
                vtype,
                bvid,
                intro,
                cover,
                upper,
                ctime,
                fav_time,
                pubtime,
                attr,
            } => bili_sync_entity::video::ActiveModel {
                bvid: Set(bvid.clone()),
                name: Set(title.clone()),
                category: Set(*vtype),
                intro: Set(intro.clone()),
                cover: Set(cover.clone()),
                ctime: Set(ctime.naive_utc()),
                pubtime: Set(pubtime.naive_utc()),
                favtime: Set(fav_time.naive_utc()),
                download_status: Set(0),
                valid: Set(*attr == 0),
                tags: Set(None),
                single_page: Set(None),
                upper_id: Set(upper.mid),
                upper_name: Set(upper.name.clone()),
                upper_face: Set(upper.face.clone()),
                ..Default::default()
            },
            VideoInfo::View {
                title,
                bvid,
                intro,
                cover,
                upper,
                ctime,
                pubtime,
                state,
            } => bili_sync_entity::video::ActiveModel {
                bvid: Set(bvid.clone()),
                name: Set(title.clone()),
                category: Set(2), // 视频合集里的内容类型肯定是视频
                intro: Set(intro.clone()),
                cover: Set(cover.clone()),
                ctime: Set(ctime.naive_utc()),
                pubtime: Set(pubtime.naive_utc()),
                download_status: Set(0),
                valid: Set(*state == 0),
                tags: Set(None),
                single_page: Set(None),
                upper_id: Set(upper.mid),
                upper_name: Set(upper.name.clone()),
                upper_face: Set(upper.face.clone()),
                ..Default::default()
            },
        }
    }

    pub fn video_key(&self) -> String {
        match self {
            // 对于合集没有 fav_time，只能用 pubtime 代替
            VideoInfo::Simple { bvid, pubtime, .. } => id_time_key(bvid, pubtime),
            VideoInfo::Detail { bvid, fav_time, .. } => id_time_key(bvid, fav_time),
            // 详情接口返回的数据仅用于填充详情，不会被作为 video_key
            _ => unreachable!(),
        }
    }

    pub fn bvid(&self) -> &str {
        match self {
            VideoInfo::Simple { bvid, .. } => bvid,
            VideoInfo::Detail { bvid, .. } => bvid,
            // 同上
            _ => unreachable!(),
        }
    }
}
