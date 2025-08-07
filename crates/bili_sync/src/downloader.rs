use core::str;
use std::io::SeekFrom;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail, ensure};
use futures::TryStreamExt;
use reqwest::{Method, header};
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::task::JoinSet;
use tokio_util::io::StreamReader;

use crate::bilibili::Client;
use crate::config::VersionedConfig;
pub struct Downloader {
    client: Client,
}

impl Downloader {
    // Downloader 使用带有默认 Header 的 Client 构建
    // 拿到 url 后下载文件不需要任何 cookie 作为身份凭证
    // 但如果不设置默认 Header，下载时会遇到 403 Forbidden 错误
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn fetch(&self, url: &str, path: &Path) -> Result<()> {
        if VersionedConfig::get().load().concurrent_limit.download.enable {
            self.fetch_parallel(url, path).await
        } else {
            self.fetch_serial(url, path).await
        }
    }

    async fn fetch_serial(&self, url: &str, path: &Path) -> Result<()> {
        let resp = self
            .client
            .request(Method::GET, url, None)
            .send()
            .await?
            .error_for_status()?;
        let expected = resp.header_content_length();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut file = File::create(path).await?;
        let mut stream_reader = StreamReader::new(resp.bytes_stream().map_err(std::io::Error::other));
        let received = tokio::io::copy(&mut stream_reader, &mut file).await?;
        file.flush().await?;
        if let Some(expected) = expected {
            ensure!(
                received == expected,
                "downloaded bytes mismatch: expected {}, got {}",
                expected,
                received
            );
        }
        Ok(())
    }

    async fn fetch_parallel(&self, url: &str, path: &Path) -> Result<()> {
        let (concurrency, threshold) = {
            let config = VersionedConfig::get().load();
            (
                config.concurrent_limit.download.concurrency,
                config.concurrent_limit.download.threshold,
            )
        };
        let resp = self
            .client
            .request(Method::HEAD, url, None)
            .send()
            .await?
            .error_for_status()?;
        let file_size = resp.header_content_length().unwrap_or_default();
        let chunk_size = file_size / concurrency as u64;
        if resp
            .headers()
            .get(header::ACCEPT_RANGES)
            .is_none_or(|v| v.to_str().unwrap_or_default() == "none") // https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Accept-Ranges#none
            || chunk_size < threshold
        {
            return self.fetch_serial(url, path).await;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let file = File::create(path).await?;
        file.set_len(file_size).await?;
        drop(file);
        let mut tasks = JoinSet::new();
        let url = Arc::new(url.to_string());
        let path = Arc::new(path.to_path_buf());
        for i in 0..concurrency {
            let start = i as u64 * chunk_size;
            let end = if i == concurrency - 1 {
                file_size
            } else {
                start + chunk_size
            } - 1;
            let (url_clone, path_clone, client_clone) = (url.clone(), path.clone(), self.client.clone());
            tasks.spawn(async move {
                let mut file = OpenOptions::new().write(true).open(path_clone.as_ref()).await?;
                file.seek(SeekFrom::Start(start)).await?;
                let range_header = format!("bytes={}-{}", start, end);
                let resp = client_clone
                    .request(Method::GET, &url_clone, None)
                    .header(header::RANGE, &range_header)
                    .send()
                    .await?
                    .error_for_status()?;
                if let Some(content_length) = resp.header_content_length() {
                    ensure!(
                        content_length == end - start + 1,
                        "content length mismatch: expected {}, got {}",
                        end - start + 1,
                        content_length
                    );
                }
                let mut stream_reader = StreamReader::new(resp.bytes_stream().map_err(std::io::Error::other));
                let received = tokio::io::copy(&mut stream_reader, &mut file).await?;
                file.flush().await?;
                ensure!(
                    received == end - start + 1,
                    "downloaded bytes mismatch: expected {}, got {}",
                    end - start + 1,
                    received,
                );
                Ok(())
            });
        }
        while let Some(res) = tasks.join_next().await {
            res??;
        }
        Ok(())
    }

    pub async fn fetch_with_fallback(&self, urls: &[&str], path: &Path) -> Result<()> {
        if urls.is_empty() {
            bail!("no urls provided");
        }
        let mut res = Ok(());
        for url in urls {
            match self.fetch(url, path).await {
                Ok(_) => return Ok(()),
                Err(err) => {
                    res = Err(err);
                }
            }
        }
        res.with_context(|| format!("failed to download from {:?}", urls))
    }

    pub async fn merge(&self, video_path: &Path, audio_path: &Path, output_path: &Path) -> Result<()> {
        let output = tokio::process::Command::new("ffmpeg")
            .args([
                "-i",
                video_path.to_string_lossy().as_ref(),
                "-i",
                audio_path.to_string_lossy().as_ref(),
                "-c",
                "copy",
                "-strict",
                "unofficial",
                "-y",
                output_path.to_string_lossy().as_ref(),
            ])
            .output()
            .await
            .context("failed to run ffmpeg")?;
        if !output.status.success() {
            bail!("ffmpeg error: {}", str::from_utf8(&output.stderr).unwrap_or("unknown"));
        }
        Ok(())
    }
}

/// reqwest.content_length() 居然指的是 body_size 而非 content-length header，没办法自己实现一下
/// https://github.com/seanmonstar/reqwest/issues/1814
trait ResponseExt {
    fn header_content_length(&self) -> Option<u64>;
}

impl ResponseExt for reqwest::Response {
    fn header_content_length(&self) -> Option<u64> {
        self.headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
    }
}
