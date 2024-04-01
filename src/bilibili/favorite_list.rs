use anyhow::{bail, Result};
use async_stream::stream;
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use futures::Stream;
use log::error;
use serde_json::Value;

use crate::bilibili::error::BiliError;
use crate::bilibili::BiliClient;
pub struct FavoriteList<'a> {
    client: &'a BiliClient,
    fid: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct FavoriteListInfo {
    pub id: i32,
    pub title: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct VideoInfo {
    pub title: String,
    #[serde(rename = "type")]
    pub vtype: i32,
    pub bvid: String,
    pub intro: String,
    pub cover: String,
    pub upper: Upper,
    #[serde(with = "ts_seconds")]
    pub ctime: DateTime<Utc>,
    #[serde(with = "ts_seconds")]
    pub fav_time: DateTime<Utc>,
    #[serde(with = "ts_seconds")]
    pub pubtime: DateTime<Utc>,
    pub attr: i32,
}

#[derive(Debug, serde::Deserialize)]
pub struct Upper {
    pub mid: i32,
    pub name: String,
    pub face: String,
}
impl<'a> FavoriteList<'a> {
    pub fn new(client: &'a BiliClient, fid: String) -> Self {
        Self { client, fid }
    }

    pub async fn get_info(&self) -> Result<FavoriteListInfo> {
        let mut res = self
            .client
            .request(reqwest::Method::GET, "https://api.bilibili.com/x/v3/fav/folder/info")
            .query(&[("media_id", &self.fid)])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;
        let (code, msg) = match (res["code"].as_u64(), res["message"].as_str()) {
            (Some(code), Some(msg)) => (code, msg),
            _ => bail!("no code or message found"),
        };
        if code != 0 {
            bail!(BiliError::RequestFailed(code, msg.to_owned()));
        }
        Ok(serde_json::from_value(res["data"].take())?)
    }

    async fn get_videos(&self, page: u32) -> Result<Value> {
        let res = self
            .client
            .request(reqwest::Method::GET, "https://api.bilibili.com/x/v3/fav/resource/list")
            .query(&[
                ("media_id", self.fid.as_str()),
                ("pn", &page.to_string()),
                ("ps", "20"),
                ("order", "mtime"),
                ("type", "0"),
                ("tid", "0"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;
        let (code, msg) = match (res["code"].as_u64(), res["message"].as_str()) {
            (Some(code), Some(msg)) => (code, msg),
            _ => bail!("no code or message found"),
        };
        if code != 0 {
            bail!(BiliError::RequestFailed(code, msg.to_owned()));
        }
        Ok(res)
    }

    // 拿到收藏夹的所有权，返回一个收藏夹下的视频流
    pub fn into_video_stream(self) -> impl Stream<Item = VideoInfo> + 'a {
        stream! {
            let mut page = 1;
            loop {
                let mut videos = match self.get_videos(page).await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("failed to get videos of page {}: {}", page, e);
                        break;
                    },
                };
                let videos_info = match serde_json::from_value::<Vec<VideoInfo>>(videos["data"]["medias"].take()) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("failed to parse videos of page {}: {}", page, e);
                        break;
                    },
                };
                for video_info in videos_info.into_iter(){
                    yield video_info;
                }
                if videos["data"]["has_more"].is_boolean() && videos["data"]["has_more"].as_bool().unwrap(){
                    page += 1;
                    continue;
                }
                break;
            }
        }
    }
}
