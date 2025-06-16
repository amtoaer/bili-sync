use anyhow::{Context, Result, anyhow};
use bili_sync_entity::*;
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{OnConflict, SimpleExpr};
use sea_orm::{DatabaseTransaction, TransactionTrait};

use crate::adapter::{VideoSource, VideoSourceEnum};
use crate::bilibili::{PageInfo, VideoInfo};
use crate::config::{Config, LegacyConfig};
use crate::utils::status::STATUS_COMPLETED;

/// 筛选未填充的视频
pub async fn filter_unfilled_videos(
    additional_expr: SimpleExpr,
    conn: &DatabaseConnection,
) -> Result<Vec<video::Model>> {
    video::Entity::find()
        .filter(
            video::Column::Valid
                .eq(true)
                .and(video::Column::DownloadStatus.eq(0))
                .and(video::Column::Category.eq(2))
                .and(video::Column::SinglePage.is_null())
                .and(additional_expr),
        )
        .all(conn)
        .await
        .context("filter unfilled videos failed")
}

/// 筛选未处理完成的视频和视频页
pub async fn filter_unhandled_video_pages(
    additional_expr: SimpleExpr,
    connection: &DatabaseConnection,
) -> Result<Vec<(video::Model, Vec<page::Model>)>> {
    video::Entity::find()
        .filter(
            video::Column::Valid
                .eq(true)
                .and(video::Column::DownloadStatus.lt(STATUS_COMPLETED))
                .and(video::Column::Category.eq(2))
                .and(video::Column::SinglePage.is_not_null())
                .and(additional_expr),
        )
        .find_with_related(page::Entity)
        .all(connection)
        .await
        .context("filter unhandled video pages failed")
}

/// 尝试创建 Video Model，如果发生冲突则忽略
pub async fn create_videos(
    videos_info: Vec<VideoInfo>,
    video_source: &VideoSourceEnum,
    connection: &DatabaseConnection,
) -> Result<()> {
    let video_models = videos_info
        .into_iter()
        .map(|v| {
            let mut model = v.into_simple_model();
            video_source.set_relation_id(&mut model);
            model
        })
        .collect::<Vec<_>>();
    video::Entity::insert_many(video_models)
        // 这里想表达的是 on 索引名，但 sea-orm 的 api 似乎只支持列名而不支持索引名，好在留空可以达到相同的目的
        .on_conflict(OnConflict::new().do_nothing().to_owned())
        .do_nothing()
        .exec(connection)
        .await?;
    Ok(())
}

/// 尝试创建 Page Model，如果发生冲突则忽略
pub async fn create_pages(
    pages_info: Vec<PageInfo>,
    video_model: &bili_sync_entity::video::Model,
    connection: &DatabaseTransaction,
) -> Result<()> {
    let page_models = pages_info
        .into_iter()
        .map(|p| p.into_active_model(video_model))
        .collect::<Vec<page::ActiveModel>>();
    for page_chunk in page_models.chunks(50) {
        page::Entity::insert_many(page_chunk.to_vec())
            .on_conflict(
                OnConflict::columns([page::Column::VideoId, page::Column::Pid])
                    .do_nothing()
                    .to_owned(),
            )
            .do_nothing()
            .exec(connection)
            .await?;
    }
    Ok(())
}

/// 更新视频 model 的下载状态
pub async fn update_videos_model(videos: Vec<video::ActiveModel>, connection: &DatabaseConnection) -> Result<()> {
    video::Entity::insert_many(videos)
        .on_conflict(
            OnConflict::column(video::Column::Id)
                .update_columns([video::Column::DownloadStatus, video::Column::Path])
                .to_owned(),
        )
        .exec(connection)
        .await?;
    Ok(())
}

/// 更新视频页 model 的下载状态
pub async fn update_pages_model(pages: Vec<page::ActiveModel>, connection: &DatabaseConnection) -> Result<()> {
    let query = page::Entity::insert_many(pages).on_conflict(
        OnConflict::column(page::Column::Id)
            .update_columns([page::Column::DownloadStatus, page::Column::Path])
            .to_owned(),
    );
    query.exec(connection).await?;
    Ok(())
}

/// 获取所有已经启用的视频源
pub async fn get_enabled_video_sources(connection: &DatabaseConnection) -> Result<Vec<VideoSourceEnum>> {
    let (favorite, watch_later, submission, collection) = tokio::try_join!(
        favorite::Entity::find()
            .filter(favorite::Column::Enabled.eq(true))
            .all(connection),
        watch_later::Entity::find()
            .filter(watch_later::Column::Enabled.eq(true))
            .all(connection),
        submission::Entity::find()
            .filter(submission::Column::Enabled.eq(true))
            .all(connection),
        collection::Entity::find()
            .filter(collection::Column::Enabled.eq(true))
            .all(connection),
    )?;
    let mut sources = Vec::with_capacity(favorite.len() + watch_later.len() + submission.len() + collection.len());
    sources.extend(favorite.into_iter().map(VideoSourceEnum::from));
    sources.extend(watch_later.into_iter().map(VideoSourceEnum::from));
    sources.extend(submission.into_iter().map(VideoSourceEnum::from));
    sources.extend(collection.into_iter().map(VideoSourceEnum::from));
    Ok(sources)
}

/// 从数据库中加载配置
pub async fn load_db_config(connection: &DatabaseConnection) -> Result<Option<Result<Config>>> {
    Ok(bili_sync_entity::config::Entity::find_by_id(1)
        .one(connection)
        .await?
        .map(|model| {
            serde_json::from_str(&model.data).map_err(|e| anyhow!("Failed to deserialize config data: {}", e))
        }))
}

/// 保存配置到数据库
pub async fn save_db_config(config: &Config, connection: &DatabaseConnection) -> Result<()> {
    let data = serde_json::to_string(config).context("Failed to serialize config data")?;
    let model = bili_sync_entity::config::ActiveModel {
        id: Set(1),
        data: Set(data),
        ..Default::default()
    };
    bili_sync_entity::config::Entity::insert(model)
        .on_conflict(
            OnConflict::column(bili_sync_entity::config::Column::Id)
                .update_column(bili_sync_entity::config::Column::Data)
                .to_owned(),
        )
        .exec(connection)
        .await
        .context("Failed to save config to database")?;
    Ok(())
}

/// 迁移旧版本配置（即将所有相关联的内容设置为 enabled）
pub async fn migrate_legacy_config(config: &LegacyConfig, connection: &DatabaseConnection) -> Result<()> {
    let transaction = connection.begin().await.context("Failed to begin transaction")?;
    tokio::try_join!(
        migrate_favorite(config, &transaction),
        migrate_watch_later(config, &transaction),
        migrate_submission(config, &transaction),
        migrate_collection(config, &transaction)
    )?;
    transaction.commit().await.context("Failed to commit transaction")?;
    Ok(())
}

async fn migrate_favorite(config: &LegacyConfig, connection: &DatabaseTransaction) -> Result<()> {
    favorite::Entity::update_many()
        .filter(favorite::Column::FId.is_in(config.favorite_list.keys().collect::<Vec<_>>()))
        .col_expr(favorite::Column::Enabled, Expr::value(true))
        .exec(connection)
        .await
        .context("Failed to migrate favorite config")?;
    Ok(())
}

async fn migrate_watch_later(config: &LegacyConfig, connection: &DatabaseTransaction) -> Result<()> {
    if config.watch_later.enabled {
        watch_later::Entity::update_many()
            .col_expr(watch_later::Column::Enabled, Expr::value(true))
            .exec(connection)
            .await
            .context("Failed to migrate watch later config")?;
    }
    Ok(())
}

async fn migrate_submission(config: &LegacyConfig, connection: &DatabaseTransaction) -> Result<()> {
    submission::Entity::update_many()
        .filter(submission::Column::UpperId.is_in(config.submission_list.keys().collect::<Vec<_>>()))
        .col_expr(submission::Column::Enabled, Expr::value(true))
        .exec(connection)
        .await
        .context("Failed to migrate submission config")?;
    Ok(())
}

async fn migrate_collection(config: &LegacyConfig, connection: &DatabaseTransaction) -> Result<()> {
    let tuples: Vec<(i64, i64, i32)> = config
        .collection_list
        .keys()
        .filter_map(|key| Some((key.sid.parse().ok()?, key.mid.parse().ok()?, key.collection_type.into())))
        .collect();
    collection::Entity::update_many()
        .filter(
            Expr::tuple([
                Expr::column(collection::Column::SId),
                Expr::column(collection::Column::MId),
                Expr::column(collection::Column::Type),
            ])
            .in_tuples(tuples),
        )
        .col_expr(collection::Column::Enabled, Expr::value(true))
        .exec(connection)
        .await
        .context("Failed to migrate collection config")?;
    Ok(())
}
