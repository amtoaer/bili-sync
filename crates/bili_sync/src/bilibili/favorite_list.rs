use anyhow::{Context, Result, anyhow};
use async_stream::try_stream;
use futures::Stream;
use serde_json::Value;

use crate::bilibili::{BiliClient, Credential, ErrorForStatusExt, Validate, VideoInfo};
pub struct FavoriteList<'a> {
    client: &'a BiliClient,
    fid: String,
    credential: &'a Credential,
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
    pub fn new(client: &'a BiliClient, fid: String, credential: &'a Credential) -> Self {
        Self {
            client,
            fid,
            credential,
        }
    }

    pub async fn get_info(&self) -> Result<FavoriteListInfo> {
        let mut res = self
            .client
            .request(
                reqwest::Method::GET,
                "https://api.bilibili.com/x/v3/fav/folder/info",
                self.credential,
            )
            .await
            .query(&[("media_id", &self.fid)])
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(res["data"].take())?)
    }

    async fn get_videos(&self, page: u32) -> Result<Value> {
        self.client
            .request(
                reqwest::Method::GET,
                "https://api.bilibili.com/x/v3/fav/resource/list",
                self.credential,
            )
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
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()
    }

    // 拿到收藏夹的所有权，返回一个收藏夹下的视频流
    pub fn into_video_stream(self) -> impl Stream<Item = Result<VideoInfo>> + 'a {
        try_stream! {
            let mut page = 1;
            loop {
                let mut videos = self
                    .get_videos(page)
                    .await
                    .with_context(|| format!("failed to get videos of favorite {} page {}", self.fid, page))?;
                let medias = &mut videos["data"]["medias"];
                if medias.as_array().is_none_or(|v| v.is_empty()) {
                    Err(anyhow!("no medias found in favorite {} page {}", self.fid, page))?;
                }
                let videos_info: Vec<VideoInfo> = serde_json::from_value(medias.take())
                    .with_context(|| format!("failed to parse videos of favorite {} page {}", self.fid, page))?;
                for video_info in videos_info {
                    yield video_info;
                }
                let has_more = &videos["data"]["has_more"];
                if let Some(v) = has_more.as_bool() {
                    if v {
                        page += 1;
                        continue;
                    }
                } else {
                    Err(anyhow!("has_more is not a bool"))?;
                }
                break;
            }
        }
    }
}
