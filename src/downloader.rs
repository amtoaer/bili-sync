use std::path::Path;

use futures_util::StreamExt;
use tokio::fs::{self, File};
use tokio::io;

use crate::bilibili::client_with_header;
use crate::Result;
pub struct Downloader {
    client: reqwest::Client,
}

impl Downloader {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub async fn fetch(&self, url: &str, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        // must be a new file
        let mut file = File::create(path).await?;
        let mut res = self.client.get(url).send().await?.bytes_stream();
        while let Some(item) = res.next().await {
            io::copy(&mut item?.as_ref(), &mut file).await?;
        }
        Ok(())
    }

    pub async fn merge(
        &self,
        video_path: &Path,
        audio_path: &Path,
        output_path: &Path,
    ) -> Result<()> {
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
        Self::new(client_with_header())
    }
}
