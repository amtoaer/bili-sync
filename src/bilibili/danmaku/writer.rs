use std::path::PathBuf;

use anyhow::Result;
use tokio::fs::{self, File};

use super::{AssWriter, Danmu, SubtitleOption};
use crate::bilibili::PageInfo;

pub struct DanmakuWriter<'a> {
    page: &'a PageInfo,
    danmaku: Vec<Danmu>,
}

impl<'a> DanmakuWriter<'a> {
    pub fn new(page: &'a PageInfo, danmaku: Vec<Danmu>) -> Self {
        DanmakuWriter { page, danmaku }
    }

    pub async fn write(self, path: PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let canvas_config = SubtitleOption::default();
        let mut writer =
            AssWriter::inited(File::create(path).await?, self.page.name.clone(), canvas_config.clone()).await?;
        let mut canvas = canvas_config.canvas();
        for danmuku in self.danmaku {
            if let Some(drawable) = canvas.draw(danmuku)? {
                writer.write(drawable).await?;
            }
        }
        writer.flush().await?;
        Ok(())
    }
}
