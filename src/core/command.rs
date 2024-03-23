use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use entity::*;
use futures_util::{pin_mut, StreamExt};
use migration::OnConflict;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::QuerySelect;

use crate::bilibili::{BiliClient, FavoriteList, FavoriteListInfo, VideoInfo};
use crate::Result;

pub async fn process_favorite(
    bili_client: Arc<BiliClient>,
    fid: i32,
    connection: Arc<DatabaseConnection>,
) -> Result<()> {
    let favorite_list = FavoriteList::new(bili_client.clone(), fid.to_string());
    let info = favorite_list.get_info().await?;
    let favorite_obj = handle_favorite_info(&info, connection.as_ref()).await?;
    println!(
        "Hi there! I'm going to scan this favorite: {:?}",
        favorite_obj
    );
    let video_stream = favorite_list.into_video_stream().chunks(10);
    pin_mut!(video_stream);
    while let Some(videos_info) = video_stream.next().await {
        let exist_bvids_pubtimes =
            exists_bvids_favtime(&videos_info, fid, connection.as_ref()).await?;
        let video_info_to_create = videos_info
            .iter()
            .filter(|v| !exist_bvids_pubtimes.contains(&(v.bvid.clone(), v.fav_time.to_string())))
            .collect::<Vec<&VideoInfo>>();
        let len = video_info_to_create.len();
        if !video_info_to_create.is_empty() {
            create_videos(video_info_to_create, &favorite_obj, connection.as_ref()).await?;
        }
        if videos_info.len() != len {
            break;
        }
    }
    Ok(())
}

// 根据获得的收藏夹信息，插入或更新数据库中的收藏夹，并返回收藏夹对象
async fn handle_favorite_info(
    info: &FavoriteListInfo,
    connection: &DatabaseConnection,
) -> Result<favorite::Model> {
    Ok(favorite::Entity::insert(favorite::ActiveModel {
        f_id: Set(info.id as i32),
        name: Set(info.title.to_string()),
        path: Set("/home/amtoaer/Documents/code/rust/bili-sync/video".to_string()),
        enabled: Set(true),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(favorite::Column::FId)
            .update_column(favorite::Column::Name)
            .update_column(favorite::Column::Path)
            .update_column(favorite::Column::Enabled)
            .to_owned(),
    )
    .exec_with_returning(connection)
    .await?)
}

// 获取数据库中存在的与该视频 favorite_id 和 bvid 重合的视频
async fn exists_bvids_favtime(
    videos_info: &[VideoInfo],
    fid: i32,
    connection: &DatabaseConnection,
) -> Result<HashSet<(String, String)>> {
    let bvids = videos_info
        .iter()
        .map(|v| v.bvid.clone())
        .collect::<Vec<String>>();
    let exist_bvid_favtime = video::Entity::find()
        .filter(
            video::Column::FavoriteId
                .eq(fid)
                .and(video::Column::Bvid.is_in(bvids)),
        )
        .select_only()
        .columns([video::Column::Bvid, video::Column::Favtime])
        .all(connection)
        .await?
        .into_iter()
        .map(|v| (v.bvid, v.favtime))
        .collect::<HashSet<(String, String)>>();
    Ok(exist_bvid_favtime)
}

async fn create_videos(
    videos_info: Vec<&VideoInfo>,
    favorite_obj: &favorite::Model,
    connection: &DatabaseConnection,
) -> Result<u64> {
    let video_models = videos_info
        .iter()
        .map(move |v| video::ActiveModel {
            favorite_id: Set(favorite_obj.id),
            bvid: Set(v.bvid.clone()),
            path: Set(Path::new(favorite_obj.path.as_str())
                .join(&v.title)
                .to_str()
                .unwrap()
                .to_string()),
            name: Set(v.title.clone()),
            category: Set(v.vtype.to_string()),
            intro: Set(v.intro.clone()),
            cover: Set(v.cover.clone()),
            ctime: Set(v.ctime.to_string()),
            pubtime: Set(v.pubtime.to_string()),
            favtime: Set(v.fav_time.to_string()),
            downloaded: Set(false),
            valid: Set(true),
            tags: Set("[]".to_string()),
            single_page: Set(false),
            ..Default::default()
        })
        .collect::<Vec<video::ActiveModel>>();
    Ok(video::Entity::insert_many(video_models)
        .exec_without_returning(connection)
        .await?)
}
