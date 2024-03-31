use std::collections::HashSet;
use std::path::Path;

use entity::*;
use migration::OnConflict;
use once_cell::sync::Lazy;
use quick_xml::events::BytesText;
use quick_xml::writer::Writer;
use quick_xml::Error;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::QuerySelect;
use serde_json::json;
use tokio::io::AsyncWriteExt;

use super::status::Status;
use crate::bilibili::{FavoriteListInfo, PageInfo, VideoInfo};
use crate::config::CONFIG;
use crate::Result;

pub static TEMPLATE: Lazy<handlebars::Handlebars> = Lazy::new(|| {
    let mut handlebars = handlebars::Handlebars::new();
    let config = CONFIG.lock().unwrap();
    handlebars
        .register_template_string("video", config.video_name.clone())
        .unwrap();
    handlebars
        .register_template_string("page", config.page_name.clone())
        .unwrap();
    handlebars
});

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
    path: &str,
    connection: &DatabaseConnection,
) -> Result<favorite::Model> {
    favorite::Entity::insert(favorite::ActiveModel {
        f_id: Set(info.id),
        name: Set(info.title.to_string()),
        path: Set(path.to_owned()),
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

/// 获取数据库中存在的与该视频 favorite_id 和 bvid 重合的视频中的 bvid 和 favtime
/// 如果 bvid 和 favtime 均相同，说明到达了上次处理到的位置
pub async fn exist_labels(
    videos_info: &[VideoInfo],
    favorite_model: &favorite::Model,
    connection: &DatabaseConnection,
) -> Result<HashSet<(String, DateTime)>> {
    let bvids = videos_info.iter().map(|v| v.bvid.clone()).collect::<Vec<String>>();
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
            name: Set(v.title.clone()),
            path: Set(Path::new(&favorite.path)
                .join(
                    TEMPLATE
                        .render(
                            "video",
                            &json!({
                                "bvid": &v.bvid,
                                "title": &v.title,
                                "upper_name": &v.upper.name,
                                "upper_mid": &v.upper.mid,
                            }),
                        )
                        .unwrap_or_else(|_| v.bvid.clone()),
                )
                .to_str()
                .unwrap()
                .to_owned()),
            category: Set(v.vtype),
            intro: Set(v.intro.clone()),
            cover: Set(v.cover.clone()),
            ctime: Set(v.ctime.naive_utc()),
            pubtime: Set(v.pubtime.naive_utc()),
            favtime: Set(v.fav_time.naive_utc()),
            download_status: Set(0),
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
    let bvids = videos_info.iter().map(|v| v.bvid.clone()).collect::<Vec<String>>();
    let mut condition = video::Column::FavoriteId
        .eq(favorite_model.id)
        .and(video::Column::Bvid.is_in(bvids))
        .and(video::Column::Valid.eq(true));
    if only_unhandled {
        condition = condition.and(video::Column::DownloadStatus.lt(Status::handled()));
    }
    if only_no_page {
        condition = condition.and(video::Column::SinglePage.is_null());
    }
    Ok(video::Entity::find().filter(condition).all(connection).await?)
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
            image: Set(p.first_frame.clone()),
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
                .and(video::Column::DownloadStatus.lt(Status::handled()))
                .and(video::Column::SinglePage.is_not_null()),
        )
        .find_with_related(page::Entity)
        .all(connection)
        .await?)
}

/// serde xml 似乎不太好用，先这么裸着写
/// （真是又臭又长啊
impl<'a> NFOSerializer<'a> {
    pub async fn generate_nfo(self) -> Result<String> {
        let mut buffer = r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
"#
        .as_bytes()
        .to_vec();
        let mut tokio_buffer = tokio::io::BufWriter::new(&mut buffer);
        let mut writer = Writer::new_with_indent(&mut tokio_buffer, b' ', 4);
        match self {
            NFOSerializer(ModelWrapper::Video(v), NFOMode::MOVIE) => {
                writer
                    .create_element("movie")
                    .write_inner_content_async::<_, _, Error>(|writer| async move {
                        writer
                            .create_element("plot")
                            .write_text_content_async(BytesText::new(&format!(r#"![CDATA[{}]]"#, &v.intro)))
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
                            .write_text_content_async(BytesText::new(&v.pubtime.format("%Y").to_string()))
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
                            .write_text_content_async(BytesText::new(&v.pubtime.format("%Y-%m-%d").to_string()))
                            .await
                            .unwrap();
                        Ok(writer)
                    })
                    .await
                    .unwrap();
            }
            NFOSerializer(ModelWrapper::Video(v), NFOMode::TVSHOW) => {
                writer
                    .create_element("tvshow")
                    .write_inner_content_async::<_, _, Error>(|writer| async move {
                        writer
                            .create_element("plot")
                            .write_text_content_async(BytesText::new(&format!(r#"![CDATA[{}]]"#, &v.intro)))
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
                            .write_text_content_async(BytesText::new(&v.pubtime.format("%Y").to_string()))
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
                            .write_text_content_async(BytesText::new(&v.pubtime.format("%Y-%m-%d").to_string()))
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
                            .write_text_content_async(BytesText::new(&v.pubtime.format("%Y-%m-%d").to_string()))
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
            pubtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2022, 2, 2).unwrap(),
                chrono::NaiveTime::from_hms_opt(2, 2, 2).unwrap(),
            ),
            bvid: "bvid".to_string(),
            tags: Some(serde_json::json!(["tag1", "tag2"])),
            ..Default::default()
        };
        assert_eq!(
            NFOSerializer(ModelWrapper::Video(&video), NFOMode::MOVIE)
                .generate_nfo()
                .await
                .unwrap(),
            r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<movie>
    <plot>![CDATA[intro]]</plot>
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
</movie>"#,
        );
        assert_eq!(
            NFOSerializer(ModelWrapper::Video(&video), NFOMode::TVSHOW)
                .generate_nfo()
                .await
                .unwrap(),
            r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<tvshow>
    <plot>![CDATA[intro]]</plot>
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
                .generate_nfo()
                .await
                .unwrap(),
            r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<person>
    <plot/>
    <outline/>
    <lockdata>false</lockdata>
    <dateadded>2022-02-02</dateadded>
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
                .generate_nfo()
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
