use anyhow::Result;
use arc_swap::access::Access;
use async_stream::stream;
use futures::Stream;
use reqwest::Method;
use serde_json::Value;

use crate::bilibili::credential::encoded_query;
use crate::bilibili::favorite_list::Upper;
use crate::bilibili::{BiliClient, Validate, VideoInfo, MIXIN_KEY};
pub struct Submission<'a> {
    client: &'a BiliClient,
    upper_id: String,
}

impl<'a> Submission<'a> {
    pub fn new(client: &'a BiliClient, upper_id: String) -> Self {
        Self { client, upper_id }
    }

    pub async fn get_info(&self) -> Result<Upper<String>> {
        let mut res = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/web-interface/card")
            .await
            .query(&[("mid", self.upper_id.as_str())])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(res["data"]["card"].take())?)
    }

    async fn get_videos(&self, page: i32) -> Result<Value> {
        self.client
            .request(Method::GET, "https://api.bilibili.com/x/space/wbi/arc/search")
            .await
            .query(&encoded_query(
                vec![
                    ("mid", self.upper_id.clone()),
                    ("order", "pubdate".to_string()),
                    ("order_avoided", "true".to_string()),
                    ("platform", "web".to_string()),
                    ("web_location", "1550101".to_string()),
                    ("pn", page.to_string()),
                    ("ps", "30".to_string()),
                ],
                MIXIN_KEY.load().as_deref(),
            ))
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()
    }

    pub fn into_video_stream(self) -> impl Stream<Item = VideoInfo> + 'a {
        stream! {
            let mut page = 1;
            loop {
                let mut videos = match self.get_videos(page).await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("failed to get videos of upper {} page {}: {}", self.upper_id, page, e);
                        break;
                    }
                };
                let vlist = &mut videos["data"]["list"]["vlist"];
                if vlist.as_array().is_none_or(|v| v.is_empty()) {
                    error!("no medias found in upper {} page {}", self.upper_id, page);
                    break;
                }
                let videos_info: Vec<VideoInfo> = match serde_json::from_value(vlist.take()) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("failed to parse videos of upper {} page {}: {}", self.upper_id, page, e);
                        break;
                    }
                };
                for video_info in videos_info {
                    yield video_info;
                }
                let count = &videos["data"]["page"]["count"];
                if count.as_i64().is_some_and(|v| v > (page * 30) as i64) {
                    page += 1;
                    continue;
                }
                break;
            }
        }
    }
}
