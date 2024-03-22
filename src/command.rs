use std::path::Path;
use std::sync::Arc;

use entity::*;
use futures_util::{pin_mut, StreamExt};
use migration::OnConflict;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;

use crate::bilibili::{BiliClient, FavoriteList};
use crate::Result;

pub async fn handle_favorite(
    bili_client: Arc<BiliClient>,
    fid: i32,
    connection: Arc<DatabaseConnection>,
) -> Result<()> {
    let favorite_list = FavoriteList::new(bili_client.clone(), fid.to_string());
    let info = favorite_list.get_info().await?;
    let favorite_obj = favorite::Entity::insert(favorite::ActiveModel {
        f_id: Set(fid),
        name: Set(info.title),
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
    .exec_with_returning(connection.as_ref())
    .await?;
    println!(
        "Hi there! I'm going to scan this favorite: {:?}",
        favorite_obj
    );
    let video_stream = favorite_list.into_video_stream();
    pin_mut!(video_stream);
    while let Some(v) = video_stream.next().await {
        let not_exists = video::Entity::find()
            .filter(
                video::Column::Bvid
                    .eq(&v.bvid)
                    .and(video::Column::FavoriteId.eq(fid)),
            )
            .count(connection.as_ref())
            .await
            .is_ok_and(|x| x == 0);
        if !not_exists {
            break;
        }
        let _video_obj = video::Entity::insert(video::ActiveModel {
            favorite_id: Set(fid),
            bvid: Set(v.bvid),
            path: Set(Path::new(favorite_obj.path.as_str())
                .join(&v.title)
                .to_str()
                .unwrap()
                .to_string()),
            name: Set(v.title),
            category: Set(v.vtype.to_string()),
            intro: Set(v.intro),
            cover: Set(v.cover),
            ctime: Set(v.ctime.to_string()),
            pubtime: Set(v.pubtime.to_string()),
            favtime: Set(v.fav_time.to_string()),
            downloaded: Set(false),
            valid: Set(true),
            tags: Set("[]".to_string()),
            single_page: Set(false),
            ..Default::default()
        })
        .exec_with_returning(connection.as_ref())
        .await?;
    }
    Ok(())
}
