// 暂时保留，避免黄色波浪线
#![allow(dead_code, unused_imports)]
use reqwest::Method;
use serde_json;
use std::error;
use std::ops::Index;
use std::rc::Rc;

static MASK_CODE: u64 = 2251799813685247;
static XOR_CODE: u64 = 23442827791579;
static BASE: u64 = 58;
static DATA: &[char] = &[
    'F', 'c', 'w', 'A', 'P', 'N', 'K', 'T', 'M', 'u', 'g', '3', 'G', 'V', '5', 'L', 'j', '7', 'E',
    'J', 'n', 'H', 'p', 'W', 's', 'x', '4', 't', 'b', '8', 'h', 'a', 'Y', 'e', 'v', 'i', 'q', 'B',
    'z', '6', 'r', 'k', 'C', 'y', '1', '2', 'm', 'U', 'S', 'D', 'Q', 'X', '9', 'R', 'd', 'o', 'Z',
    'f',
];

pub struct Credential {
    sessdata: String,
    bili_jct: String,
    buvid3: String,
    dedeuserid: String,
    ac_time_value: String,
}

impl Credential {
    pub fn new(
        sessdata: String,
        bili_jct: String,
        buvid3: String,
        dedeuserid: String,
        ac_time_value: String,
    ) -> Self {
        Self {
            sessdata,
            bili_jct,
            buvid3,
            dedeuserid,
            ac_time_value,
        }
    }
}

pub struct BiliClient {
    credential: Option<Credential>,
    client: reqwest::Client,
}

impl BiliClient {
    pub fn anonymous() -> Self {
        let credential = None;
        let client = reqwest::Client::new();
        Self { credential, client }
    }

    pub fn authenticated(credential: Credential) -> Self {
        let credential = Some(credential);
        let client = reqwest::Client::new();
        Self { credential, client }
    }

    fn set_header(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let req =req.header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36 Edg/116.0.1938.54")
        .header("Referer", "https://www.bilibili.com");
        if let Some(credential) = &self.credential {
            return req.header("cookie", format!("SESSDATA={}", credential.sessdata))
            .header("cookie", format!("bili_jct={}", credential.bili_jct))
            .header("cookie", format!("buvid3={}", credential.buvid3))
            .header(
                "cookie",
                format!("DedeUserID={}", credential.dedeuserid),
            )
            .header(
                "cookie",
                format!("ac_time_value={}", credential.ac_time_value),
            ).header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36 Edg/116.0.1938.54")
            .header("Referer", "https://www.bilibili.com");
        }
        req
    }

    pub fn request(&self, method: Method, url: &str) -> reqwest::RequestBuilder {
        self.set_header(self.client.request(method, url))
    }
}

struct Video {
    client: Rc<BiliClient>,
    pub aid: u64,
    pub bvid: String,
}

impl Video {
    pub fn new(client: Rc<BiliClient>, bvid: String) -> Self {
        let aid = bvid_to_aid(&bvid);
        Self { client, aid, bvid }
    }

    pub async fn get_info(&self) -> Result<serde_json::Value, Box<dyn error::Error>> {
        let res = self
            .client
            .request(
                Method::GET,
                &"https://api.bilibili.com/x/web-interface/view",
            )
            .query(&[("aid", self.aid.to_string()), ("bvid", self.bvid.clone())])
            .send()
            .await?
            .text()
            .await?;
        let json: serde_json::Value = serde_json::from_str(&res)?;
        Ok(json)
    }
}

fn bvid_to_aid(bvid: &str) -> u64 {
    let mut bvid = bvid.chars().collect::<Vec<_>>();
    (bvid[3], bvid[9]) = (bvid[9], bvid[3]);
    (bvid[4], bvid[7]) = (bvid[7], bvid[4]);
    let mut tmp = 0u64;
    for i in 3..bvid.len() {
        let idx = DATA.iter().position(|&x| x == bvid[i]).unwrap();
        tmp = tmp * BASE + idx as u64;
    }
    return (tmp & MASK_CODE) ^ XOR_CODE;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bvid_to_aid() {
        assert_eq!(bvid_to_aid("BV1Tr421n746"), 1401752220u64);
        assert_eq!(bvid_to_aid("BV1sH4y1s7fe"), 1051892992u64);
    }

    #[ignore = "only for manual test, need to connect to the internet"]
    #[tokio::test]
    async fn test_get_video_info() {
        let client = Rc::new(BiliClient::anonymous());
        let video = Video::new(client, "BV1sH4y1s7fe".to_string());
        let info = video.get_info().await.unwrap();
        assert_eq!(info["code"], 0);
    }
}
