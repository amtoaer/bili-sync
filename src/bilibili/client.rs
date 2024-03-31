use reqwest::{header, Method};

use crate::bilibili::Credential;
use crate::config::CONFIG;
use crate::Result;

// 一个对 reqwest::Client 的简单封装，用于 Bilibili 请求
pub struct Client(reqwest::Client);

impl Client {
    pub fn new() -> Self {
        // 正常访问 api 所必须的 header，作为默认 header 添加到每个请求中
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36 Edg/116.0.1938.54",
            ),
        );
        headers.insert(
            header::REFERER,
            header::HeaderValue::from_static("https://www.bilibili.com"),
        );
        Self(
            reqwest::Client::builder()
                .default_headers(headers)
                .gzip(true)
                .build()
                .unwrap(),
        )
    }

    // a wrapper of reqwest::Client::request to add credential to the request
    pub fn request(&self, method: Method, url: &str, credential: Option<&Credential>) -> reqwest::RequestBuilder {
        let mut req = self.0.request(method, url);
        // 如果有 credential，会将其转换成 cookie 添加到请求的 header 中
        if let Some(credential) = credential {
            req = req
                .header(header::COOKIE, format!("SESSDATA={}", credential.sessdata))
                .header(header::COOKIE, format!("bili_jct={}", credential.bili_jct))
                .header(header::COOKIE, format!("buvid3={}", credential.buvid3))
                .header(header::COOKIE, format!("DedeUserID={}", credential.dedeuserid))
                .header(header::COOKIE, format!("ac_time_value={}", credential.ac_time_value));
        }
        req
    }
}

// clippy 建议实现 Default trait
impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BiliClient {
    credential: Option<Credential>,
    client: Client,
}

impl BiliClient {
    pub fn new(credential: Option<Credential>) -> Self {
        let client = Client::new();
        Self { credential, client }
    }

    pub fn request(&self, method: Method, url: &str) -> reqwest::RequestBuilder {
        self.client.request(method, url, self.credential.as_ref())
    }

    pub async fn check_refresh(&mut self) -> Result<()> {
        let Some(credential) = self.credential.as_mut() else {
            return Ok(());
        };
        if !credential.need_refresh(&self.client).await? {
            return Ok(());
        }
        credential.refresh(&self.client).await?;

        let mut config = CONFIG.lock().unwrap();
        config.credential = Some(credential.clone());
        config.save()
    }
}
