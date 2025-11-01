use std::time::Duration;

use anyhow::Result;
use leaky_bucket::RateLimiter;
use reqwest::{Method, header};
use sea_orm::DatabaseConnection;
use ua_generator::ua;

use crate::bilibili::Credential;
use crate::bilibili::credential::WbiImg;
use crate::config::{RateLimit, VersionedCache, VersionedConfig};

// 一个对 reqwest::Client 的简单封装，用于 Bilibili 请求
#[derive(Clone)]
pub struct Client(reqwest::Client);

impl Client {
    pub fn new() -> Self {
        // 正常访问 api 所必须的 header，作为默认 header 添加到每个请求中
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(ua::spoof_chrome_ua()),
        );
        headers.insert(
            header::REFERER,
            header::HeaderValue::from_static("https://www.bilibili.com"),
        );
        Self(
            reqwest::Client::builder()
                .default_headers(headers)
                .gzip(true)
                .connect_timeout(std::time::Duration::from_secs(10))
                .read_timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build reqwest client"),
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
    pub client: Client,
    pub limiter: VersionedCache<Option<RateLimiter>>,
}

impl BiliClient {
    pub fn new() -> Self {
        let client = Client::new();
        let limiter = VersionedCache::new(
            |config| {
                Ok(config
                    .concurrent_limit
                    .rate_limit
                    .as_ref()
                    .map(|RateLimit { limit, duration }| {
                        RateLimiter::builder()
                            .initial(*limit)
                            .refill(*limit)
                            .max(*limit)
                            .interval(Duration::from_millis(*duration))
                            .build()
                    }))
            },
            &VersionedConfig::get().load(),
        )
        .expect("failed to create rate limiter");
        Self { client, limiter }
    }

    /// 获取一个预构建的请求，通过该方法获取请求时会检查并等待速率限制
    pub async fn request(&self, method: Method, url: &str) -> reqwest::RequestBuilder {
        if let Some(limiter) = self.limiter.load().as_ref() {
            limiter.acquire_one().await;
        }
        let credential = &VersionedConfig::get().load().credential;
        self.client.request(method, url, Some(credential))
    }

    pub async fn check_refresh(&self, connection: &DatabaseConnection) -> Result<()> {
        let credential = &VersionedConfig::get().load().credential;
        if !credential.need_refresh(&self.client).await? {
            return Ok(());
        }
        let new_credential = credential.refresh(&self.client).await?;
        VersionedConfig::get()
            .update_credential(new_credential, connection)
            .await?;
        Ok(())
    }

    /// 获取 wbi img，用于生成请求签名
    pub async fn wbi_img(&self) -> Result<WbiImg> {
        let credential = &VersionedConfig::get().load().credential;
        credential.wbi_img(&self.client).await
    }
}
