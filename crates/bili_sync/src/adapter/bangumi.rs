use std::path::Path;
use std::pin::Pin;
use std::path::PathBuf;

use anyhow::{Result, Context};
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::Stream;
use sea_orm::prelude::*;
use sea_orm::{
    ColumnTrait, EntityTrait, QueryFilter, QueryOrder, ActiveValue::Set
};
use sea_orm::sea_query::Expr;
use tracing::info;

use bili_sync_entity::VideoSourceTrait;
use sea_orm::sea_query::SimpleExpr;

use crate::bilibili::{BiliClient, VideoInfo};
use crate::bilibili::bangumi::Bangumi;
use crate::adapter::VideoSource;
use crate::config::BangumiConfig;

#[derive(Clone)]
pub struct BangumiSource {
    pub id: i32,
    pub name: String,
    pub latest_row_at: NaiveDateTime,
    pub season_id: Option<String>,
    pub media_id: Option<String>,
    pub ep_id: Option<String>,
    pub path: PathBuf,
    pub download_all_seasons: bool,
}

impl BangumiSource {
    #[allow(dead_code)]
    pub async fn load_all(
        conn: &DatabaseConnection,
    ) -> Result<Vec<Self>> {
        let models = bili_sync_entity::video_source::Entity::find()
            .filter(bili_sync_entity::video_source::Column::Type.eq(1))
            .order_by_asc(bili_sync_entity::video_source::Column::Id)
            .all(conn)
            .await?;
        let sources = models
            .into_iter()
            .map(|m| Self {
                id: m.id,
                name: m.name,
                latest_row_at: m.latest_row_at,
                season_id: m.season_id,
                media_id: m.media_id,
                ep_id: m.ep_id,
                path: m.path.into(),
                download_all_seasons: m.download_all_seasons.unwrap_or(false),
            })
            .collect();
        Ok(sources)
    }

    pub async fn video_stream_from(
        &self,
        bili_client: &BiliClient,
        _path: &Path,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<VideoInfo>> + Send>>> {
        let bangumi = Bangumi::new(bili_client, self.media_id.clone(), self.season_id.clone(), self.ep_id.clone());
        
        if self.download_all_seasons {
            info!("正在获取所有季度的番剧内容");
            Ok(Box::pin(bangumi.into_all_seasons_video_stream()))
        } else {
            info!("仅获取当前季度的番剧内容");
            Ok(Box::pin(bangumi.into_video_stream()))
        }
    }
    
    // 初始化番剧源到数据库
    pub async fn init_to_db(
        bangumi_config: &BangumiConfig,
        conn: &DatabaseConnection,
    ) -> Result<i32> {
        let name = if let Some(media_id) = &bangumi_config.media_id {
            format!("番剧媒体：{}", media_id)
        } else if let Some(ep_id) = &bangumi_config.ep_id {
            format!("番剧剧集：{}", ep_id)
        } else if let Some(season_id) = &bangumi_config.season_id {
            format!("番剧季度：{}", season_id)
        } else {
            "未命名番剧".to_string()
        };
        
        // 查询是否已存在相同配置的番剧记录
        let source = bili_sync_entity::video_source::Entity::find()
            .filter(
                bili_sync_entity::video_source::Column::Type.eq(1)
                    .and(match (&bangumi_config.media_id, &bangumi_config.season_id, &bangumi_config.ep_id) {
                        (Some(media_id), _, _) => {
                            bili_sync_entity::video_source::Column::MediaId.eq(media_id.clone()).into()
                        }
                        (_, Some(season_id), _) => {
                            bili_sync_entity::video_source::Column::SeasonId.eq(season_id.clone()).into()
                        }
                        (_, _, Some(ep_id)) => {
                            bili_sync_entity::video_source::Column::EpId.eq(ep_id.clone()).into()
                        }
                        _ => Expr::val(false).into(),
                    })
            )
            .one(conn)
            .await?;
        
        if let Some(source) = source {
            // 更新现有记录
            let mut updates = Vec::new();
            
            if source.path != bangumi_config.path.to_string_lossy().to_string() {
                updates.push(bili_sync_entity::video_source::ActiveModel {
                    id: Set(source.id),
                    path: Set(bangumi_config.path.to_string_lossy().to_string()),
                    ..Default::default()
                });
            }

            if source.download_all_seasons != Some(bangumi_config.download_all_seasons) {
                updates.push(bili_sync_entity::video_source::ActiveModel {
                    id: Set(source.id),
                    download_all_seasons: Set(Some(bangumi_config.download_all_seasons)),
                    ..Default::default()
                });
            }
            
            for update in updates {
                bili_sync_entity::video_source::Entity::update(update)
                    .exec(conn)
                    .await
                    .context("Failed to update bangumi source")?;
            }
            
            return Ok(source.id);
        }
        
        // 创建新记录
        let now = Utc::now().naive_utc();
        let new_source = bili_sync_entity::video_source::ActiveModel {
            name: Set(name),
            r#type: Set(1), // 1 表示番剧类型
            latest_row_at: Set(now),
            season_id: Set(bangumi_config.season_id.clone()),
            media_id: Set(bangumi_config.media_id.clone()),
            ep_id: Set(bangumi_config.ep_id.clone()),
            path: Set(bangumi_config.path.to_string_lossy().to_string()),
            download_all_seasons: Set(Some(bangumi_config.download_all_seasons)),
            ..Default::default()
        };
        
        let result = bili_sync_entity::video_source::Entity::insert(new_source)
            .exec(conn)
            .await
            .context("Failed to insert bangumi source")?;
        
        Ok(result.last_insert_id)
    }
}

impl VideoSourceTrait for BangumiSource {
    fn get_latest_row_at(&self) -> NaiveDateTime {
        self.latest_row_at
    }

    fn log_refresh_video_start(&self) {
        info!("开始获取番剧 {} 的更新", self.name);
    }

    fn log_refresh_video_end(&self, count: usize) {
        info!("番剧 {} 获取更新完毕，新增 {} 个视频", self.name, count);
    }

    fn log_fetch_video_start(&self) {
        info!("开始获取番剧 {} 的详细信息", self.name);
    }

    fn log_fetch_video_end(&self) {
        info!("番剧 {} 的详细信息获取完毕", self.name);
    }

    fn log_download_video_start(&self) {
        info!("开始下载番剧 {} 的视频", self.name);
    }

    fn log_download_video_end(&self) {
        info!("番剧 {} 的视频下载完毕", self.name);
    }

    fn filter_expr(&self) -> SimpleExpr {
        bili_sync_entity::video::Column::SourceId
            .eq(self.id)
            .and(bili_sync_entity::video::Column::SourceType.eq(1))
    }

    fn should_take(&self, _release_datetime: &DateTime<Utc>, _latest_row_at: &DateTime<Utc>) -> bool {
        true
    }

    fn update_latest_row_at(
        &self,
        latest_row_at: NaiveDateTime,
    ) -> bili_sync_entity::video_source::ActiveModel {
        let mut model = <bili_sync_entity::video_source::ActiveModel as sea_orm::ActiveModelTrait>::default();
        model.id = Set(self.id);
        model.latest_row_at = Set(latest_row_at);
        model
    }

    fn set_relation_id(&self, model: &mut bili_sync_entity::video::ActiveModel) {
        model.source_id = Set(Some(self.id));
        model.source_type = Set(Some(1));
    }
}

impl VideoSource for BangumiSource {
    fn filter_expr(&self) -> SimpleExpr {
        bili_sync_entity::video::Column::SourceId
            .eq(self.id)
            .and(bili_sync_entity::video::Column::SourceType.eq(1))
    }

    fn set_relation_id(&self, model: &mut bili_sync_entity::video::ActiveModel) {
        model.source_id = Set(Some(self.id));
        model.source_type = Set(Some(1));
    }

    fn get_latest_row_at(&self) -> NaiveDateTime {
        self.latest_row_at
    }

    fn update_latest_row_at(&self, datetime: NaiveDateTime) -> crate::adapter::_ActiveModel {
        let mut model = <bili_sync_entity::video_source::ActiveModel as sea_orm::ActiveModelTrait>::default();
        model.id = Set(self.id);
        model.latest_row_at = Set(datetime);
        crate::adapter::_ActiveModel::Bangumi(model)
    }

    fn path(&self) -> &Path {
        &self.path
    }
    
    // 总是返回true，表示应该下载所有番剧内容，不管发布时间
    fn should_take(&self, _release_datetime: &chrono::DateTime<Utc>, _latest_row_at: &chrono::DateTime<Utc>) -> bool {
        true
    }

    fn log_refresh_video_start(&self) {
        info!("开始获取番剧 {} 的更新", self.name);
    }

    fn log_refresh_video_end(&self, count: usize) {
        info!("番剧 {} 获取更新完毕，新增 {} 个视频", self.name, count);
    }

    fn log_fetch_video_start(&self) {
        info!("开始获取番剧 {} 的详细信息", self.name);
    }

    fn log_fetch_video_end(&self) {
        info!("番剧 {} 的详细信息获取完毕", self.name);
    }

    fn log_download_video_start(&self) {
        info!("开始下载番剧 {} 的视频", self.name);
    }

    fn log_download_video_end(&self) {
        info!("番剧 {} 的视频下载完毕", self.name);
    }
} 