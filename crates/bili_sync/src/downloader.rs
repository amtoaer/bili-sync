use core::str;
use std::path::Path;

use anyhow::{bail, ensure, Result};
use futures::StreamExt;
use reqwest::Method;
use tokio::fs::{self, File};
use tokio::io::{self, AsyncWriteExt};

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
        let resp = self.client.request(Method::GET, url, None).send().await?;
        let expected = resp.content_length().unwrap_or_else(|| {
            warn!("content length is missing, fallback to 0");
            0
        });
        let mut received = 0u64;
        let mut stream = resp.bytes_stream();
        while let Some(bytes) = stream.next().await {
            let bytes = bytes?;
            received += bytes.len() as u64;
            io::copy(&mut bytes.as_ref(), &mut file).await?;
        }
        file.flush().await?;
        ensure!(
            received >= expected,
            "received {} bytes, expected {} bytes",
            received,
            expected
        );
        Ok(())
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
