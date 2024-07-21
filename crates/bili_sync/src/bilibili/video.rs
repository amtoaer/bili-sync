use anyhow::{bail, Result};
use futures::stream::FuturesUnordered;
use futures::TryStreamExt;
use prost::Message;
use reqwest::Method;

use super::credential::encoded_query;
use super::MIXIN_KEY;
use crate::bilibili::analyzer::PageAnalyzer;
use crate::bilibili::client::BiliClient;
use crate::bilibili::danmaku::{DanmakuElem, DanmakuWriter, DmSegMobileReply};
use crate::bilibili::{Validate, VideoInfo};

static MASK_CODE: u64 = 2251799813685247;
static XOR_CODE: u64 = 23442827791579;
static BASE: u64 = 58;
static DATA: &[char] = &[
    'F', 'c', 'w', 'A', 'P', 'N', 'K', 'T', 'M', 'u', 'g', '3', 'G', 'V', '5', 'L', 'j', '7', 'E', 'J', 'n', 'H', 'p',
    'W', 's', 'x', '4', 't', 'b', '8', 'h', 'a', 'Y', 'e', 'v', 'i', 'q', 'B', 'z', '6', 'r', 'k', 'C', 'y', '1', '2',
    'm', 'U', 'S', 'D', 'Q', 'X', '9', 'R', 'd', 'o', 'Z', 'f',
];

pub struct Video<'a> {
    client: &'a BiliClient,
    pub aid: String,
    pub bvid: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Tag {
    pub tag_name: String,
}

impl serde::Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.tag_name)
    }
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
    pub fn new(client: &'a BiliClient, bvid: String) -> Self {
        let aid = bvid_to_aid(&bvid).to_string();
        Self { client, aid, bvid }
    }

    #[allow(dead_code)]
    /// 直接调用视频信息接口获取详细的视频信息
    pub async fn get_view_info(&self) -> Result<VideoInfo> {
        let mut res = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/web-interface/view")
            .query(&[("aid", &self.aid), ("bvid", &self.bvid)])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(res["data"].take())?)
    }

    pub async fn get_pages(&self) -> Result<Vec<PageInfo>> {
        let mut res = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/player/pagelist")
            .query(&[("aid", &self.aid), ("bvid", &self.bvid)])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(res["data"].take())?)
    }

    pub async fn get_tags(&self) -> Result<Vec<Tag>> {
        let mut res = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/web-interface/view/detail/tag")
            .query(&[("aid", &self.aid), ("bvid", &self.bvid)])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(res["data"].take())?)
    }

    pub async fn get_danmaku_writer(&self, page: &'a PageInfo) -> Result<DanmakuWriter> {
        let tasks = FuturesUnordered::new();
        for i in 1..=(page.duration + 359) / 360 {
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
            .request(Method::GET, "http://api.bilibili.com/x/v2/dm/web/seg.so")
            .query(&[("type", 1), ("oid", page.cid), ("segment_index", segment_idx)])
            .send()
            .await?
            .error_for_status()?;
        let headers = std::mem::take(res.headers_mut());
        let content_type = headers.get("content-type");
        if !content_type.is_some_and(|v| v == "application/octet-stream") {
            bail!(
                "unexpected content type: {:?}, body: {:?}",
                content_type,
                res.text().await
            );
        }
        Ok(DmSegMobileReply::decode(res.bytes().await?)?.elems)
    }

    pub async fn get_page_analyzer(&self, page: &PageInfo) -> Result<PageAnalyzer> {
        let mut res = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/player/wbi/playurl")
            .query(&encoded_query(
                vec![
                    ("avid", self.aid.as_str()),
                    ("cid", page.cid.to_string().as_str()),
                    ("qn", "127"),
                    ("otype", "json"),
                    ("fnval", "4048"),
                    ("fourk", "1"),
                ],
                MIXIN_KEY.load().as_ref().unwrap(),
            ))
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(PageAnalyzer::new(res["data"].take()))
    }
}

fn bvid_to_aid(bvid: &str) -> u64 {
    let mut bvid = bvid.chars().collect::<Vec<_>>();
    (bvid[3], bvid[9]) = (bvid[9], bvid[3]);
    (bvid[4], bvid[7]) = (bvid[7], bvid[4]);
    let mut tmp = 0u64;
    for char in bvid.into_iter().skip(3) {
        let idx = DATA.iter().position(|&x| x == char).unwrap();
        tmp = tmp * BASE + idx as u64;
    }
    (tmp & MASK_CODE) ^ XOR_CODE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bvid_to_aid() {
        assert_eq!(bvid_to_aid("BV1Tr421n746"), 1401752220u64);
        assert_eq!(bvid_to_aid("BV1sH4y1s7fe"), 1051892992u64);
    }
}
