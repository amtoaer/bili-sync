use std::path::Path;

use anyhow::Result;
use bili_sync_entity::*;
use bili_sync_migration::OnConflict;
use chrono::{DateTime, Utc};
use handlebars::handlebars_helper;
use once_cell::sync::Lazy;
use quick_xml::events::{BytesCData, BytesText};
use quick_xml::writer::Writer;
use quick_xml::Error;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use tokio::io::AsyncWriteExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::bilibili::{FavoriteListInfo, PageInfo, VideoInfo};
use crate::config::{NFOTimeType, CONFIG};
use crate::model::VideoListModel;

pub static TEMPLATE: Lazy<handlebars::Handlebars> = Lazy::new(|| {
    let mut handlebars = handlebars::Handlebars::new();
    handlebars_helper!(truncate: |s: String, len: usize| {
        if s.chars().count() > len {
            s.chars().take(len).collect::<String>()
        } else {
            s.to_string()
        }
    });
    handlebars.register_helper("truncate", Box::new(truncate));
    handlebars
        .register_template_string("video", &CONFIG.video_name)
        .unwrap();
    handlebars.register_template_string("page", &CONFIG.page_name).unwrap();
    handlebars
});

#[allow(clippy::upper_case_acronyms)]
pub enum NFOMode {
    MOVIE,
    TVSHOW,
    EPOSODE,
    UPPER,
}

pub enum ModelWrapper<'a> {
    Video(&'a video::Model),
    Page(&'a page::Model),
}

pub struct NFOSerializer<'a>(pub ModelWrapper<'a>, pub NFOMode);

/// 根据获得的收藏夹信息，插入或更新数据库中的收藏夹，并返回收藏夹对象
pub async fn handle_favorite_info(
    info: &FavoriteListInfo,
    path: &Path,
    connection: &DatabaseConnection,
) -> Result<favorite::Model> {
    favorite::Entity::insert(favorite::ActiveModel {
        f_id: Set(info.id),
        name: Set(info.title.clone()),
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
    Ok(favorite::Entity::find()
        .filter(favorite::Column::FId.eq(info.id))
        .one(connection)
        .await?
        .unwrap())
}

/// 尝试创建 Video Model，如果发生冲突则忽略
pub async fn create_videos(
    videos_info: &[VideoInfo],
    favorite: &favorite::Model,
    connection: &DatabaseConnection,
) -> Result<()> {
    let video_models = favorite.video_models_by_info(videos_info)?;
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

pub async fn total_video_count(favorite_model: &favorite::Model, connection: &DatabaseConnection) -> Result<u64> {
    Ok(video::Entity::find()
        .filter(video::Column::FavoriteId.eq(favorite_model.id))
        .count(connection)
        .await?)
}

/// 创建视频的所有分 P
pub async fn create_video_pages(
    pages_info: &[PageInfo],
    video_model: &video::Model,
    connection: &impl ConnectionTrait,
) -> Result<()> {
    let page_models = pages_info
        .iter()
        .map(move |p| {
            let (width, height) = match &p.dimension {
                Some(d) => {
                    if d.rotate == 0 {
                        (Some(d.width), Some(d.height))
                    } else {
                        (Some(d.height), Some(d.width))
                    }
                }
                None => (None, None),
            };
            page::ActiveModel {
                video_id: Set(video_model.id),
                cid: Set(p.cid),
                pid: Set(p.page),
                name: Set(p.name.clone()),
                width: Set(width),
                height: Set(height),
                duration: Set(p.duration),
                image: Set(p.first_frame.clone()),
                download_status: Set(0),
                ..Default::default()
            }
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

/// 更新视频 model 的下载状态
pub async fn update_videos_model(videos: Vec<video::ActiveModel>, connection: &DatabaseConnection) -> Result<()> {
    video::Entity::insert_many(videos)
        .on_conflict(
            OnConflict::column(video::Column::Id)
                .update_column(video::Column::DownloadStatus)
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

/// serde xml 似乎不太好用，先这么裸着写
/// （真是又臭又长啊
impl<'a> NFOSerializer<'a> {
    pub async fn generate_nfo(self, nfo_time_type: &NFOTimeType) -> Result<String> {
        let mut buffer = r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
"#
        .as_bytes()
        .to_vec();
        let mut tokio_buffer = tokio::io::BufWriter::new(&mut buffer);
        let mut writer = Writer::new_with_indent(&mut tokio_buffer, b' ', 4);
        match self {
            NFOSerializer(ModelWrapper::Video(v), NFOMode::MOVIE) => {
                let nfo_time = match nfo_time_type {
                    NFOTimeType::FavTime => v.favtime,
                    NFOTimeType::PubTime => v.pubtime,
                };
                writer
                    .create_element("movie")
                    .write_inner_content_async::<_, _, Error>(|writer| async move {
                        writer
                            .create_element("plot")
                            .write_cdata_content_async(BytesCData::new(&v.intro))
                            .await
                            .unwrap();
                        writer.create_element("outline").write_empty_async().await.unwrap();
                        writer
                            .create_element("title")
                            .write_text_content_async(BytesText::new(&v.name))
                            .await
                            .unwrap();
                        writer
                            .create_element("actor")
                            .write_inner_content_async::<_, _, Error>(|writer| async move {
                                writer
                                    .create_element("name")
                                    .write_text_content_async(BytesText::new(&v.upper_id.to_string()))
                                    .await
                                    .unwrap();
                                writer
                                    .create_element("role")
                                    .write_text_content_async(BytesText::new(&v.upper_name))
                                    .await
                                    .unwrap();
                                Ok(writer)
                            })
                            .await
                            .unwrap();
                        writer
                            .create_element("year")
                            .write_text_content_async(BytesText::new(&nfo_time.format("%Y").to_string()))
                            .await
                            .unwrap();
                        if let Some(tags) = &v.tags {
                            let tags: Vec<String> = serde_json::from_value(tags.clone()).unwrap();
                            for tag in tags {
                                writer
                                    .create_element("genre")
                                    .write_text_content_async(BytesText::new(&tag))
                                    .await
                                    .unwrap();
                            }
                        }
                        writer
                            .create_element("uniqueid")
                            .with_attribute(("type", "bilibili"))
                            .write_text_content_async(BytesText::new(&v.bvid))
                            .await
                            .unwrap();
                        writer
                            .create_element("aired")
                            .write_text_content_async(BytesText::new(&nfo_time.format("%Y-%m-%d").to_string()))
                            .await
                            .unwrap();
                        Ok(writer)
                    })
                    .await
                    .unwrap();
            }
            NFOSerializer(ModelWrapper::Video(v), NFOMode::TVSHOW) => {
                let nfo_time = match nfo_time_type {
                    NFOTimeType::FavTime => v.favtime,
                    NFOTimeType::PubTime => v.pubtime,
                };
                writer
                    .create_element("tvshow")
                    .write_inner_content_async::<_, _, Error>(|writer| async move {
                        writer
                            .create_element("plot")
                            .write_cdata_content_async(BytesCData::new(&v.intro))
                            .await
                            .unwrap();
                        writer.create_element("outline").write_empty_async().await.unwrap();
                        writer
                            .create_element("title")
                            .write_text_content_async(BytesText::new(&v.name))
                            .await
                            .unwrap();
                        writer
                            .create_element("actor")
                            .write_inner_content_async::<_, _, Error>(|writer| async move {
                                writer
                                    .create_element("name")
                                    .write_text_content_async(BytesText::new(&v.upper_id.to_string()))
                                    .await
                                    .unwrap();
                                writer
                                    .create_element("role")
                                    .write_text_content_async(BytesText::new(&v.upper_name))
                                    .await
                                    .unwrap();
                                Ok(writer)
                            })
                            .await
                            .unwrap();
                        writer
                            .create_element("year")
                            .write_text_content_async(BytesText::new(&nfo_time.format("%Y").to_string()))
                            .await
                            .unwrap();
                        if let Some(tags) = &v.tags {
                            let tags: Vec<String> = serde_json::from_value(tags.clone()).unwrap();
                            for tag in tags {
                                writer
                                    .create_element("genre")
                                    .write_text_content_async(BytesText::new(&tag))
                                    .await
                                    .unwrap();
                            }
                        }
                        writer
                            .create_element("uniqueid")
                            .with_attribute(("type", "bilibili"))
                            .write_text_content_async(BytesText::new(&v.bvid))
                            .await
                            .unwrap();
                        writer
                            .create_element("aired")
                            .write_text_content_async(BytesText::new(&nfo_time.format("%Y-%m-%d").to_string()))
                            .await
                            .unwrap();
                        Ok(writer)
                    })
                    .await
                    .unwrap();
            }
            NFOSerializer(ModelWrapper::Video(v), NFOMode::UPPER) => {
                writer
                    .create_element("person")
                    .write_inner_content_async::<_, _, Error>(|writer| async move {
                        writer.create_element("plot").write_empty_async().await.unwrap();
                        writer.create_element("outline").write_empty_async().await.unwrap();
                        writer
                            .create_element("lockdata")
                            .write_text_content_async(BytesText::new("false"))
                            .await
                            .unwrap();
                        writer
                            .create_element("dateadded")
                            .write_text_content_async(BytesText::new(
                                &v.pubtime.format("%Y-%m-%d %H:%M:%S").to_string(),
                            ))
                            .await
                            .unwrap();
                        writer
                            .create_element("title")
                            .write_text_content_async(BytesText::new(&v.upper_id.to_string()))
                            .await
                            .unwrap();
                        writer
                            .create_element("sorttitle")
                            .write_text_content_async(BytesText::new(&v.upper_id.to_string()))
                            .await
                            .unwrap();
                        Ok(writer)
                    })
                    .await
                    .unwrap();
            }
            NFOSerializer(ModelWrapper::Page(p), NFOMode::EPOSODE) => {
                writer
                    .create_element("episodedetails")
                    .write_inner_content_async::<_, _, Error>(|writer| async move {
                        writer.create_element("plot").write_empty_async().await.unwrap();
                        writer.create_element("outline").write_empty_async().await.unwrap();
                        writer
                            .create_element("title")
                            .write_text_content_async(BytesText::new(&p.name))
                            .await
                            .unwrap();
                        writer
                            .create_element("season")
                            .write_text_content_async(BytesText::new("1"))
                            .await
                            .unwrap();
                        writer
                            .create_element("episode")
                            .write_text_content_async(BytesText::new(&p.pid.to_string()))
                            .await
                            .unwrap();
                        Ok(writer)
                    })
                    .await
                    .unwrap();
            }
            _ => unreachable!(),
        }
        tokio_buffer.flush().await?;
        Ok(std::str::from_utf8(&buffer).unwrap().to_owned())
    }
}

pub fn init_logger(log_level: &str) {
    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::builder().parse_lossy(log_level))
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
            "%Y-%m-%d %H:%M:%S%.3f".to_owned(),
        ))
        .finish()
        .try_init()
        .expect("初始化日志失败");
}

/// 对于视频标记，均由 bvid 和时间戳构成
pub fn id_time_key(bvid: &String, time: &DateTime<Utc>) -> String {
    format!("{}-{}", bvid, time.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_nfo() {
        let video = video::Model {
            intro: "intro".to_string(),
            name: "name".to_string(),
            upper_id: 1,
            upper_name: "upper_name".to_string(),
            favtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2022, 2, 2).unwrap(),
                chrono::NaiveTime::from_hms_opt(2, 2, 2).unwrap(),
            ),
            pubtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2033, 3, 3).unwrap(),
                chrono::NaiveTime::from_hms_opt(3, 3, 3).unwrap(),
            ),
            bvid: "bvid".to_string(),
            tags: Some(serde_json::json!(["tag1", "tag2"])),
            ..Default::default()
        };
        assert_eq!(
            NFOSerializer(ModelWrapper::Video(&video), NFOMode::MOVIE)
                .generate_nfo(&NFOTimeType::PubTime)
                .await
                .unwrap(),
            r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<movie>
    <plot><![CDATA[intro]]></plot>
    <outline/>
    <title>name</title>
    <actor>
        <name>1</name>
        <role>upper_name</role>
    </actor>
    <year>2033</year>
    <genre>tag1</genre>
    <genre>tag2</genre>
    <uniqueid type="bilibili">bvid</uniqueid>
    <aired>2033-03-03</aired>
</movie>"#,
        );
        assert_eq!(
            NFOSerializer(ModelWrapper::Video(&video), NFOMode::TVSHOW)
                .generate_nfo(&NFOTimeType::FavTime)
                .await
                .unwrap(),
            r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<tvshow>
    <plot><![CDATA[intro]]></plot>
    <outline/>
    <title>name</title>
    <actor>
        <name>1</name>
        <role>upper_name</role>
    </actor>
    <year>2022</year>
    <genre>tag1</genre>
    <genre>tag2</genre>
    <uniqueid type="bilibili">bvid</uniqueid>
    <aired>2022-02-02</aired>
</tvshow>"#,
        );
        assert_eq!(
            NFOSerializer(ModelWrapper::Video(&video), NFOMode::UPPER)
                .generate_nfo(&NFOTimeType::FavTime)
                .await
                .unwrap(),
            r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<person>
    <plot/>
    <outline/>
    <lockdata>false</lockdata>
    <dateadded>2033-03-03 03:03:03</dateadded>
    <title>1</title>
    <sorttitle>1</sorttitle>
</person>"#,
        );
        let page = page::Model {
            name: "name".to_string(),
            pid: 3,
            ..Default::default()
        };
        assert_eq!(
            NFOSerializer(ModelWrapper::Page(&page), NFOMode::EPOSODE)
                .generate_nfo(&NFOTimeType::FavTime)
                .await
                .unwrap(),
            r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<episodedetails>
    <plot/>
    <outline/>
    <title>name</title>
    <season>1</season>
    <episode>3</episode>
</episodedetails>"#,
        );
    }
}
