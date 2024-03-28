use std::collections::HashSet;
use std::path::Path;

use entity::*;
use migration::OnConflict;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::QuerySelect;

use crate::bilibili::{FavoriteListInfo, PageInfo, VideoInfo};
use crate::Result;

/// 根据获得的收藏夹信息，插入或更新数据库中的收藏夹，并返回收藏夹对象
pub async fn handle_favorite_info(
    info: &FavoriteListInfo,
    connection: &DatabaseConnection,
) -> Result<favorite::Model> {
    favorite::Entity::insert(favorite::ActiveModel {
        f_id: Set(info.id),
        name: Set(info.title.to_string()),
        path: Set("/home/amtoaer/Documents/code/rust/bili-sync/video".to_string()),
        enabled: Set(true),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(favorite::Column::FId)
            .update_columns([
                favorite::Column::Name,
                favorite::Column::Path,
                favorite::Column::Enabled,
            ])
            .to_owned(),
    )
    .exec(connection)
    .await?;
    Ok(favorite::Entity::find()
        .filter(favorite::Column::FId.eq(info.id))
        .one(connection)
        .await?
        .unwrap())
}

/// 获取数据库中存在的与该视频 favorite_id 和 bvid 重合的视频中的 bvid 和 favtime
/// 如果 bvid 和 favtime 均相同，说明到达了上次处理到的位置
pub async fn exist_labels(
    videos_info: &[VideoInfo],
    favorite_model: &favorite::Model,
    connection: &DatabaseConnection,
) -> Result<HashSet<(String, DateTime)>> {
    let bvids = videos_info
        .iter()
        .map(|v| v.bvid.clone())
        .collect::<Vec<String>>();
    let exist_labels = video::Entity::find()
        .filter(
            video::Column::FavoriteId
                .eq(favorite_model.id)
                .and(video::Column::Bvid.is_in(bvids)),
        )
        .select_only()
        .columns([video::Column::Bvid, video::Column::Favtime])
        .into_tuple()
        .all(connection)
        .await?
        .into_iter()
        .collect::<HashSet<(String, DateTime)>>();
    Ok(exist_labels)
}

/// 尝试创建 Video Model，如果发生冲突则忽略
pub async fn create_videos(
    videos_info: &[VideoInfo],
    favorite: &favorite::Model,
    connection: &DatabaseConnection,
) -> Result<()> {
    let video_models = videos_info
        .iter()
        .map(move |v| video::ActiveModel {
            favorite_id: Set(favorite.id),
            bvid: Set(v.bvid.clone()),
            path: Set(Path::new(favorite.path.as_str())
                .join(&v.title)
                .to_str()
                .unwrap()
                .to_string()),
            name: Set(v.title.clone()),
            category: Set(v.vtype),
            intro: Set(v.intro.clone()),
            cover: Set(v.cover.clone()),
            ctime: Set(v.ctime.naive_utc()),
            pubtime: Set(v.pubtime.naive_utc()),
            favtime: Set(v.fav_time.naive_utc()),
            handled: Set(false),
            valid: Set(v.attr == 0),
            tags: Set(None),
            single_page: Set(None),
            upper_id: Set(v.upper.mid),
            upper_name: Set(v.upper.name.clone()),
            upper_face: Set(v.upper.face.clone()),
            ..Default::default()
        })
        .collect::<Vec<video::ActiveModel>>();
    video::Entity::insert_many(video_models)
        .on_conflict(
            OnConflict::columns([video::Column::FavoriteId, video::Column::Bvid])
                .do_nothing()
                .to_owned(),
        )
        .do_nothing()
        .exec(connection)
        .await?;
    Ok(())
}

/// 筛选所有符合条件的视频
pub async fn filter_videos(
    videos_info: &[VideoInfo],
    favorite_model: &favorite::Model,
    only_unhandled: bool,
    only_no_page: bool,
    connection: &DatabaseConnection,
) -> Result<Vec<video::Model>> {
    let bvids = videos_info
        .iter()
        .map(|v| v.bvid.clone())
        .collect::<Vec<String>>();
    let mut condition = video::Column::FavoriteId
        .eq(favorite_model.id)
        .and(video::Column::Bvid.is_in(bvids))
        .and(video::Column::Valid.eq(true));
    if only_unhandled {
        condition = condition.and(video::Column::Handled.eq(false));
    }
    if only_no_page {
        condition = condition.and(video::Column::SinglePage.is_null());
    }
    Ok(video::Entity::find()
        .filter(condition)
        .all(connection)
        .await?)
}
/// 创建视频的所有分 P
pub async fn create_video_pages(
    pages_info: &[PageInfo],
    video_model: &video::Model,
    connection: &DatabaseConnection,
) -> Result<()> {
    let page_models = pages_info
        .iter()
        .map(move |p| page::ActiveModel {
            video_id: Set(video_model.id),
            cid: Set(p.cid),
            pid: Set(p.page),
            name: Set(p.name.clone()),
            path: Set(Path::new(video_model.path.as_str())
                .join(&p.name)
                .to_str()
                .unwrap()
                .to_string()),
            image: Set(p.first_frame.clone()),
            valid: Set(video_model.valid),
            download_status: Set(0),
            ..Default::default()
        })
        .collect::<Vec<page::ActiveModel>>();
    page::Entity::insert_many(page_models)
        .on_conflict(
            OnConflict::columns([page::Column::VideoId, page::Column::Pid])
                .do_nothing()
                .to_owned(),
        )
        .do_nothing()
        .exec(connection)
        .await?;
    Ok(())
}

/// 获取所有未处理的视频和页
pub async fn unhandled_videos_pages(
    favorite_model: &favorite::Model,
    connection: &DatabaseConnection,
) -> Result<Vec<(video::Model, Vec<page::Model>)>> {
    Ok(video::Entity::find()
        .filter(
            video::Column::FavoriteId
                .eq(favorite_model.id)
                .and(video::Column::Valid.eq(true))
                .and(video::Column::Handled.eq(false))
                .and(video::Column::SinglePage.is_not_null()),
        )
        .find_with_related(page::Entity)
        .all(connection)
        .await?)
}
