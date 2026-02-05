use anyhow::{Context, Result, ensure};
use futures::TryStreamExt;
use futures::stream::FuturesUnordered;
use prost::Message;
use reqwest::Method;

use crate::bilibili::analyzer::PageAnalyzer;
use crate::bilibili::client::BiliClient;
use crate::bilibili::danmaku::{DanmakuElem, DanmakuWriter, DmSegMobileReply};
use crate::bilibili::subtitle::{SubTitle, SubTitleBody, SubTitleInfo, SubTitlesInfo};
use crate::bilibili::{Credential, ErrorForStatusExt, MIXIN_KEY, Validate, VideoInfo, WbiSign};

pub struct Video<'a> {
    client: &'a BiliClient,
    pub bvid: String,
    credential: &'a Credential,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct PageInfo {
    pub cid: i64,
    pub page: i32,
    #[serde(rename = "part")]
    pub name: String,
    pub duration: u32,
    pub first_frame: Option<String>,
    pub dimension: Option<Dimension>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct Dimension {
    pub width: u32,
    pub height: u32,
    pub rotate: u32,
}

impl<'a> Video<'a> {
    pub fn new(client: &'a BiliClient, bvid: String, credential: &'a Credential) -> Self {
        Self {
            client,
            bvid,
            credential,
        }
    }

    /// 直接调用视频信息接口获取详细的视频信息，视频信息中包含了视频的分页信息
    pub async fn get_view_info(&self) -> Result<VideoInfo> {
        let mut res = self
            .client
            .request(
                Method::GET,
                "https://api.bilibili.com/x/web-interface/wbi/view",
                self.credential,
            )
            .await
            .query(&[("bvid", &self.bvid)])
            .wbi_sign(MIXIN_KEY.load().as_deref())?
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(res["data"].take())?)
    }

    #[cfg(test)]
    pub async fn get_pages(&self) -> Result<Vec<PageInfo>> {
        let mut res = self
            .client
            .request(
                Method::GET,
                "https://api.bilibili.com/x/player/pagelist",
                self.credential,
            )
            .await
            .query(&[("bvid", &self.bvid)])
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(res["data"].take())?)
    }

    pub async fn get_tags(&self) -> Result<Vec<String>> {
        let res = self
            .client
            .request(
                Method::GET,
                "https://api.bilibili.com/x/web-interface/view/detail/tag",
                self.credential,
            )
            .await
            .query(&[("bvid", &self.bvid)])
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(res["data"]
            .as_array()
            .context("tags is not an array")?
            .iter()
            .filter_map(|v| v["tag_name"].as_str().map(String::from))
            .collect())
    }

    pub async fn get_danmaku_writer(&self, page: &'a PageInfo) -> Result<DanmakuWriter<'a>> {
        let tasks = FuturesUnordered::new();
        for i in 1..=page.duration.div_ceil(360) {
            tasks.push(self.get_danmaku_segment(page, i as i64));
        }
        let result: Vec<Vec<DanmakuElem>> = tasks.try_collect().await?;
        let mut result: Vec<DanmakuElem> = result.into_iter().flatten().collect();
        result.sort_by_key(|d| d.progress);
        Ok(DanmakuWriter::new(page, result.into_iter().map(|x| x.into()).collect()))
    }

    async fn get_danmaku_segment(&self, page: &PageInfo, segment_idx: i64) -> Result<Vec<DanmakuElem>> {
        let mut res = self
            .client
            .request(
                Method::GET,
                "https://api.bilibili.com/x/v2/dm/wbi/web/seg.so",
                self.credential,
            )
            .await
            .query(&[("type", 1), ("oid", page.cid), ("segment_index", segment_idx)])
            .wbi_sign(MIXIN_KEY.load().as_deref())?
            .send()
            .await?
            .error_for_status_ext()?;
        let headers = std::mem::take(res.headers_mut());
        let content_type = headers.get("content-type");
        ensure!(
            content_type.is_some_and(|v| v == "application/octet-stream"),
            "unexpected content type: {:?}, body: {:?}",
            content_type,
            res.text().await
        );
        Ok(DmSegMobileReply::decode(res.bytes().await?)?.elems)
    }

    pub async fn get_page_analyzer(&self, page: &PageInfo) -> Result<PageAnalyzer> {
        let mut res = self
            .client
            .request(
                Method::GET,
                "https://api.bilibili.com/x/player/wbi/playurl",
                self.credential,
            )
            .await
            .query(&[
                ("bvid", self.bvid.as_str()),
                ("qn", "127"),
                ("otype", "json"),
                ("fnval", "4048"),
                ("fourk", "1"),
            ])
            .query(&[("cid", page.cid)])
            .wbi_sign(MIXIN_KEY.load().as_deref())?
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(PageAnalyzer::new(res["data"].take()))
    }

    pub async fn get_subtitles(&self, page: &PageInfo) -> Result<Vec<SubTitle>> {
        let mut res = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/player/wbi/v2", self.credential)
            .await
            .query(&[("bvid", self.bvid.as_str())])
            .query(&[("cid", page.cid)])
            .wbi_sign(MIXIN_KEY.load().as_deref())?
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        // 接口返回的信息，包含了一系列的字幕，每个字幕包含了字幕的语言和 json 下载地址
        match serde_json::from_value::<Option<SubTitlesInfo>>(res["data"]["subtitle"].take())? {
            Some(subtitles_info) => {
                let tasks = subtitles_info
                    .subtitles
                    .into_iter()
                    .filter(|v| !v.is_ai_sub())
                    .map(|v| self.get_subtitle(v))
                    .collect::<FuturesUnordered<_>>();
                tasks.try_collect().await
            }
            None => Ok(vec![]),
        }
    }

    async fn get_subtitle(&self, info: SubTitleInfo) -> Result<SubTitle> {
        let mut res = self
            .client
            .client // 这里可以直接使用 inner_client，因为该请求不需要鉴权
            .request(Method::GET, format!("https:{}", &info.subtitle_url).as_str(), None)
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?;
        let body: SubTitleBody = serde_json::from_value(res["body"].take())?;
        Ok(SubTitle { lan: info.lan, body })
    }
}
