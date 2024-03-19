use reqwest::{header, Method};

use crate::bilibili::Credential;
use crate::Result;

pub struct Client(reqwest::Client);

impl Client {
    pub fn new() -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36 Edg/116.0.1938.54"));
        headers.insert(
            header::REFERER,
            header::HeaderValue::from_static("https://www.bilibili.com"),
        );
        Self(
            reqwest::Client::builder()
                .default_headers(headers)
                .build()
                .unwrap(),
        )
    }

    pub fn request(
        &self,
        method: Method,
        url: &str,
        credential: Option<&Credential>,
    ) -> reqwest::RequestBuilder {
        let mut req = self.0.request(method, url);
        if let Some(credential) = credential {
            req = req
                .header(header::COOKIE, format!("SESSDATA={}", credential.sessdata))
                .header(header::COOKIE, format!("bili_jct={}", credential.bili_jct))
                .header(header::COOKIE, format!("buvid3={}", credential.buvid3))
                .header(
                    header::COOKIE,
                    format!("DedeUserID={}", credential.dedeuserid),
                )
                .header(
                    header::COOKIE,
                    format!("ac_time_value={}", credential.ac_time_value),
                );
        }
        req
    }
}

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
            // no credential, just ignore it
            return Ok(());
        };
        if credential.check(&self.client).await? {
            // is valid, no need to refresh
            return Ok(());
        }
        credential.refresh(&self.client).await
    }
}
