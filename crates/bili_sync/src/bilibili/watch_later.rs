use anyhow::{Context, Result, anyhow};
use async_stream::try_stream;
use futures::Stream;
use serde_json::Value;

use crate::bilibili::{BiliClient, Validate, VideoInfo};
pub struct WatchLater<'a> {
    client: &'a BiliClient,
}

impl<'a> WatchLater<'a> {
    pub fn new(client: &'a BiliClient) -> Self {
        Self { client }
    }

    async fn get_videos(&self) -> Result<Value> {
        self.client
            .request(reqwest::Method::GET, "https://api.bilibili.com/x/v2/history/toview")
            .await
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()
    }

    pub fn into_video_stream(self) -> impl Stream<Item = Result<VideoInfo>> + 'a {
        try_stream! {
            let mut videos = self
                .get_videos()
                .await
                .with_context(|| "Failed to get watch later list")?;
            let list = &mut videos["data"]["list"];
            if list.as_array().is_none_or(|v| v.is_empty()) {
                Err(anyhow!("No videos found in watch later list"))?;
            }
            let videos_info: Vec<VideoInfo> =
                serde_json::from_value(list.take()).with_context(|| "Failed to parse watch later list")?;
            for video_info in videos_info {
                yield video_info;
            }
        }
    }
}
