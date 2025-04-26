use core::str;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use futures::TryStreamExt;
use reqwest::Method;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio_util::io::StreamReader;

use crate::bilibili::Client;
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
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut file = File::create(path).await?;
        let resp = self
            .client
            .request(Method::GET, url, None)
            .send()
            .await?
            .error_for_status()?;
        let expected = resp.content_length().unwrap_or_default();
        let mut stream_reader = StreamReader::new(resp.bytes_stream().map_err(std::io::Error::other));
        let received = tokio::io::copy(&mut stream_reader, &mut file).await?;
        file.flush().await?;
        ensure!(
            received >= expected,
            "received {} bytes, expected {} bytes",
            received,
            expected
        );
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
                "-movflags",
                "faststart",
                "-strict",
                "unofficial",
                "-strict",
                "experimental",
                "-y",
                output_path.to_string_lossy().as_ref(),
            ])
            .output()
            .await?;
        if !output.status.success() {
            bail!("ffmpeg error: {}", str::from_utf8(&output.stderr).unwrap_or("unknown"));
        }
        Ok(())
    }
}
