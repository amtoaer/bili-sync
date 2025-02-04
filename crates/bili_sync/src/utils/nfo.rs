use anyhow::Result;
use bili_sync_entity::*;
use quick_xml::events::{BytesCData, BytesText};
use quick_xml::writer::Writer;
use quick_xml::Error;
use tokio::io::AsyncWriteExt;

use crate::config::NFOTimeType;

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

/// serde xml 似乎不太好用，先这么裸着写
/// （真是又臭又长啊
impl NFOSerializer<'_> {
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
                            .write_cdata_content_async(BytesCData::new(Self::format_plot(v)))
                            .await?;
                        writer.create_element("outline").write_empty_async().await?;
                        writer
                            .create_element("title")
                            .write_text_content_async(BytesText::new(&v.name))
                            .await?;
                        writer
                            .create_element("actor")
                            .write_inner_content_async::<_, _, Error>(|writer| async move {
                                writer
                                    .create_element("name")
                                    .write_text_content_async(BytesText::new(&v.upper_id.to_string()))
                                    .await?;
                                writer
                                    .create_element("role")
                                    .write_text_content_async(BytesText::new(&v.upper_name))
                                    .await?;
                                Ok(writer)
                            })
                            .await?;
                        writer
                            .create_element("year")
                            .write_text_content_async(BytesText::new(&nfo_time.format("%Y").to_string()))
                            .await?;
                        if let Some(tags) = &v.tags {
                            let tags: Vec<String> = serde_json::from_value(tags.clone()).unwrap_or_default();
                            for tag in tags {
                                writer
                                    .create_element("genre")
                                    .write_text_content_async(BytesText::new(&tag))
                                    .await?;
                            }
                        }
                        writer
                            .create_element("uniqueid")
                            .with_attribute(("type", "bilibili"))
                            .write_text_content_async(BytesText::new(&v.bvid))
                            .await?;
                        writer
                            .create_element("aired")
                            .write_text_content_async(BytesText::new(&nfo_time.format("%Y-%m-%d").to_string()))
                            .await?;
                        Ok(writer)
                    })
                    .await?;
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
                            .write_cdata_content_async(BytesCData::new(Self::format_plot(v)))
                            .await?;
                        writer.create_element("outline").write_empty_async().await?;
                        writer
                            .create_element("title")
                            .write_text_content_async(BytesText::new(&v.name))
                            .await?;
                        writer
                            .create_element("actor")
                            .write_inner_content_async::<_, _, Error>(|writer| async move {
                                writer
                                    .create_element("name")
                                    .write_text_content_async(BytesText::new(&v.upper_id.to_string()))
                                    .await?;
                                writer
                                    .create_element("role")
                                    .write_text_content_async(BytesText::new(&v.upper_name))
                                    .await?;
                                Ok(writer)
                            })
                            .await?;
                        writer
                            .create_element("year")
                            .write_text_content_async(BytesText::new(&nfo_time.format("%Y").to_string()))
                            .await?;
                        if let Some(tags) = &v.tags {
                            let tags: Vec<String> = serde_json::from_value(tags.clone()).unwrap_or_default();
                            for tag in tags {
                                writer
                                    .create_element("genre")
                                    .write_text_content_async(BytesText::new(&tag))
                                    .await?;
                            }
                        }
                        writer
                            .create_element("uniqueid")
                            .with_attribute(("type", "bilibili"))
                            .write_text_content_async(BytesText::new(&v.bvid))
                            .await?;
                        writer
                            .create_element("aired")
                            .write_text_content_async(BytesText::new(&nfo_time.format("%Y-%m-%d").to_string()))
                            .await?;
                        Ok(writer)
                    })
                    .await?;
            }
            NFOSerializer(ModelWrapper::Video(v), NFOMode::UPPER) => {
                writer
                    .create_element("person")
                    .write_inner_content_async::<_, _, Error>(|writer| async move {
                        writer.create_element("plot").write_empty_async().await?;
                        writer.create_element("outline").write_empty_async().await?;
                        writer
                            .create_element("lockdata")
                            .write_text_content_async(BytesText::new("false"))
                            .await?;
                        writer
                            .create_element("dateadded")
                            .write_text_content_async(BytesText::new(
                                &v.pubtime.format("%Y-%m-%d %H:%M:%S").to_string(),
                            ))
                            .await?;
                        writer
                            .create_element("title")
                            .write_text_content_async(BytesText::new(&v.upper_id.to_string()))
                            .await?;
                        writer
                            .create_element("sorttitle")
                            .write_text_content_async(BytesText::new(&v.upper_id.to_string()))
                            .await?;
                        Ok(writer)
                    })
                    .await?;
            }
            NFOSerializer(ModelWrapper::Page(p), NFOMode::EPOSODE) => {
                writer
                    .create_element("episodedetails")
                    .write_inner_content_async::<_, _, Error>(|writer| async move {
                        writer.create_element("plot").write_empty_async().await?;
                        writer.create_element("outline").write_empty_async().await?;
                        writer
                            .create_element("title")
                            .write_text_content_async(BytesText::new(&p.name))
                            .await?;
                        writer
                            .create_element("season")
                            .write_text_content_async(BytesText::new("1"))
                            .await?;
                        writer
                            .create_element("episode")
                            .write_text_content_async(BytesText::new(&p.pid.to_string()))
                            .await?;
                        Ok(writer)
                    })
                    .await?;
            }
            _ => unreachable!(),
        }
        tokio_buffer.flush().await?;
        Ok(String::from_utf8(buffer)?)
    }

    #[inline]
    fn format_plot(model: &video::Model) -> String {
        format!(
            r#"原始视频：<a href="https://www.bilibili.com/video/{}/">{}</a><br/><br/>{}"#,
            model.bvid, model.bvid, model.intro
        )
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
            favtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2022, 2, 2).unwrap(),
                chrono::NaiveTime::from_hms_opt(2, 2, 2).unwrap(),
            ),
            pubtime: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2033, 3, 3).unwrap(),
                chrono::NaiveTime::from_hms_opt(3, 3, 3).unwrap(),
            ),
            bvid: "BV1nWcSeeEkV".to_string(),
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
    <plot><![CDATA[原始视频：<a href="https://www.bilibili.com/video/BV1nWcSeeEkV/">BV1nWcSeeEkV</a><br/><br/>intro]]></plot>
    <outline/>
    <title>name</title>
    <actor>
        <name>1</name>
        <role>upper_name</role>
    </actor>
    <year>2033</year>
    <genre>tag1</genre>
    <genre>tag2</genre>
    <uniqueid type="bilibili">BV1nWcSeeEkV</uniqueid>
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
    <plot><![CDATA[原始视频：<a href="https://www.bilibili.com/video/BV1nWcSeeEkV/">BV1nWcSeeEkV</a><br/><br/>intro]]></plot>
    <outline/>
    <title>name</title>
    <actor>
        <name>1</name>
        <role>upper_name</role>
    </actor>
    <year>2022</year>
    <genre>tag1</genre>
    <genre>tag2</genre>
    <uniqueid type="bilibili">BV1nWcSeeEkV</uniqueid>
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
