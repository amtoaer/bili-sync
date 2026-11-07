use anyhow::{Context, Result, anyhow};
use bili_sync_entity::*;
use itertools::Itertools;
use rand::seq::SliceRandom;
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{Expr, OnConflict, SimpleExpr};
use sea_orm::{ConnectionTrait, DatabaseTransaction, IdenStatic, Statement};

use crate::adapter::{VideoSource, VideoSourceEnum};
use crate::bilibili::VideoInfo;
use crate::config::Config;
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
                .and(video::Column::ShouldDownload.eq(true))
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
pub async fn create_pages(pages_model: Vec<page::ActiveModel>, connection: &DatabaseTransaction) -> Result<()> {
    let mut pages = pages_model.into_iter();
    loop {
        // 这里 insert_many 要求 IntoIterator，vec 上调用 chunks 返回的类型不匹配，需要 to_vec 做 clone
        // itertools 的 into_iter().chunks() 由于 !Send 也无法直接使用
        // 暂时手写 take + collect 作为避免 clone 的折中方案
        let page_chunk = pages.by_ref().take(200).collect::<Vec<_>>();
        if page_chunk.is_empty() {
            break;
        }
        page::Entity::insert_many(page_chunk)
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

/// 更新视频 model 的详情字段
pub async fn update_video_detail_models(
    videos: Vec<video::ActiveModel>,
    connection: &DatabaseTransaction,
) -> Result<()> {
    if videos.is_empty() {
        return Ok(());
    }
    let columns = [
        video::Column::Id,
        video::Column::CollectionId,
        video::Column::FavoriteId,
        video::Column::WatchLaterId,
        video::Column::SubmissionId,
        video::Column::UpperId,
        video::Column::UpperName,
        video::Column::UpperFace,
        video::Column::Staff,
        video::Column::Name,
        video::Column::Bvid,
        video::Column::EpId,
        video::Column::Intro,
        video::Column::Cover,
        video::Column::Ctime,
        video::Column::Pubtime,
        video::Column::Favtime,
        video::Column::DownloadStatus,
        video::Column::Valid,
        video::Column::ShouldDownload,
        video::Column::Tags,
        video::Column::SinglePage,
    ];
    let row = format!("({})", std::iter::repeat_n("?", columns.len()).join(", "));
    let rows = std::iter::repeat_n(row.as_str(), videos.len()).join(", ");
    let mut values = Vec::with_capacity(videos.len() * columns.len());
    for video in videos {
        for column in columns {
            values.push(
                video
                    .get(column)
                    .into_value()
                    .ok_or_else(|| anyhow!("video column {} is not set", column.as_str()))?,
            );
        }
    }
    let sql = format!(
        "WITH tempdata({}) AS (VALUES {}) \
        UPDATE video \
        SET {} \
        FROM tempdata \
        WHERE video.id = tempdata.id",
        columns.iter().map(IdenStatic::as_str).join(", "),
        rows,
        columns
            .iter()
            .skip(1)
            .map(|column| {
                let column = column.as_str();
                format!("{} = tempdata.{}", column, column)
            })
            .join(", ")
    );
    connection
        .execute(Statement::from_sql_and_values(
            connection.get_database_backend(),
            sql,
            values,
        ))
        .await?;
    Ok(())
}

/// 将视频标记为失效
pub async fn set_video_models_invalid(video_ids: Vec<i32>, connection: &DatabaseTransaction) -> Result<()> {
    if video_ids.is_empty() {
        return Ok(());
    }
    video::Entity::update_many()
        .filter(video::Column::Id.is_in(video_ids))
        .col_expr(video::Column::Valid, Expr::value(false))
        .exec(connection)
        .await?;
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

/// 更新视频页 model 的下载状态、路径和弹幕同步时间
pub async fn update_pages_model(pages: Vec<page::ActiveModel>, connection: &DatabaseConnection) -> Result<()> {
    let query = page::Entity::insert_many(pages).on_conflict(
        OnConflict::column(page::Column::Id)
            .update_columns([
                page::Column::DownloadStatus,
                page::Column::Path,
                page::Column::DanmakuLastSyncedAt,
            ])
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
    // 此处将视频源随机打乱顺序，从概率上确保每个视频源都有机会优先执行，避免后面视频源的长期饥饿问题
    sources.shuffle(&mut rand::rng());
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

#[cfg(test)]
mod bangumi_tests {
    use super::*;
    use bili_sync_migration::{Migrator, MigratorTrait};
    use sea_orm::{Database, TransactionTrait};
    use serde_json::json;

    fn episode() -> VideoInfo {
        serde_json::from_value(json!({
            "title": "罗小黑战记 第8话 深山", "bvid": "BV1Rs411f7ZM", "desc": "简介",
            "pic": "https://example.com/cover.jpg", "owner": {"mid": 1, "name": "作者", "face": ""},
            "ctime": 1378868400, "pubdate": 1378868400, "state": 0,
            "is_upower_exclusive": false, "is_upower_play": false,
            "redirect_url": "https://www.bilibili.com/bangumi/play/ep32367",
            "pages": [{"cid": 34474885773_i64, "page": 1, "part": "第8话 深山", "duration": 418}]
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn bangumi_scan_persists_episode_and_deduplicates() -> Result<()> {
        let db = Database::connect("sqlite::memory:").await?;
        Migrator::up(&db, None).await?;
        let source: VideoSourceEnum = collection::ActiveModel {
            s_id: Set(32367),
            m_id: Set(0),
            r#type: Set(4),
            name: Set("罗小黑战记".into()),
            path: Set("downloads".into()),
            enabled: Set(true),
            ..Default::default()
        }
        .insert(&db)
        .await?
        .into();
        assert_eq!(episode().release_datetime().timestamp(), 1378868400);
        for _ in 0..2 {
            create_videos(vec![episode()], &source, &db).await?;
        }
        let videos = filter_unfilled_videos(source.filter_expr(), &db).await?;
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].ep_id, Some(32367));
        assert!(videos[0].valid);
        let mut info = episode();
        let VideoInfo::Detail { pages, .. } = &mut info else {
            unreachable!()
        };
        let pages = std::mem::take(pages)
            .into_iter()
            .map(|p| p.into_active_model(videos[0].id))
            .collect();
        let mut model = info.into_detail_model(videos[0].clone(), false);
        model.single_page = Set(Some(true));
        model.should_download = Set(true);
        let txn = db.begin().await?;
        create_pages(pages, &txn).await?;
        update_video_detail_models(vec![model], &txn).await?;
        txn.commit().await?;
        let pending = filter_unhandled_video_pages(source.filter_expr(), &db).await?;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0.ep_id, Some(32367));
        assert_eq!(pending[0].1[0].cid, 34474885773);
        assert_eq!(pending[0].1[0].duration, 418);
        assert!(filter_unfilled_videos(source.filter_expr(), &db).await?.is_empty());
        // The nullable migration preserves ordinary video behavior.
        let mut ordinary = episode();
        if let VideoInfo::Detail { redirect_url, .. } = &mut ordinary {
            *redirect_url = None;
        }
        let ordinary = ordinary.into_detail_model(Default::default(), false);
        assert_eq!(ordinary.ep_id, Set(None));
        assert_eq!(ordinary.valid, Set(true));
        db.close().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires Bilibili network access; fetches metadata and stream URLs only"]
    async fn live_bangumi_ep32367() -> Result<()> {
        use crate::bilibili::{BestStream, BiliClient, Video};
        let config = Config::default();
        let client = BiliClient::new();
        let video = Video::new(&client, "BV1Rs411f7ZM", &config.credential).with_episode(Some(32367));
        let info = video.get_view_info().await?;
        let VideoInfo::Detail { pages, title, .. } = info else {
            unreachable!()
        };
        assert!(title.contains("深山"));
        assert_eq!(pages.len(), 1);
        let stream = video
            .get_page_analyzer(&pages[0])
            .await?
            .best_stream(&config.filter_option)?;
        assert!(matches!(stream, BestStream::VideoAudio { audio: Some(_), .. }));
        Ok(())
    }
}
