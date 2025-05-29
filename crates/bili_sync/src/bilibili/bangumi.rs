#![allow(unused)]
use anyhow::{Context, Result, anyhow};
use async_stream::try_stream;
use futures::Stream;
use reqwest::Method;
use serde::Deserialize;
use serde_aux::prelude::deserialize_string_from_number;

use crate::bilibili::{BiliClient, Validate, VideoInfo};

pub struct Bangumi<'a> {
    client: &'a BiliClient,
    season_id: String,
}

impl<'a> Bangumi<'a> {
    pub fn new(client: &'a BiliClient, season_id: String) -> Self {
        Self { client, season_id }
    }

    pub async fn get_season_info(&self) -> Result<serde_json::Value> {
        self.client
            .request(
                Method::GET,
                &format!(
                    "https://api.bilibili.com/pgc/view/web/season?season_id={}",
                    self.season_id
                ),
            )
            .await
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()
    }

    async fn get_episodes(&self) -> Result<Vec<VideoInfo>> {
        let mut season_info = self.get_season_info().await?;
        Ok(
            serde_json::from_value::<Vec<VideoInfo>>(season_info["result"]["episodes"].take())?
                .into_iter()
                .enumerate()
                .map(|(idx, mut episode)| match episode {
                    VideoInfo::Bangumi { ref mut ep_num, .. } => {
                        *ep_num = idx as i64 + 1;
                        episode
                    }
                    _ => unreachable!(),
                })
                .collect(),
        )
    }

    pub fn into_video_stream(&'a self) -> impl Stream<Item = Result<VideoInfo>> + 'a {
        try_stream! {
            for episode in self.get_episodes().await? {
                yield episode;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bilibili::BiliClient;

    #[tokio::test]
    async fn test_bangumi() -> Result<()> {
        let client = BiliClient::new();
        let bangumi = Bangumi::new(&client, "39180".to_string());
        let episodes = bangumi.get_episodes().await?;
        for episode in episodes {
            dbg!(episode);
        }
        Ok(())
    }
}
