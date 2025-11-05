use core::str;
use std::io::SeekFrom;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail, ensure};
use async_tempfile::TempFile;
use futures::TryStreamExt;
use reqwest::{Method, header};
use tokio::fs::{self};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::task::JoinSet;
use tokio_util::io::StreamReader;

use crate::bilibili::Client;
use crate::config::ConcurrentDownloadLimit;

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

    pub async fn fetch(&self, url: &str, path: &Path, concurrent_download: &ConcurrentDownloadLimit) -> Result<()> {
        let mut temp_file = TempFile::new().await?;
        self.fetch_internal(url, &mut temp_file, concurrent_download).await?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::copy(temp_file.file_path(), path).await?;
        // temp_file 的 drop 需要 std::fs::remove_file
        // 如果交由 rust 自动执行虽然逻辑正确但会略微阻塞异步上下文
        // 尽量主动调用，保证正常执行的情况下文件清除操作由 spawn_blocking 在专门线程中完成
        temp_file.drop_async().await;
        Ok(())
    }

    pub async fn multi_fetch(
        &self,
        urls: &[&str],
        path: &Path,
        concurrent_download: &ConcurrentDownloadLimit,
    ) -> Result<()> {
        let temp_file = self.multi_fetch_internal(urls, concurrent_download).await?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::copy(temp_file.file_path(), path).await?;
        temp_file.drop_async().await;
        Ok(())
    }

    pub async fn multi_fetch_and_merge(
        &self,
        video_urls: &[&str],
        audio_urls: &[&str],
        path: &Path,
        concurrent_download: &ConcurrentDownloadLimit,
    ) -> Result<()> {
        let (video_temp_file, audio_temp_file) = tokio::try_join!(
            self.multi_fetch_internal(video_urls, concurrent_download),
            self.multi_fetch_internal(audio_urls, concurrent_download)
        )?;
        let final_temp_file = TempFile::new().await?;
        let output = Command::new("ffmpeg")
            .args([
                "-i",
                video_temp_file.file_path().to_string_lossy().as_ref(),
                "-i",
                audio_temp_file.file_path().to_string_lossy().as_ref(),
                "-c",
                "copy",
                "-strict",
                "unofficial",
                "-f",
                "mp4",
                "-y",
                final_temp_file.file_path().to_string_lossy().as_ref(),
            ])
            .output()
            .await
            .context("failed to run ffmpeg")?;
        if !output.status.success() {
            bail!("ffmpeg error: {}", str::from_utf8(&output.stderr).unwrap_or("unknown"));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::copy(final_temp_file.file_path(), path).await?;
        tokio::join!(
            video_temp_file.drop_async(),
            audio_temp_file.drop_async(),
            final_temp_file.drop_async()
        );
        Ok(())
    }

    async fn multi_fetch_internal(
        &self,
        urls: &[&str],
        concurrent_download: &ConcurrentDownloadLimit,
    ) -> Result<TempFile> {
        if urls.is_empty() {
            bail!("no urls provided");
        }
        let mut temp_file = TempFile::new().await?;
        for (idx, url) in urls.iter().enumerate() {
            match self.fetch_internal(url, &mut temp_file, concurrent_download).await {
                Ok(_) => return Ok(temp_file),
                Err(e) => {
                    if idx == urls.len() - 1 {
                        temp_file.drop_async().await;
                        return Err(e).with_context(|| format!("failed to download file from all {} urls", urls.len()));
                    }
                    temp_file.set_len(0).await?;
                    temp_file.rewind().await?;
                }
            }
        }
        unreachable!()
    }

    async fn fetch_internal(
        &self,
        url: &str,
        file: &mut TempFile,
        concurrent_download: &ConcurrentDownloadLimit,
    ) -> Result<()> {
        if concurrent_download.enable {
            self.fetch_parallel(url, file, concurrent_download).await
        } else {
            self.fetch_serial(url, file).await
        }
    }

    async fn fetch_serial(&self, url: &str, file: &mut TempFile) -> Result<()> {
        let resp = self
            .client
            .request(Method::GET, url, None)
            .send()
            .await?
            .error_for_status()?;
        let expected = resp.header_content_length();
        let mut stream_reader = StreamReader::new(resp.bytes_stream().map_err(std::io::Error::other));
        let received = tokio::io::copy(&mut stream_reader, file).await?;
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

    async fn fetch_parallel(
        &self,
        url: &str,
        file: &mut TempFile,
        concurrent_download: &ConcurrentDownloadLimit,
    ) -> Result<()> {
        let (concurrency, threshold) = (concurrent_download.concurrency, concurrent_download.threshold);
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
            return self.fetch_serial(url, file).await;
        }
        file.set_len(file_size).await?;
        let mut tasks = JoinSet::new();
        let url = Arc::new(url.to_string());
        for i in 0..concurrency {
            let start = i as u64 * chunk_size;
            let end = if i == concurrency - 1 {
                file_size
            } else {
                start + chunk_size
            } - 1;
            let (url_clone, client_clone) = (url.clone(), self.client.clone());
            let mut file_clone = file.open_rw().await?;
            tasks.spawn(async move {
                file_clone.seek(SeekFrom::Start(start)).await?;
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
                let received = tokio::io::copy(&mut stream_reader, &mut file_clone).await?;
                file_clone.flush().await?;
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
