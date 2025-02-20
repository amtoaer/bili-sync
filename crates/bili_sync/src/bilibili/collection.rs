use std::fmt::{Display, Formatter};

use anyhow::{Context, Result, anyhow};
use async_stream::try_stream;
use futures::Stream;
use reqwest::Method;
use serde::Deserialize;
use serde_json::Value;

use crate::bilibili::credential::encoded_query;
use crate::bilibili::{BiliClient, MIXIN_KEY, Validate, VideoInfo};

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum CollectionType {
    Series,
    Season,
}

impl From<CollectionType> for i32 {
    fn from(v: CollectionType) -> Self {
        match v {
            CollectionType::Series => 1,
            CollectionType::Season => 2,
        }
    }
}

impl From<i32> for CollectionType {
    fn from(v: i32) -> Self {
        match v {
            1 => CollectionType::Series,
            2 => CollectionType::Season,
            _ => panic!("invalid collection type"),
        }
    }
}

impl Display for CollectionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CollectionType::Series => "列表",
            CollectionType::Season => "合集",
        };
        write!(f, "{}", s)
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct CollectionItem {
    pub mid: String,
    pub sid: String,
    pub collection_type: CollectionType,
}

pub struct Collection<'a> {
    client: &'a BiliClient,
    collection: &'a CollectionItem,
}

#[derive(Debug, PartialEq)]
pub struct CollectionInfo {
    pub name: String,
    pub mid: i64,
    pub sid: i64,
    pub collection_type: CollectionType,
}

impl<'de> Deserialize<'de> for CollectionInfo {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CollectionInfoRaw {
            mid: i64,
            name: String,
            season_id: Option<i64>,
            series_id: Option<i64>,
        }
        let raw = CollectionInfoRaw::deserialize(deserializer)?;
        let (sid, collection_type) = match (raw.season_id, raw.series_id) {
            (Some(sid), None) => (sid, CollectionType::Season),
            (None, Some(sid)) => (sid, CollectionType::Series),
            _ => return Err(serde::de::Error::custom("invalid collection info")),
        };
        Ok(CollectionInfo {
            mid: raw.mid,
            name: raw.name,
            sid,
            collection_type,
        })
    }
}

impl<'a> Collection<'a> {
    pub fn new(client: &'a BiliClient, collection: &'a CollectionItem) -> Self {
        Self { client, collection }
    }

    pub async fn get_info(&self) -> Result<CollectionInfo> {
        let meta = match self.collection.collection_type {
            // 没有找到专门获取 Season 信息的接口，所以直接获取第一页，从里面取 meta 信息
            CollectionType::Season => self.get_videos(1).await?["data"]["meta"].take(),
            CollectionType::Series => self.get_series_info().await?["data"]["meta"].take(),
        };
        Ok(serde_json::from_value(meta)?)
    }

    async fn get_series_info(&self) -> Result<Value> {
        self.client
            .request(Method::GET, "https://api.bilibili.com/x/series/series")
            .await
            .query(&[("series_id", self.collection.sid.as_str())])
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?
            .validate()
    }

    async fn get_videos(&self, page: i32) -> Result<Value> {
        let page = page.to_string();
        let (url, query) = match self.collection.collection_type {
            CollectionType::Series => (
                "https://api.bilibili.com/x/series/archives",
                encoded_query(
                    vec![
                        ("mid", self.collection.mid.as_str()),
                        ("series_id", self.collection.sid.as_str()),
                        ("only_normal", "true"),
                        ("sort", "desc"),
                        ("pn", page.as_str()),
                        ("ps", "30"),
                    ],
                    MIXIN_KEY.load().as_deref(),
                ),
            ),
            CollectionType::Season => (
                "https://api.bilibili.com/x/polymer/web-space/seasons_archives_list",
                encoded_query(
                    vec![
                        ("mid", self.collection.mid.as_str()),
                        ("season_id", self.collection.sid.as_str()),
                        ("sort_reverse", "true"),
                        ("page_num", page.as_str()),
                        ("page_size", "30"),
                    ],
                    MIXIN_KEY.load().as_deref(),
                ),
            ),
        };
        self.client
            .request(Method::GET, url)
            .await
            .query(&query)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?
            .validate()
    }

    pub fn into_video_stream(self) -> impl Stream<Item = Result<VideoInfo>> + 'a {
        try_stream! {
            let mut page = 1;
            loop {
                let mut videos = self.get_videos(page).await.with_context(|| {
                    format!(
                        "failed to get videos of collection {:?} page {}",
                        self.collection, page
                    )
                })?;
                let archives = &mut videos["data"]["archives"];
                if archives.as_array().is_none_or(|v| v.is_empty()) {
                    Err(anyhow!(
                        "no videos found in collection {:?} page {}",
                        self.collection,
                        page
                    ))?;
                }
                let videos_info: Vec<VideoInfo> = serde_json::from_value(archives.take()).with_context(|| {
                    format!(
                        "failed to parse videos of collection {:?} page {}",
                        self.collection, page
                    )
                })?;
                for video_info in videos_info {
                    yield video_info;
                }
                let page_info = &videos["data"]["page"];
                let fields = match self.collection.collection_type {
                    CollectionType::Series => ["num", "size", "total"],
                    CollectionType::Season => ["page_num", "page_size", "total"],
                };
                let values = fields
                    .iter()
                    .map(|f| page_info[f].as_i64())
                    .collect::<Vec<Option<i64>>>();
                if let [Some(num), Some(size), Some(total)] = values[..] {
                    if num * size < total {
                        page += 1;
                        continue;
                    }
                } else {
                    Err(anyhow!(
                        "invalid page info of collection {:?} page {}: read {:?} from {}",
                        self.collection,
                        page,
                        fields,
                        page_info
                    ))?;
                }
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_info_parse() {
        let testcases = vec![
            (
                r#"
                    {
                        "category": 0,
                        "cover": "https://archive.biliimg.com/bfs/archive/a6fbf7a7b9f4af09d9cf40482268634df387ef68.jpg",
                        "description": "",
                        "mid": 521722088,
                        "name": "合集·【命运方舟全剧情解说】",
                        "ptime": 1714701600,
                        "season_id": 1987140,
                        "total": 10
                    }
                "#,
                CollectionInfo {
                    mid: 521722088,
                    name: "合集·【命运方舟全剧情解说】".to_owned(),
                    sid: 1987140,
                    collection_type: CollectionType::Season,
                },
            ),
            (
                r#"
                    {
                        "series_id": 387212,
                        "mid": 521722088,
                        "name": "提瓦特冒险记",
                        "description": "原神沙雕般的游戏体验",
                        "keywords": [
                            ""
                        ],
                        "creator": "",
                        "state": 2,
                        "last_update_ts": 1633167320,
                        "total": 3,
                        "ctime": 1633167320,
                        "mtime": 1633167320,
                        "raw_keywords": "",
                        "category": 1
                    }
                "#,
                CollectionInfo {
                    mid: 521722088,
                    name: "提瓦特冒险记".to_owned(),
                    sid: 387212,
                    collection_type: CollectionType::Series,
                },
            ),
        ];
        for (json, expect) in testcases {
            let info: CollectionInfo = serde_json::from_str(json).unwrap();
            assert_eq!(info, expect);
        }
    }
}
