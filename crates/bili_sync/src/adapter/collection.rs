use std::path::Path;
use std::pin::Pin;

use anyhow::{Context, Result};
use bili_sync_entity::*;
use chrono::Utc;
use futures::Stream;
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{OnConflict, SimpleExpr};
use sea_orm::{DatabaseConnection, Unchanged};

use crate::adapter::{_ActiveModel, VideoSource, VideoSourceEnum};
use crate::bilibili::{BiliClient, Collection, CollectionItem, CollectionType, VideoInfo};

impl VideoSource for collection::Model {
    fn filter_expr(&self) -> SimpleExpr {
        video::Column::CollectionId.eq(self.id)
    }

    fn set_relation_id(&self, video_model: &mut video::ActiveModel) {
        video_model.collection_id = Set(Some(self.id));
    }

    fn path(&self) -> &Path {
        Path::new(self.path.as_str())
    }

    fn get_latest_row_at(&self) -> DateTime {
        self.latest_row_at
    }

    fn update_latest_row_at(&self, datetime: DateTime) -> _ActiveModel {
        _ActiveModel::Collection(collection::ActiveModel {
            id: Unchanged(self.id),
            latest_row_at: Set(datetime),
            ..Default::default()
        })
    }

    fn should_take(&self, _release_datetime: &chrono::DateTime<Utc>, _latest_row_at: &chrono::DateTime<Utc>) -> bool {
        // collection（视频合集/视频列表）返回的内容似乎并非严格按照时间排序，并且不同 collection 的排序方式也不同
        // 为了保证程序正确性，collection 不根据时间提前 break，而是每次都全量拉取
        true
    }

    fn log_refresh_video_start(&self) {
        info!("开始扫描{}「{}」..", CollectionType::from(self.r#type), self.name);
    }

    fn log_refresh_video_end(&self, count: usize) {
        info!(
            "扫描{}「{}」完成，已拉取 {} 条视频",
            CollectionType::from(self.r#type),
            self.name,
            count,
        );
    }

    fn log_fetch_video_start(&self) {
        info!(
            "开始填充{}「{}」视频详情..",
            CollectionType::from(self.r#type),
            self.name
        );
    }

    fn log_fetch_video_end(&self) {
        info!("填充{}「{}」视频详情完成", CollectionType::from(self.r#type), self.name);
    }

    fn log_download_video_start(&self) {
        info!("开始下载{}「{}」视频..", CollectionType::from(self.r#type), self.name);
    }

    fn log_download_video_end(&self) {
        info!("下载{}「{}」视频完成", CollectionType::from(self.r#type), self.name);
    }
}

// 修改初始化合集源的方法中的类型转换问题
pub async fn init_collection_sources(
    conn: &DatabaseConnection,
    collection_list: &std::collections::HashMap<CollectionItem, std::path::PathBuf>,
) -> Result<()> {
    // 遍历配置中的合集列表
    for (collection_item, path) in collection_list {
        // 检查数据库中是否已存在该合集的记录
        let existing = collection::Entity::find()
            .filter(
                collection::Column::SId
                    .eq(collection_item.sid.clone())
                    .and(collection::Column::MId.eq(collection_item.mid.clone()))
                    .and(collection::Column::Type.eq(Into::<i32>::into(collection_item.collection_type.clone())))
            )
            .one(conn)
            .await?;
        
        if existing.is_none() {
            // 如果不存在，尝试获取合集信息并创建新记录
            let bili_client = crate::bilibili::BiliClient::new(String::new());
            let collection = Collection::new(&bili_client, collection_item);
            
            // 尝试获取合集信息
            match collection.get_info().await {
                Ok(collection_info) => {
                    // collection_info.sid 和 collection_info.mid 已经是 i64 类型
                    let model = collection::ActiveModel {
                        id: Set(Default::default()),
                        s_id: Set(collection_info.sid), // 已经是 i64 类型
                        m_id: Set(collection_info.mid), // 已经是 i64 类型
                        r#type: Set(collection_info.collection_type.into()),
                        name: Set(collection_info.name.clone()),
                        path: Set(path.to_string_lossy().to_string()),
                        latest_row_at: Set(chrono::Utc::now().naive_utc()),
                        ..Default::default()
                    };
                    
                    // 插入数据库
                    let result = collection::Entity::insert(model)
                        .exec(conn)
                        .await
                        .context("Failed to insert collection source")?;
                    
                    info!("初始化合集源: {} (ID: {})", collection_info.name, result.last_insert_id);
                },
                Err(e) => {
                    // 如果获取失败，创建一个临时记录
                    warn!("获取合集 {:?} 信息失败: {}, 创建临时记录", collection_item, e);
                    
                    // 尝试将sid和mid转换为i64
                    let sid_i64 = match collection_item.sid.parse::<i64>() {
                        Ok(id) => id,
                        Err(e) => {
                            warn!("无效的合集ID {}: {}, 跳过", collection_item.sid, e);
                            continue;
                        }
                    };
                    
                    let mid_i64 = match collection_item.mid.parse::<i64>() {
                        Ok(id) => id,
                        Err(e) => {
                            warn!("无效的UP主ID {}: {}, 跳过", collection_item.mid, e);
                            continue;
                        }
                    };
                    
                    let model = collection::ActiveModel {
                        id: Set(Default::default()),
                        s_id: Set(sid_i64), // 转换为i64类型
                        m_id: Set(mid_i64), // 转换为i64类型
                        r#type: Set(collection_item.collection_type.clone().into()),
                        name: Set(format!("合集 {}/{}", collection_item.sid, collection_item.mid)),
                        path: Set(path.to_string_lossy().to_string()),
                        latest_row_at: Set(chrono::Utc::now().naive_utc()),
                        ..Default::default()
                    };
                    
                    let result = collection::Entity::insert(model)
                        .exec(conn)
                        .await
                        .context("Failed to insert temporary collection source")?;
                    
                    info!("初始化临时合集源: {:?} (ID: {})", collection_item, result.last_insert_id);
                }
            }
        } else if let Some(existing) = existing {
            // 如果已存在，更新路径
            if existing.path != path.to_string_lossy().to_string() {
                let model = collection::ActiveModel {
                    id: Set(existing.id),
                    path: Set(path.to_string_lossy().to_string()),
                    ..Default::default()
                };
                
                // 更新数据库
                collection::Entity::update(model)
                    .exec(conn)
                    .await
                    .context("Failed to update collection source")?;
                
                info!("更新合集源: {} (ID: {})", existing.name, existing.id);
            }
        }
    }
    
    Ok(())
}

pub(super) async fn collection_from<'a>(
    collection_item: &'a CollectionItem,
    path: &Path,
    bili_client: &'a BiliClient,
    connection: &DatabaseConnection,
) -> Result<(
    VideoSourceEnum,
    Pin<Box<dyn Stream<Item = Result<VideoInfo>> + 'a + Send>>,
)> {
    let collection = Collection::new(bili_client, collection_item);
    let collection_info = collection.get_info().await?;
    collection::Entity::insert(collection::ActiveModel {
        s_id: Set(collection_info.sid),
        m_id: Set(collection_info.mid),
        r#type: Set(collection_info.collection_type.into()),
        name: Set(collection_info.name.clone()),
        path: Set(path.to_string_lossy().to_string()),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::columns([
            collection::Column::SId,
            collection::Column::MId,
            collection::Column::Type,
        ])
        .update_columns([collection::Column::Name, collection::Column::Path])
        .to_owned(),
    )
    .exec(connection)
    .await?;
    Ok((
        collection::Entity::find()
            .filter(
                collection::Column::SId
                    .eq(collection_item.sid.clone())
                    .and(collection::Column::MId.eq(collection_item.mid.clone()))
                    .and(collection::Column::Type.eq(Into::<i32>::into(collection_item.collection_type.clone()))),
            )
            .one(connection)
            .await?
            .context("collection not found")?
            .into(),
        Box::pin(collection.into_video_stream()),
    ))
}
