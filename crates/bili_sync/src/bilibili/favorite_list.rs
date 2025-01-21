use anyhow::Result;
use async_stream::stream;
use futures::Stream;
use serde_json::Value;

use crate::bilibili::{BiliClient, Validate, VideoInfo};
pub struct FavoriteList<'a> {
    client: &'a BiliClient,
    fid: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct FavoriteListInfo {
    pub id: i64,
    pub title: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Upper<T> {
    pub mid: T,
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
            .await
            .query(&[("media_id", &self.fid)])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(res["data"].take())?)
    }

    async fn get_videos(&self, page: u32) -> Result<Value> {
        self.client
            .request(reqwest::Method::GET, "https://api.bilibili.com/x/v3/fav/resource/list")
            .await
            .query(&[
                ("media_id", self.fid.as_str()),
                ("pn", page.to_string().as_str()),
                ("ps", "20"),
                ("order", "mtime"),
                ("type", "0"),
                ("tid", "0"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()
    }

    // 拿到收藏夹的所有权，返回一个收藏夹下的视频流
    pub fn into_video_stream(self) -> impl Stream<Item = VideoInfo> + 'a {
        stream! {
            let mut page = 1;
            loop {
                let mut videos = match self.get_videos(page).await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("failed to get videos of favorite {} page {}: {}", self.fid, page, e);
                        break;
                    }
                };
                let medias = &mut videos["data"]["medias"];
                if medias.as_array().is_none_or(|v| v.is_empty()) {
                    error!("no medias found in favorite {} page {}", self.fid, page);
                    break;
                }
                let videos_info: Vec<VideoInfo> = match serde_json::from_value(medias.take()) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("failed to parse videos of favorite {} page {}: {}", self.fid, page, e);
                        break;
                    }
                };
                for video_info in videos_info {
                    yield video_info;
                }
                let has_more = &videos["data"]["has_more"];
                if has_more.as_bool().is_some_and(|v| v) {
                    page += 1;
                    continue;
                }
                break;
            }
        }
    }
}
