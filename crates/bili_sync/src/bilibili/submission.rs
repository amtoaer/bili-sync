use anyhow::{Context, Result, anyhow};
use async_stream::try_stream;
use futures::Stream;
use reqwest::Method;
use serde_json::Value;

use crate::bilibili::favorite_list::Upper;
use crate::bilibili::{BiliClient, Credential, Dynamic, ErrorForStatusExt, MIXIN_KEY, Validate, VideoInfo, WbiSign};
pub struct Submission<'a> {
    client: &'a BiliClient,
    pub upper_id: String,
    credential: &'a Credential,
}

impl<'a> From<Submission<'a>> for Dynamic<'a> {
    fn from(submission: Submission<'a>) -> Self {
        Dynamic::new(submission.client, submission.upper_id, submission.credential)
    }
}

impl<'a> Submission<'a> {
    pub fn new(client: &'a BiliClient, upper_id: String, credential: &'a Credential) -> Self {
        Self {
            client,
            upper_id,
            credential,
        }
    }

    pub async fn get_info(&self) -> Result<Upper<String>> {
        let mut res = self
            .client
            .request(
                Method::GET,
                "https://api.bilibili.com/x/web-interface/card",
                self.credential,
            )
            .await
            .query(&[("mid", self.upper_id.as_str())])
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(res["data"]["card"].take())?)
    }

    async fn get_videos(&self, page: i32) -> Result<Value> {
        self.client
            .request(
                Method::GET,
                "https://api.bilibili.com/x/space/wbi/arc/search",
                self.credential,
            )
            .await
            .query(&[
                ("mid", self.upper_id.as_str()),
                ("order", "pubdate"),
                ("order_avoided", "true"),
                ("platform", "web"),
                ("web_location", "1550101"),
                ("ps", "30"),
            ])
            .query(&[("pn", page)])
            .wbi_sign(MIXIN_KEY.load().as_deref())?
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()
    }

    pub fn into_video_stream(self) -> impl Stream<Item = Result<VideoInfo>> + 'a {
        try_stream! {
            let mut page = 1;
            loop {
                let mut videos = self
                    .get_videos(page)
                    .await
                    .with_context(|| format!("failed to get videos of upper {} page {}", self.upper_id, page))?;
                let vlist = &mut videos["data"]["list"]["vlist"];
                if vlist.as_array().is_none_or(|v| v.is_empty()) {
                    Err(anyhow!("no medias found in upper {} page {}", self.upper_id, page))?;
                }
                let videos_info: Vec<VideoInfo> = serde_json::from_value(vlist.take())
                    .with_context(|| format!("failed to parse videos of upper {} page {}", self.upper_id, page))?;
                for video_info in videos_info {
                    yield video_info;
                }
                let count = &videos["data"]["page"]["count"];
                if let Some(v) = count.as_i64() {
                    if v > (page * 30) as i64 {
                        page += 1;
                        continue;
                    }
                } else {
                    Err(anyhow!("count is not an i64"))?;
                }
                break;
            }
        }
    }
}
