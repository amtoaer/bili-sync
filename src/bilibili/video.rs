use std::sync::Arc;

use reqwest::Method;

use crate::bilibili::analyzer::PageAnalyzer;
use crate::bilibili::client::BiliClient;
use crate::Result;

static MASK_CODE: u64 = 2251799813685247;
static XOR_CODE: u64 = 23442827791579;
static BASE: u64 = 58;
static DATA: &[char] = &[
    'F', 'c', 'w', 'A', 'P', 'N', 'K', 'T', 'M', 'u', 'g', '3', 'G', 'V', '5', 'L', 'j', '7', 'E',
    'J', 'n', 'H', 'p', 'W', 's', 'x', '4', 't', 'b', '8', 'h', 'a', 'Y', 'e', 'v', 'i', 'q', 'B',
    'z', '6', 'r', 'k', 'C', 'y', '1', '2', 'm', 'U', 'S', 'D', 'Q', 'X', '9', 'R', 'd', 'o', 'Z',
    'f',
];

pub struct Video {
    client: Arc<BiliClient>,
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

#[derive(Debug, serde::Deserialize)]
pub struct PageInfo {
    pub cid: i32,
    pub page: i32,
    #[serde(rename = "part")]
    pub name: String,
    #[serde(default = "String::new")]
    pub first_frame: String, // 可能不存在，默认填充为空
}

impl Video {
    pub fn new(client: Arc<BiliClient>, bvid: String) -> Self {
        let aid = bvid_to_aid(&bvid).to_string();
        Self { client, aid, bvid }
    }

    pub async fn get_pages(&self) -> Result<Vec<PageInfo>> {
        let mut res = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/player/pagelist")
            .query(&[("aid", &self.aid), ("bvid", &self.bvid)])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        Ok(serde_json::from_value(res["data"].take())?)
    }

    pub async fn get_tags(&self) -> Result<Vec<Tag>> {
        let mut res = self
            .client
            .request(
                Method::GET,
                "https://api.bilibili.com/x/web-interface/view/detail/tag",
            )
            .query(&[("aid", &self.aid), ("bvid", &self.bvid)])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        Ok(serde_json::from_value(res["data"].take())?)
    }

    pub async fn get_page_analyzer(&self, page: &PageInfo) -> Result<PageAnalyzer> {
        let mut res = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/player/wbi/playurl")
            .query(&[
                ("avid", self.aid.as_str()),
                ("cid", page.cid.to_string().as_str()),
                ("qn", "127"),
                ("otype", "json"),
                ("fnval", "4048"),
                ("fourk", "1"),
            ])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        if res["code"] != 0 {
            return Err(format!("get page analyzer failed: {}", res["message"]).into());
        }
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

    #[tokio::test]
    async fn test_bvid_to_aid() {
        assert_eq!(bvid_to_aid("BV1Tr421n746"), 1401752220u64);
        assert_eq!(bvid_to_aid("BV1sH4y1s7fe"), 1051892992u64);
    }
}
