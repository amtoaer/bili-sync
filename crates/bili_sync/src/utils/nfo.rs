use anyhow::Result;
use bili_sync_entity::*;
use chrono::NaiveDateTime;
use quick_xml::Error;
use quick_xml::events::{BytesCData, BytesText};
use quick_xml::writer::Writer;
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::config::{CONFIG, NFOTimeType};

#[allow(clippy::upper_case_acronyms)]
pub enum NFO<'a> {
    Movie(Movie<'a>),
    TVShow(TVShow<'a>),
    Upper(Upper),
    Episode(Episode<'a>),
}

pub struct Movie<'a> {
    pub name: &'a str,
    pub intro: &'a str,
    pub bvid: &'a str,
    pub upper_id: i64,
    pub upper_name: &'a str,
    pub aired: NaiveDateTime,
    pub tags: Option<Vec<String>>,
}

pub struct TVShow<'a> {
    pub name: &'a str,
    pub intro: &'a str,
    pub bvid: &'a str,
    pub upper_id: i64,
    pub upper_name: &'a str,
    pub aired: NaiveDateTime,
    pub tags: Option<Vec<String>>,
}

pub struct Upper {
    pub upper_id: String,
    pub pubtime: NaiveDateTime,
}

pub struct Episode<'a> {
    pub name: &'a str,
    pub pid: String,
}

impl NFO<'_> {
    pub async fn generate_nfo(self) -> Result<String> {
        let mut buffer = r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
"#
        .as_bytes()
        .to_vec();
        let mut tokio_buffer = BufWriter::new(&mut buffer);
        let writer = Writer::new_with_indent(&mut tokio_buffer, b' ', 4);
        match self {
            NFO::Movie(movie) => {
                Self::write_movie_nfo(writer, movie).await?;
            }
            NFO::TVShow(tvshow) => {
                Self::write_tvshow_nfo(writer, tvshow).await?;
            }
            NFO::Upper(upper) => {
                Self::write_upper_nfo(writer, upper).await?;
            }
            NFO::Episode(episode) => {
                Self::write_episode_nfo(writer, episode).await?;
            }
        }
        tokio_buffer.flush().await?;
        Ok(String::from_utf8(buffer)?)
    }

    async fn write_movie_nfo(mut writer: Writer<&mut BufWriter<&mut Vec<u8>>>, movie: Movie<'_>) -> Result<()> {
        writer
            .create_element("movie")
            .write_inner_content_async::<_, _, Error>(|writer| async move {
                writer
                    .create_element("plot")
                    .write_cdata_content_async(BytesCData::new(Self::format_plot(movie.bvid, movie.intro)))
                    .await?;
                writer.create_element("outline").write_empty_async().await?;
                writer
                    .create_element("title")
                    .write_text_content_async(BytesText::new(movie.name))
                    .await?;
                writer
                    .create_element("actor")
                    .write_inner_content_async::<_, _, Error>(|writer| async move {
                        writer
                            .create_element("name")
                            .write_text_content_async(BytesText::new(&movie.upper_id.to_string()))
                            .await?;
                        writer
                            .create_element("role")
                            .write_text_content_async(BytesText::new(movie.upper_name))
                            .await?;
                        Ok(writer)
                    })
                    .await?;
                writer
                    .create_element("year")
                    .write_text_content_async(BytesText::new(&movie.aired.format("%Y").to_string()))
                    .await?;
                if let Some(tags) = movie.tags {
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
                    .write_text_content_async(BytesText::new(movie.bvid))
                    .await?;
                writer
                    .create_element("aired")
                    .write_text_content_async(BytesText::new(&movie.aired.format("%Y-%m-%d").to_string()))
                    .await?;
                Ok(writer)
            })
            .await?;
        Ok(())
    }

    async fn write_tvshow_nfo(mut writer: Writer<&mut BufWriter<&mut Vec<u8>>>, tvshow: TVShow<'_>) -> Result<()> {
        writer
            .create_element("tvshow")
            .write_inner_content_async::<_, _, Error>(|writer| async move {
                writer
                    .create_element("plot")
                    .write_cdata_content_async(BytesCData::new(Self::format_plot(tvshow.bvid, tvshow.intro)))
                    .await?;
                writer.create_element("outline").write_empty_async().await?;
                writer
                    .create_element("title")
                    .write_text_content_async(BytesText::new(tvshow.name))
                    .await?;
                writer
                    .create_element("actor")
                    .write_inner_content_async::<_, _, Error>(|writer| async move {
                        writer
                            .create_element("name")
                            .write_text_content_async(BytesText::new(&tvshow.upper_id.to_string()))
                            .await?;
                        writer
                            .create_element("role")
                            .write_text_content_async(BytesText::new(tvshow.upper_name))
                            .await?;
                        Ok(writer)
                    })
                    .await?;
                writer
                    .create_element("year")
                    .write_text_content_async(BytesText::new(&tvshow.aired.format("%Y").to_string()))
                    .await?;
                if let Some(tags) = tvshow.tags {
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
                    .write_text_content_async(BytesText::new(tvshow.bvid))
                    .await?;
                writer
                    .create_element("aired")
                    .write_text_content_async(BytesText::new(&tvshow.aired.format("%Y-%m-%d").to_string()))
                    .await?;
                Ok(writer)
            })
            .await?;
        Ok(())
    }

    async fn write_upper_nfo(mut writer: Writer<&mut BufWriter<&mut Vec<u8>>>, upper: Upper) -> Result<()> {
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
                    .write_text_content_async(BytesText::new(&upper.pubtime.format("%Y-%m-%d %H:%M:%S").to_string()))
                    .await?;
                writer
                    .create_element("title")
                    .write_text_content_async(BytesText::new(&upper.upper_id))
                    .await?;
                writer
                    .create_element("sorttitle")
                    .write_text_content_async(BytesText::new(&upper.upper_id))
                    .await?;
                Ok(writer)
            })
            .await?;
        Ok(())
    }

    async fn write_episode_nfo(mut writer: Writer<&mut BufWriter<&mut Vec<u8>>>, episode: Episode<'_>) -> Result<()> {
        writer
            .create_element("episodedetails")
            .write_inner_content_async::<_, _, Error>(|writer| async move {
                writer.create_element("plot").write_empty_async().await?;
                writer.create_element("outline").write_empty_async().await?;
                writer
                    .create_element("title")
                    .write_text_content_async(BytesText::new(episode.name))
                    .await?;
                writer
                    .create_element("season")
                    .write_text_content_async(BytesText::new("1"))
                    .await?;
                writer
                    .create_element("episode")
                    .write_text_content_async(BytesText::new(&episode.pid))
                    .await?;
                Ok(writer)
            })
            .await?;
        Ok(())
    }

    #[inline]
    fn format_plot(bvid: &str, intro: &str) -> String {
        format!(
            r#"原始视频：<a href="https://www.bilibili.com/video/{}/">{}</a><br/><br/>{}"#,
            bvid, bvid, intro,
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
            NFO::Movie((&video).into()).generate_nfo().await.unwrap(),
            r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<movie>
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
</movie>"#,
        );
        assert_eq!(
            NFO::TVShow((&video).into()).generate_nfo().await.unwrap(),
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
            NFO::Upper((&video).into()).generate_nfo().await.unwrap(),
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
            NFO::Episode((&page).into()).generate_nfo().await.unwrap(),
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

impl<'a> From<&'a video::Model> for Movie<'a> {
    fn from(video: &'a video::Model) -> Self {
        Self {
            name: &video.name,
            intro: &video.intro,
            bvid: &video.bvid,
            upper_id: video.upper_id,
            upper_name: &video.upper_name,
            aired: match CONFIG.nfo_time_type {
                NFOTimeType::FavTime => video.favtime,
                NFOTimeType::PubTime => video.pubtime,
            },
            tags: video
                .tags
                .as_ref()
                .and_then(|tags| serde_json::from_value(tags.clone()).ok()),
        }
    }
}

impl<'a> From<&'a video::Model> for TVShow<'a> {
    fn from(video: &'a video::Model) -> Self {
        Self {
            name: &video.name,
            intro: &video.intro,
            bvid: &video.bvid,
            upper_id: video.upper_id,
            upper_name: &video.upper_name,
            aired: match CONFIG.nfo_time_type {
                NFOTimeType::FavTime => video.favtime,
                NFOTimeType::PubTime => video.pubtime,
            },
            tags: video
                .tags
                .as_ref()
                .and_then(|tags| serde_json::from_value(tags.clone()).ok()),
        }
    }
}

impl<'a> From<&'a video::Model> for Upper {
    fn from(video: &'a video::Model) -> Self {
        Self {
            upper_id: video.upper_id.to_string(),
            pubtime: video.pubtime,
        }
    }
}

impl<'a> From<&'a page::Model> for Episode<'a> {
    fn from(page: &'a page::Model) -> Self {
        Self {
            name: &page.name,
            pid: page.pid.to_string(),
        }
    }
}
