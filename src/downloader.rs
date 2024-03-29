use std::path::Path;

use futures_util::StreamExt;
use reqwest::Method;
use tokio::fs::{self, File};
use tokio::io;

use crate::bilibili::Client;
use crate::Result;
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
        let mut res = self.client.request(Method::GET, url, None).send().await?.bytes_stream();
        while let Some(item) = res.next().await {
            io::copy(&mut item?.as_ref(), &mut file).await?;
        }
        Ok(())
    }

    pub async fn merge(&self, video_path: &Path, audio_path: &Path, output_path: &Path) -> Result<()> {
        let output = tokio::process::Command::new("ffmpeg")
            .args([
                "-i",
                video_path.to_str().unwrap(),
                "-i",
                audio_path.to_str().unwrap(),
                "-c",
                "copy",
                output_path.to_str().unwrap(),
            ])
            .output()
            .await?;
        if !output.status.success() {
            return match String::from_utf8(output.stderr) {
                Ok(err) => Err(err.into()),
                _ => Err("ffmpeg error".into()),
            };
        }
        let _ = fs::remove_file(video_path).await;
        let _ = fs::remove_file(audio_path).await;
        Ok(())
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new(Client::new())
    }
}
