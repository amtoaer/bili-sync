use anyhow::{Context, Result, anyhow};
use async_stream::try_stream;
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use futures::Stream;
use reqwest::Method;
use serde_json::Value;

use crate::bilibili::{BiliClient, MIXIN_KEY, Validate, VideoInfo, WbiSign};

pub struct Dynamic<'a> {
    client: &'a BiliClient,
    pub upper_id: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct DynamicItemPublished {
    #[serde(with = "ts_seconds")]
    pub_ts: DateTime<Utc>,
}

impl<'a> Dynamic<'a> {
    pub fn new(client: &'a BiliClient, upper_id: String) -> Self {
        Self { client, upper_id }
    }

    pub async fn get_dynamics(&self, offset: Option<String>) -> Result<Value> {
        self.client
            .request(
                Method::GET,
                "https://api.bilibili.com/x/polymer/web-dynamic/v1/feed/space",
            )
            .await
            .query(&[
                ("host_mid", self.upper_id.as_str()),
                ("offset", offset.as_deref().unwrap_or("")),
                ("type", "video"),
            ])
            .wbi_sign(MIXIN_KEY.load().as_deref())?
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()
    }

    pub fn into_video_stream(self) -> impl Stream<Item = Result<VideoInfo>> + 'a {
        try_stream! {
            let mut offset = None;
            loop {
                let mut res = self
                    .get_dynamics(offset.take())
                    .await
                    .with_context(|| "failed to get dynamics")?;
                let items = res["data"]["items"].as_array_mut().context("items not exist")?;
                for item in items.iter_mut() {
                    if item["type"].as_str().is_none_or(|t| t != "DYNAMIC_TYPE_AV") {
                        continue;
                    }
                    let published: DynamicItemPublished = serde_json::from_value(item["modules"]["module_author"].take())
                        .with_context(|| "failed to parse published time")?;
                    let mut video_info: VideoInfo =
                        serde_json::from_value(item["modules"]["module_dynamic"]["major"]["archive"].take())?;
                    // 这些地方不使用 let else 是因为 try_stream! 宏不支持
                    if let VideoInfo::Dynamic { ref mut pubtime, .. } = video_info {
                        *pubtime = published.pub_ts;
                        yield video_info;
                    } else {
                        Err(anyhow!("video info is not dynamic"))?;
                    }
                }
                if let (Some(has_more), Some(new_offset)) =
                    (res["data"]["has_more"].as_bool(), res["data"]["offset"].as_str())
                {
                    if !has_more {
                        break;
                    }
                    offset = Some(new_offset.to_string());
                } else {
                    Err(anyhow!("no has_more or offset found"))?;
                }
            }
        }
    }
}
