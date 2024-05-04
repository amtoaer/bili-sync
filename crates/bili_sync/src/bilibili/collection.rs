#![allow(dead_code)]

use anyhow::Result;
use async_stream::stream;
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use futures::Stream;
use reqwest::Method;
use serde::Deserialize;
use serde_json::Value;

use crate::bilibili::{BiliClient, Validate};

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum CollectionType {
    Series,
    Season,
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

#[derive(Debug, Deserialize)]
pub struct SimpleVideoInfo {
    pub bvid: String,
    #[serde(rename = "pic")]
    pub cover: String,
    #[serde(with = "ts_seconds")]
    pub ctime: DateTime<Utc>,
    #[serde(rename = "pubdate", with = "ts_seconds")]
    pub pubtime: DateTime<Utc>,
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
        let mut collection_info: CollectionInfo = serde_json::from_value(meta)?;
        collection_info.collection_type = self.collection.collection_type.clone();
        Ok(collection_info)
    }

    async fn get_series_info(&self) -> Result<Value> {
        assert!(
            self.collection.collection_type == CollectionType::Series,
            "collection type is not series"
        );
        self.client
            .request(Method::GET, "https://api.bilibili.com/x/series/series")
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
                vec![
                    ("mid", self.collection.mid.as_str()),
                    ("series_id", self.collection.sid.as_str()),
                    ("only_normal", "true"),
                    ("sort", "desc"),
                    ("pn", page.as_str()),
                    ("ps", "30"),
                ],
            ),
            CollectionType::Season => (
                "https://api.bilibili.com/x/polymer/web-space/seasons_archives_list",
                vec![
                    ("mid", self.collection.mid.as_str()),
                    ("season_id", self.collection.sid.as_str()),
                    ("sort_reverse", "true"),
                    ("page_num", page.as_str()),
                    ("page_size", "30"),
                ],
            ),
        };
        self.client
            .request(Method::GET, url)
            .query(&query)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?
            .validate()
    }

    pub async fn into_simple_video_stream(self) -> impl Stream<Item = SimpleVideoInfo> + 'a {
        stream! {
            let mut page = 1;
            loop {
                let mut videos = match self.get_videos(page).await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("failed to get videos of collection {:?} page {}: {}", self.collection, page, e);
                        break;
                    },
                };
                if !videos["data"]["archives"].is_array() {
                    warn!("no videos found in collection {:?} page {}", self.collection, page);
                    break;
                }
                let videos_info = match serde_json::from_value::<Vec<SimpleVideoInfo>>(videos["data"]["archives"].take()) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("failed to parse videos of collection {:?} page {}: {}", self.collection, page, e);
                        break;
                    },
                };
                for video_info in videos_info.into_iter(){
                    yield video_info;
                }
                let fields = match self.collection.collection_type{
                    CollectionType::Series => ["num", "size", "total"],
                    CollectionType::Season => ["page_num", "page_size", "total"],
                };
                let fields  = fields.into_iter().map(|f| videos["data"]["page"][f].as_i64()).collect::<Option<Vec<i64>>>().map(|v| (v[0], v[1], v[2]));
                let Some((num, size, total)) = fields else {
                    error!("failed to get pages of collection {:?} page {}: {:?}", self.collection, page, fields);
                    break;
                };
                if num * size >= total {
                    break;
                }
                page += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::{pin_mut, StreamExt};

    use super::*;
    use crate::core::utils::init_logging;

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

    #[ignore = "only for manual test"]
    #[tokio::test]
    async fn test_get_info() {
        init_logging().expect("初始化日志失败");
        let client = BiliClient::new();
        let testcases = vec![
            (
                CollectionItem {
                    mid: "521722088".to_owned(),
                    sid: "4523".to_owned(),
                    collection_type: CollectionType::Season,
                },
                133,
            ),
            (
                CollectionItem {
                    mid: "521722088".to_owned(),
                    sid: "387210".to_owned(),
                    collection_type: CollectionType::Series,
                },
                90,
            ),
        ];
        for (collection_item, expect) in testcases {
            let collection = Collection::new(&client, &collection_item);
            let simple_video_stream = collection.into_simple_video_stream().await;
            pin_mut!(simple_video_stream);
            let videos = simple_video_stream.collect::<Vec<_>>().await;
            assert_eq!(videos.len(), expect);
            // from the newest to the oldest
            assert!(videos.first().unwrap().pubtime >= videos.last().unwrap().pubtime);
        }
    }
}
