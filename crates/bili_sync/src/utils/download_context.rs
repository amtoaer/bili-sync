use sea_orm::DatabaseConnection;

use crate::adapter::VideoSourceEnum;
use crate::bilibili::BiliClient;
use crate::config::Config;
use crate::downloader::Downloader;

#[derive(Clone, Copy)]
pub struct DownloadContext<'a> {
    pub bili_client: &'a BiliClient,
    pub video_source: &'a VideoSourceEnum,
    pub template: &'a handlebars::Handlebars<'a>,
    pub connection: &'a DatabaseConnection,
    pub downloader: &'a Downloader,
    pub config: &'a Config,
}

impl<'a> DownloadContext<'a> {
    pub fn new(
        bili_client: &'a BiliClient,
        video_source: &'a VideoSourceEnum,
        template: &'a handlebars::Handlebars<'a>,
        connection: &'a DatabaseConnection,
        downloader: &'a Downloader,
        config: &'a Config,
    ) -> Self {
        Self {
            bili_client,
            video_source,
            template,
            connection,
            downloader,
            config,
        }
    }
}
