use anyhow::Result;
use async_stream::stream;
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
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()
    }

    pub fn into_video_stream(self) -> impl Stream<Item = VideoInfo> + 'a {
        stream! {
            let Ok(mut videos) = self.get_videos().await else {
                error!("Failed to get watch later list");
                return;
            };
            if !videos["data"]["list"].is_array() {
                error!("Watch later list is not an array");
            }
            let videos_info = match serde_json::from_value::<Vec<VideoInfo>>(videos["data"]["list"].take()) {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to parse watch later list: {}", e);
                    return;
                }
            };
            for video in videos_info {
                yield video;
            }
        }
    }
}
