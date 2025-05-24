use std::path::Path;
use std::pin::Pin;
use std::collections::HashMap;

use anyhow::{Context, Result};
use bili_sync_entity::*;
use futures::Stream;
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{OnConflict, SimpleExpr};
use sea_orm::{DatabaseConnection, Unchanged};
use chrono::Utc;

use crate::adapter::{_ActiveModel, VideoSource, VideoSourceEnum};
use crate::bilibili::{BiliClient, FavoriteList, VideoInfo};

impl VideoSource for favorite::Model {
    fn filter_expr(&self) -> SimpleExpr {
        video::Column::FavoriteId.eq(self.id)
    }

    fn set_relation_id(&self, video_model: &mut video::ActiveModel) {
        video_model.favorite_id = Set(Some(self.id));
    }

    fn path(&self) -> &Path {
        Path::new(self.path.as_str())
    }

    fn get_latest_row_at(&self) -> DateTime {
        self.latest_row_at
    }

    fn update_latest_row_at(&self, datetime: DateTime) -> _ActiveModel {
        _ActiveModel::Favorite(favorite::ActiveModel {
            id: Unchanged(self.id),
            latest_row_at: Set(datetime),
            ..Default::default()
        })
    }

    fn should_take(&self, _release_datetime: &chrono::DateTime<Utc>, _latest_row_at: &chrono::DateTime<Utc>) -> bool {
        true
    }

    fn log_refresh_video_start(&self) {
        info!("开始扫描收藏夹「{}」..", self.name);
    }

    fn log_refresh_video_end(&self, count: usize) {
        info!("扫描收藏夹「{}」完成，获取到 {} 条新视频", self.name, count);
    }

    fn log_fetch_video_start(&self) {
        info!("开始填充收藏夹「{}」视频详情..", self.name);
    }

    fn log_fetch_video_end(&self) {
        info!("填充收藏夹「{}」视频详情完成", self.name);
    }

    fn log_download_video_start(&self) {
        info!("开始下载收藏夹「{}」视频..", self.name);
    }

    fn log_download_video_end(&self) {
        info!("下载收藏夹「{}」视频完成", self.name);
    }
}

pub async fn init_favorite_sources(
    conn: &DatabaseConnection,
    favorite_list: &HashMap<String, std::path::PathBuf>,
) -> Result<()> {
    for (fid, path) in favorite_list {
        let fid_i64 = match fid.parse::<i64>() {
            Ok(id) => id,
            Err(e) => {
                warn!("无效的收藏夹ID {}: {}, 跳过", fid, e);
                continue;
            }
        };
        
        let existing = favorite::Entity::find()
            .filter(favorite::Column::FId.eq(fid_i64))
            .one(conn)
            .await?;
        
        if existing.is_none() {
            let bili_client = crate::bilibili::BiliClient::new(String::new());
            let favorite = FavoriteList::new(&bili_client, fid.to_owned());
            
            match favorite.get_info().await {
                Ok(favorite_info) => {
                    let model = favorite::ActiveModel {
                        id: Set(Default::default()),
                        f_id: Set(favorite_info.id),
                        name: Set(favorite_info.title.clone()),
                        path: Set(path.to_string_lossy().to_string()),
                        created_at: Set(chrono::Utc::now().to_string()),
                        latest_row_at: Set(chrono::Utc::now().naive_utc()),
                        ..Default::default()
                    };
                    
                    let result = favorite::Entity::insert(model)
                        .exec(conn)
                        .await
                        .context("Failed to insert favorite source")?;
                    
                    info!("初始化收藏夹源: {} (ID: {})", favorite_info.title, result.last_insert_id);
                },
                Err(e) => {
                    warn!("获取收藏夹 {} 信息失败: {}, 创建临时记录", fid, e);
                    
                    let model = favorite::ActiveModel {
                        id: Set(Default::default()),
                        f_id: Set(fid_i64),
                        name: Set(format!("收藏夹 {}", fid)),
                        path: Set(path.to_string_lossy().to_string()),
                        created_at: Set(chrono::Utc::now().to_string()),
                        latest_row_at: Set(chrono::Utc::now().naive_utc()),
                        ..Default::default()
                    };
                    
                    let result = favorite::Entity::insert(model)
                        .exec(conn)
                        .await
                        .context("Failed to insert temporary favorite source")?;
                    
                    info!("初始化临时收藏夹源: {} (ID: {})", fid, result.last_insert_id);
                }
            }
        } else if let Some(existing) = existing {
            if existing.path != path.to_string_lossy().to_string() {
                let model = favorite::ActiveModel {
                    id: Set(existing.id),
                    path: Set(path.to_string_lossy().to_string()),
                    ..Default::default()
                };
                
                favorite::Entity::update(model)
                    .exec(conn)
                    .await
                    .context("Failed to update favorite source")?;
                
                info!("更新收藏夹源: {} (ID: {})", existing.name, existing.id);
            }
        }
    }
    
    Ok(())
}

pub(super) async fn favorite_from<'a>(
    fid: &str,
    path: &Path,
    bili_client: &'a BiliClient,
    connection: &DatabaseConnection,
) -> Result<(
    VideoSourceEnum,
    Pin<Box<dyn Stream<Item = Result<VideoInfo>> + 'a + Send>>,
)> {
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
        favorite::Entity::find()
            .filter(favorite::Column::FId.eq(favorite_info.id))
            .one(connection)
            .await?
            .context("favorite not found")?
            .into(),
        Box::pin(favorite.into_video_stream()),
    ))
}
