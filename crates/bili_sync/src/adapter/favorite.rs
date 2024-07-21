use std::collections::HashSet;
use std::path::Path;
use std::pin::Pin;

use anyhow::Result;
use bili_sync_entity::*;
use bili_sync_migration::OnConflict;
use filenamify::filenamify;
use futures::Stream;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, QuerySelect, TransactionTrait};

use crate::adapter::VideoListModel;
use crate::bilibili::{BiliClient, BiliError, FavoriteList, Video, VideoInfo};
use crate::config::TEMPLATE;
use crate::utils::id_time_key;
use crate::utils::model::create_video_pages;
use crate::utils::status::Status;

pub async fn favorite_from<'a>(
    fid: &str,
    path: &Path,
    bili_client: &'a BiliClient,
    connection: &DatabaseConnection,
) -> Result<(Box<dyn VideoListModel>, Pin<Box<dyn Stream<Item = VideoInfo> + 'a>>)> {
    let favorite = FavoriteList::new(bili_client, fid.to_owned());
    let favorite_info = favorite.get_info().await?;
    favorite::Entity::insert(favorite::ActiveModel {
        f_id: Set(favorite_info.id),
        name: Set(favorite_info.title.clone()),
        path: Set(path.to_string_lossy().to_string()),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(favorite::Column::FId)
            .update_columns([favorite::Column::Name, favorite::Column::Path])
            .to_owned(),
    )
    .exec(connection)
    .await?;
    Ok((
        Box::new(
            favorite::Entity::find()
                .filter(favorite::Column::FId.eq(favorite_info.id))
                .one(connection)
                .await?
                .unwrap(),
        ),
        Box::pin(favorite.into_video_stream()),
    ))
}

use async_trait::async_trait;

#[async_trait]
impl VideoListModel for favorite::Model {
    async fn video_count(&self, connection: &DatabaseConnection) -> Result<u64> {
        Ok(video::Entity::find()
            .filter(video::Column::FavoriteId.eq(self.id))
            .count(connection)
            .await?)
    }

    async fn unfilled_videos(&self, connection: &DatabaseConnection) -> Result<Vec<video::Model>> {
        Ok(video::Entity::find()
            .filter(
                video::Column::FavoriteId
                    .eq(self.id)
                    .and(video::Column::Valid.eq(true))
                    .and(video::Column::DownloadStatus.eq(0))
                    .and(video::Column::Category.eq(2))
                    .and(video::Column::SinglePage.is_null()),
            )
            .all(connection)
            .await?)
    }

    async fn unhandled_video_pages(
        &self,
        connection: &DatabaseConnection,
    ) -> Result<Vec<(video::Model, Vec<page::Model>)>> {
        Ok(video::Entity::find()
            .filter(
                video::Column::FavoriteId
                    .eq(self.id)
                    .and(video::Column::Valid.eq(true))
                    .and(video::Column::DownloadStatus.lt(Status::handled()))
                    .and(video::Column::Category.eq(2))
                    .and(video::Column::SinglePage.is_not_null()),
            )
            .find_with_related(page::Entity)
            .all(connection)
            .await?)
    }

    async fn exist_labels(
        &self,
        videos_info: &[VideoInfo],
        connection: &DatabaseConnection,
    ) -> Result<HashSet<String>> {
        let bvids = videos_info.iter().map(|v| v.bvid().to_string()).collect::<Vec<_>>();
        Ok(video::Entity::find()
            .filter(
                video::Column::FavoriteId
                    .eq(self.id)
                    .and(video::Column::Bvid.is_in(bvids)),
            )
            .select_only()
            .columns([video::Column::Bvid, video::Column::Favtime])
            .into_tuple()
            .all(connection)
            .await?
            .into_iter()
            .map(|(bvid, time)| id_time_key(&bvid, &time))
            .collect::<HashSet<_>>())
    }

    fn video_model_by_info(&self, video_info: &VideoInfo, base_model: Option<video::Model>) -> video::ActiveModel {
        let mut video_model = video_info.to_model(base_model);
        video_model.favorite_id = Set(Some(self.id));
        if let Some(fmt_args) = &video_info.to_fmt_args() {
            video_model.path = Set(Path::new(&self.path)
                .join(filenamify(
                    TEMPLATE
                        .render("video", fmt_args)
                        .unwrap_or_else(|_| video_info.bvid().to_string()),
                ))
                .to_string_lossy()
                .to_string());
        }
        video_model
    }

    async fn fetch_videos_detail(
        &self,
        bili_clent: &BiliClient,
        videos_model: Vec<video::Model>,
        connection: &DatabaseConnection,
    ) -> Result<()> {
        for video_model in videos_model {
            let video = Video::new(bili_clent, video_model.bvid.clone());
            let info: Result<_> = async { Ok((video.get_tags().await?, video.get_pages().await?)) }.await;
            match info {
                Ok((tags, pages_info)) => {
                    let txn = connection.begin().await?;
                    // 将分页信息写入数据库
                    create_video_pages(&pages_info, &video_model, &txn).await?;
                    // 将页标记和 tag 写入数据库
                    let mut video_active_model: video::ActiveModel = video_model.into();
                    video_active_model.single_page = Set(Some(pages_info.len() == 1));
                    video_active_model.tags = Set(Some(serde_json::to_value(tags).unwrap()));
                    video_active_model.save(&txn).await?;
                    txn.commit().await?;
                }
                Err(e) => {
                    error!(
                        "获取视频 {} - {} 的详细信息失败，错误为：{}",
                        &video_model.bvid, &video_model.name, e
                    );
                    if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                        let mut video_active_model: video::ActiveModel = video_model.into();
                        video_active_model.valid = Set(false);
                        video_active_model.save(connection).await?;
                    }
                    continue;
                }
            };
        }
        Ok(())
    }

    fn log_fetch_video_start(&self) {
        info!("开始获取收藏夹 {} - {} 的视频与分页信息...", self.f_id, self.name);
    }

    fn log_fetch_video_end(&self) {
        info!("获取收藏夹 {} - {} 的视频与分页信息完成", self.f_id, self.name);
    }

    fn log_download_video_start(&self) {
        info!("开始下载收藏夹: {} - {} 中所有未处理过的视频...", self.f_id, self.name);
    }

    fn log_download_video_end(&self) {
        info!("下载收藏夹: {} - {} 中未处理过的视频完成", self.f_id, self.name);
    }

    fn log_refresh_video_start(&self) {
        info!("开始扫描收藏夹: {} - {} 的新视频...", self.f_id, self.name);
    }

    fn log_refresh_video_end(&self, got_count: usize, new_count: u64) {
        info!(
            "扫描收藏夹: {} - {} 的新视频完成，获取了 {} 条新视频，其中有 {} 条新视频",
            self.f_id, self.name, got_count, new_count
        );
    }
}
