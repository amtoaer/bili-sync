use anyhow::{bail, Result};
use async_stream::stream;
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use futures::Stream;
use serde_json::Value;

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
            .json::<serde_json::Value>()
            .await?;
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
            .json::<serde_json::Value>()
            .await?;
        if res["code"] != 0 {
            bail!("get favorite videos failed: {}", res["message"]);
        }
        Ok(res)
    }

    // 拿到收藏夹的所有权，返回一个收藏夹下的视频流
    pub fn into_video_stream(self) -> impl Stream<Item = VideoInfo> + 'a {
        stream! {
            let mut page = 1;
            loop {
                let Ok(mut videos) = self.get_videos(page).await else{
                    break;
                };
                let Ok(videos_info) = serde_json::from_value::<Vec<VideoInfo>>(videos["data"]["medias"].take()) else{
                    break;
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
