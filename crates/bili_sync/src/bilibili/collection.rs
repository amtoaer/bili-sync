use std::fmt::{Display, Formatter};

use anyhow::{Context, Result, anyhow, bail};
use async_stream::try_stream;
use futures::Stream;
use reqwest::Method;
use serde::Deserialize;
use serde_json::Value;

use crate::bilibili::{Bangumi, BangumiTarget, BiliClient, Credential, ErrorForStatusExt, Validate, VideoInfo};

#[derive(PartialEq, Eq, Hash, Clone, Debug, Default, Copy)]
pub enum CollectionType {
    Series,
    #[default]
    Season,
    BangumiSeason,
    BangumiEpisode,
}

impl<'de> serde::Deserialize<'de> for CollectionType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = i32::deserialize(deserializer)?;
        CollectionType::try_from(v).map_err(serde::de::Error::custom)
    }
}

impl From<CollectionType> for i32 {
    fn from(v: CollectionType) -> Self {
        match v {
            CollectionType::Series => 1,
            CollectionType::Season => 2,
            CollectionType::BangumiSeason => 3,
            CollectionType::BangumiEpisode => 4,
        }
    }
}

impl TryFrom<i32> for CollectionType {
    type Error = anyhow::Error;

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(CollectionType::Series),
            2 => Ok(CollectionType::Season),
            3 => Ok(CollectionType::BangumiSeason),
            4 => Ok(CollectionType::BangumiEpisode),
            v => Err(anyhow!("got invalid collection type {}", v)),
        }
    }
}

impl CollectionType {
    pub fn is_bangumi(self) -> bool {
        matches!(self, Self::BangumiSeason | Self::BangumiEpisode)
    }

    pub fn from_expected(v: i32) -> Self {
        Self::try_from(v).expect("invalid collection type")
    }
}

impl Display for CollectionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CollectionType::Series => "列表",
            CollectionType::Season => "合集",
            CollectionType::BangumiSeason => "番剧整季",
            CollectionType::BangumiEpisode => "番剧单集",
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
    pub collection: CollectionItem,
    credential: &'a Credential,
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
    pub fn new(client: &'a BiliClient, collection: CollectionItem, credential: &'a Credential) -> Self {
        Self {
            client,
            collection,
            credential,
        }
    }

    pub async fn get_info(&self) -> Result<CollectionInfo> {
        if self.collection.collection_type.is_bangumi() {
            let target = self.bangumi_target()?;
            let season = Bangumi::new(self.client, self.credential).get_season(target).await?;
            let name = match target {
                BangumiTarget::Season(_) => season.title,
                BangumiTarget::Episode(id) => {
                    let episode = season
                        .episodes
                        .iter()
                        .find(|episode| episode.id == id)
                        .context("番剧中未找到指定分集")?;
                    format!("{} - {} {}", season.title, episode.title, episode.long_title)
                        .trim()
                        .to_owned()
                }
            };
            return Ok(CollectionInfo {
                name,
                mid: 0,
                sid: self.collection.sid.parse()?,
                collection_type: self.collection.collection_type,
            });
        }
        let meta = match self.collection.collection_type {
            // 没有找到专门获取 Season 信息的接口，所以直接获取第一页，从里面取 meta 信息
            CollectionType::Season => self.get_videos(1).await?["data"]["meta"].take(),
            CollectionType::Series => self.get_series_info().await?["data"]["meta"].take(),
            CollectionType::BangumiSeason | CollectionType::BangumiEpisode => unreachable!(),
        };
        Ok(serde_json::from_value(meta)?)
    }

    fn bangumi_target(&self) -> Result<BangumiTarget> {
        let id = self.collection.sid.parse()?;
        match self.collection.collection_type {
            CollectionType::BangumiSeason => Ok(BangumiTarget::Season(id)),
            CollectionType::BangumiEpisode => Ok(BangumiTarget::Episode(id)),
            _ => bail!("该视频源不是番剧"),
        }
    }

    async fn get_series_info(&self) -> Result<Value> {
        self.client
            .request(Method::GET, "https://api.bilibili.com/x/series/series", self.credential)
            .await
            .query(&[("series_id", self.collection.sid.as_str())])
            .send()
            .await?
            .error_for_status_ext()?
            .json::<Value>()
            .await?
            .validate()
    }

    async fn get_videos(&self, page: i32) -> Result<Value> {
        let req = match self.collection.collection_type {
            CollectionType::Series => self
                .client
                .request(
                    Method::GET,
                    "https://api.bilibili.com/x/series/archives",
                    self.credential,
                )
                .await
                .query(&[("pn", page)])
                .query(&[
                    ("mid", self.collection.mid.as_str()),
                    ("series_id", self.collection.sid.as_str()),
                    ("only_normal", "true"),
                    ("sort", "desc"),
                    ("ps", "30"),
                ]),
            CollectionType::Season => self
                .client
                .request(
                    Method::GET,
                    "https://api.bilibili.com/x/polymer/web-space/seasons_archives_list",
                    self.credential,
                )
                .await
                .query(&[("page_num", page)])
                .query(&[
                    ("mid", self.collection.mid.as_str()),
                    ("season_id", self.collection.sid.as_str()),
                    ("sort_reverse", "true"),
                    ("page_size", "30"),
                ]),
            CollectionType::BangumiSeason | CollectionType::BangumiEpisode => bail!("番剧不使用投稿分页接口"),
        };
        req.send()
            .await?
            .error_for_status_ext()?
            .json::<Value>()
            .await?
            .validate()
    }

    pub fn into_video_stream(self) -> impl Stream<Item = Result<VideoInfo>> + 'a {
        try_stream! {
            if self.collection.collection_type.is_bangumi() {
                let target = self.bangumi_target()?;
                let season = Bangumi::new(self.client, self.credential).get_season(target).await?;
                let episodes = season.episodes.iter().filter(|episode| match target {
                    BangumiTarget::Season(_) => true,
                    BangumiTarget::Episode(id) => episode.id == id,
                }).collect::<Vec<_>>();
                if episodes.is_empty() {
                    Err(anyhow!("番剧中没有可下载的分集"))?;
                }
                for episode in episodes {
                    // 连载中的整季可能提前列出尚未发布的集数；后续全量扫描会补上。
                    // 单集明确指定的目标仍应报错，避免把未发布内容当作已完成。
                    if matches!(target, BangumiTarget::Season(_))
                        && (episode.cid <= 0 || episode.bvid.is_empty()) {
                        continue;
                    }
                    yield episode.to_video_info(&season)?;
                }
                return;
            }
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
                    if page == 1 {
                        break;
                    }
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
                    CollectionType::BangumiSeason | CollectionType::BangumiEpisode => unreachable!(),
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
