use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use quick_xml::de::from_str;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct RssVideo {
    pub video_id: String,
    pub title: String,
    pub url: String,
    pub description: String,
    pub uploader: String,
    pub thumbnail: Option<String>,
    pub published_at: DateTime<Utc>,
}

pub async fn fetch_channel_videos(channel_id: &str) -> Result<Vec<RssVideo>> {
    let url = format!("https://www.youtube.com/feeds/videos.xml?channel_id={channel_id}");
    let body = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; bili-sync YouTube RSS)")
        .build()?
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let feed: Feed = from_str(&body).context("failed to parse youtube rss feed")?;
    feed.entries
        .into_iter()
        .map(|entry| {
            let published_at = DateTime::parse_from_rfc3339(&entry.published)
                .with_context(|| format!("invalid publish time for {}", entry.video_id))?
                .with_timezone(&Utc);
            Ok(RssVideo {
                video_id: entry.video_id,
                title: entry.title,
                url: entry.link.href,
                description: entry
                    .media_group
                    .as_ref()
                    .and_then(|group| group.description.clone())
                    .unwrap_or_default(),
                uploader: entry.author.name,
                thumbnail: entry
                    .media_group
                    .and_then(|group| group.thumbnail.map(|thumb| thumb.url)),
                published_at,
            })
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct Feed {
    #[serde(rename = "entry", default)]
    entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
struct Entry {
    title: String,
    #[serde(rename = "yt:videoId")]
    video_id: String,
    published: String,
    link: Link,
    author: Author,
    #[serde(rename = "media:group")]
    media_group: Option<MediaGroup>,
}

#[derive(Debug, Deserialize)]
struct Link {
    #[serde(rename = "@href")]
    href: String,
}

#[derive(Debug, Deserialize)]
struct Author {
    name: String,
}

#[derive(Debug, Deserialize)]
struct MediaGroup {
    #[serde(rename = "media:description")]
    description: Option<String>,
    #[serde(rename = "media:thumbnail")]
    thumbnail: Option<MediaThumbnail>,
}

#[derive(Debug, Deserialize)]
struct MediaThumbnail {
    #[serde(rename = "@url")]
    url: String,
}
