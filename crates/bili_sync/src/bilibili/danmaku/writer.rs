use std::path::PathBuf;

use anyhow::Result;
use tokio::fs::{self, File};

use crate::bilibili::PageInfo;
use crate::bilibili::danmaku::canvas::CanvasConfig;
use crate::bilibili::danmaku::{AssWriter, Danmu};
use crate::config::config_template_owned;

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
        let config_template = config_template_owned();
        let config = &config_template.config;
        let canvas_config = CanvasConfig::new(&config.danmaku_option, self.page);
        let mut canvas = canvas_config.clone().canvas();
        let mut writer = AssWriter::construct(File::create(path).await?, self.page.name.clone(), canvas_config).await?;
        for danmuku in self.danmaku {
            if let Some(drawable) = canvas.draw(danmuku)? {
                writer.write(drawable).await?;
            }
        }
        writer.flush().await?;
        Ok(())
    }
}
