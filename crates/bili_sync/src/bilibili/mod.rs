use std::borrow::Cow;
use std::sync::Arc;

pub use analyzer::{BestStream, FilterOption};
use anyhow::{Result, bail, ensure};
use arc_swap::ArcSwapOption;
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
pub use client::{BiliClient, Client};
pub use collection::{Collection, CollectionItem, CollectionType};
pub use credential::Credential;
pub use danmaku::DanmakuOption;
pub use dynamic::Dynamic;
pub use error::BiliError;
pub use favorite_list::FavoriteList;
use favorite_list::Upper;
pub use me::Me;
use once_cell::sync::Lazy;
use reqwest::RequestBuilder;
pub use submission::Submission;
pub use video::{Dimension, PageInfo, Video};
pub use watch_later::WatchLater;

mod analyzer;
mod client;
mod collection;
mod credential;
mod danmaku;
mod dynamic;
mod error;
mod favorite_list;
mod me;
mod submission;
mod subtitle;
mod video;
mod watch_later;

static MIXIN_KEY: Lazy<ArcSwapOption<String>> = Lazy::new(Default::default);

pub(crate) fn set_global_mixin_key(key: String) {
    MIXIN_KEY.store(Some(Arc::new(key)));
}

pub(crate) trait Validate {
    type Output;

    fn validate(self) -> Result<Self::Output>;
}

impl Validate for serde_json::Value {
    type Output = serde_json::Value;

    fn validate(self) -> Result<Self::Output> {
        let (code, msg) = match (self["code"].as_i64(), self["message"].as_str()) {
            (Some(code), Some(msg)) => (code, msg),
            _ => bail!(BiliError::InvalidResponse(self.to_string())),
        };
        if code == -352 || !self["data"]["v_voucher"].is_null() {
            bail!(BiliError::RiskControlOccurred(self.to_string()));
        }
        ensure!(
            code == 0,
            BiliError::ErrorResponse(code, msg.to_owned(), self.to_string())
        );
        Ok(self)
    }
}

pub(crate) trait WbiSign {
    type Output;

    fn wbi_sign(self, mixin_key: Option<impl AsRef<str>>) -> Result<Self::Output>;
}

impl WbiSign for RequestBuilder {
    type Output = RequestBuilder;

    fn wbi_sign(self, mixin_key: Option<impl AsRef<str>>) -> Result<Self::Output> {
        let Some(mixin_key) = mixin_key else {
            return Ok(self);
        };
        let (client, req) = self.build_split();
        let mut req = req?;
        sign_request(&mut req, mixin_key.as_ref(), chrono::Utc::now().timestamp())?;
        Ok(RequestBuilder::from_parts(client, req))
    }
}

fn sign_request(req: &mut reqwest::Request, mixin_key: &str, timestamp: i64) -> Result<()> {
    let mut query_pairs = req.url().query_pairs().collect::<Vec<_>>();
    let timestamp = timestamp.to_string();
    query_pairs.push(("wts".into(), Cow::Borrowed(timestamp.as_str())));
    query_pairs.sort_by(|a, b| a.0.cmp(&b.0));
    let query_str = serde_urlencoded::to_string(query_pairs)?.replace('+', "%20");
    let w_rid = format!("{:x}", md5::compute(query_str + mixin_key));
    req.url_mut()
        .query_pairs_mut()
        .extend_pairs([("w_rid", w_rid), ("wts", timestamp)]);
    Ok(())
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
/// 注意此处的顺序是有要求的，因为对于 untagged 的 enum 来说，serde 会按照顺序匹配
/// > There is no explicit tag identifying which variant the data contains.
/// > Serde will try to match the data against each variant in order and the first one that deserializes successfully is the one returned.
pub enum VideoInfo {
    /// 从视频详情接口获取的视频信息
    Detail {
        title: String,
        bvid: String,
        #[serde(rename = "desc")]
        intro: String,
        #[serde(rename = "pic")]
        cover: String,
        #[serde(rename = "owner")]
        upper: Upper<i64>,
        #[serde(with = "ts_seconds")]
        ctime: DateTime<Utc>,
        #[serde(rename = "pubdate", with = "ts_seconds")]
        pubtime: DateTime<Utc>,
        is_upower_exclusive: bool,
        is_upower_play: bool,
        redirect_url: Option<String>,
        pages: Vec<PageInfo>,
        state: i32,
    },
    /// 从收藏夹接口获取的视频信息
    Favorite {
        title: String,
        #[serde(rename = "type")]
        vtype: i32,
        bvid: String,
        intro: String,
        cover: String,
        upper: Upper<i64>,
        #[serde(with = "ts_seconds")]
        ctime: DateTime<Utc>,
        #[serde(with = "ts_seconds")]
        fav_time: DateTime<Utc>,
        #[serde(with = "ts_seconds")]
        pubtime: DateTime<Utc>,
        attr: i32,
    },
    /// 从稍后再看接口获取的视频信息
    WatchLater {
        title: String,
        bvid: String,
        #[serde(rename = "desc")]
        intro: String,
        #[serde(rename = "pic")]
        cover: String,
        #[serde(rename = "owner")]
        upper: Upper<i64>,
        #[serde(with = "ts_seconds")]
        ctime: DateTime<Utc>,
        #[serde(rename = "add_at", with = "ts_seconds")]
        fav_time: DateTime<Utc>,
        #[serde(rename = "pubdate", with = "ts_seconds")]
        pubtime: DateTime<Utc>,
        state: i32,
    },
    /// 从视频合集/视频列表接口获取的视频信息
    Collection {
        bvid: String,
        #[serde(rename = "pic")]
        cover: String,
        #[serde(with = "ts_seconds")]
        ctime: DateTime<Utc>,
        #[serde(rename = "pubdate", with = "ts_seconds")]
        pubtime: DateTime<Utc>,
    },
    // 从用户投稿接口获取的视频信息
    Submission {
        title: String,
        bvid: String,
        #[serde(rename = "description")]
        intro: String,
        #[serde(rename = "pic")]
        cover: String,
        #[serde(rename = "created", with = "ts_seconds")]
        ctime: DateTime<Utc>,
    },
    // 从动态获取的视频信息（此处 pubtime 未在结构中，因此使用 default + 手动赋值）
    Dynamic {
        title: String,
        bvid: String,
        desc: String,
        cover: String,
        #[serde(default)]
        pubtime: DateTime<Utc>,
    },
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anyhow::Context;
    use futures::StreamExt;
    use reqwest::Method;

    use super::*;
    use crate::bilibili::credential::WbiImg;
    use crate::config::VersionedConfig;
    use crate::database::setup_database;
    use crate::utils::init_logger;

    #[ignore = "only for manual test"]
    #[tokio::test]
    async fn test_video_info_type() -> Result<()> {
        VersionedConfig::init_for_test(&setup_database(Path::new("./test.sqlite")).await?).await?;
        let credential = &VersionedConfig::get().read().credential;
        init_logger("None,bili_sync=debug", None);
        let bili_client = BiliClient::new();
        // 请求 UP 主视频必须要获取 mixin key，使用 key 计算请求参数的签名，否则直接提示权限不足返回空
        let mixin_key = bili_client
            .wbi_img(credential)
            .await?
            .into_mixin_key()
            .context("no mixin key")?;
        set_global_mixin_key(mixin_key);
        let collection = Collection::new(
            &bili_client,
            CollectionItem {
                mid: "521722088".to_string(),
                sid: "4523".to_string(),
                collection_type: CollectionType::Season,
            },
            &credential,
        );
        let videos = collection
            .into_video_stream()
            .take(20)
            .filter_map(|v| futures::future::ready(v.ok()))
            .collect::<Vec<_>>()
            .await;
        assert!(videos.iter().all(|v| matches!(v, VideoInfo::Collection { .. })));
        assert!(videos.iter().rev().is_sorted_by_key(|v| v.release_datetime()));
        // 测试收藏夹
        let favorite = FavoriteList::new(&bili_client, "3144336058".to_string(), &credential);
        let videos = favorite
            .into_video_stream()
            .take(20)
            .filter_map(|v| futures::future::ready(v.ok()))
            .collect::<Vec<_>>()
            .await;
        assert!(videos.iter().all(|v| matches!(v, VideoInfo::Favorite { .. })));
        assert!(videos.iter().rev().is_sorted_by_key(|v| v.release_datetime()));
        // 测试稍后再看
        let watch_later = WatchLater::new(&bili_client, &credential);
        let videos = watch_later
            .into_video_stream()
            .take(20)
            .filter_map(|v| futures::future::ready(v.ok()))
            .collect::<Vec<_>>()
            .await;
        assert!(videos.iter().all(|v| matches!(v, VideoInfo::WatchLater { .. })));
        assert!(videos.iter().rev().is_sorted_by_key(|v| v.release_datetime()));
        // 测试投稿
        let submission = Submission::new(&bili_client, "956761".to_string(), &credential);
        let videos = submission
            .into_video_stream()
            .take(20)
            .filter_map(|v| futures::future::ready(v.ok()))
            .collect::<Vec<_>>()
            .await;
        assert!(videos.iter().all(|v| matches!(v, VideoInfo::Submission { .. })));
        assert!(videos.iter().rev().is_sorted_by_key(|v| v.release_datetime()));
        // 测试动态
        let dynamic = Dynamic::new(&bili_client, "659898".to_string(), &credential);
        let videos = dynamic
            .into_video_stream()
            .take(20)
            .filter_map(|v| futures::future::ready(v.ok()))
            .collect::<Vec<_>>()
            .await;
        assert!(videos.iter().all(|v| matches!(v, VideoInfo::Dynamic { .. })));
        assert!(videos.iter().skip(1).rev().is_sorted_by_key(|v| v.release_datetime()));
        Ok(())
    }

    #[ignore = "only for manual test"]
    #[tokio::test]
    async fn test_subtitle_parse() -> Result<()> {
        VersionedConfig::init_for_test(&setup_database(Path::new("./test.sqlite")).await?).await?;
        let credential = &VersionedConfig::get().read().credential;
        let bili_client = BiliClient::new();
        let mixin_key = bili_client
            .wbi_img(credential)
            .await?
            .into_mixin_key()
            .context("no mixin key")?;
        set_global_mixin_key(mixin_key);
        let video = Video::new(&bili_client, "BV1gLfnY8E6D".to_string(), &credential);
        let pages = video.get_pages().await?;
        println!("pages: {:?}", pages);
        let subtitles = video.get_subtitles(&pages[0]).await?;
        for subtitle in subtitles {
            println!(
                "{}: {}",
                subtitle.lan,
                subtitle.body.to_string().chars().take(200).collect::<String>()
            );
        }
        Ok(())
    }

    #[ignore = "only for manual test"]
    #[tokio::test]
    async fn test_upower_parse() -> Result<()> {
        VersionedConfig::init_for_test(&setup_database(Path::new("./test.sqlite")).await?).await?;
        let credential = &VersionedConfig::get().read().credential;
        let bili_client = BiliClient::new();
        let mixin_key = bili_client
            .wbi_img(credential)
            .await?
            .into_mixin_key()
            .context("no mixin key")?;
        set_global_mixin_key(mixin_key);
        for (bvid, (upower_exclusive, upower_play)) in [
            ("BV1HxXwYEEqt", (true, false)),  // 充电专享且无权观看
            ("BV16w41187fx", (true, true)),   // 充电专享但有权观看
            ("BV1n34jzPEYq", (false, false)), // 普通视频
        ] {
            let video = Video::new(&bili_client, bvid.to_string(), credential);
            let info = video.get_view_info().await?;
            let VideoInfo::Detail {
                is_upower_exclusive,
                is_upower_play,
                ..
            } = info
            else {
                unreachable!();
            };
            assert_eq!(is_upower_exclusive, upower_exclusive, "bvid: {}", bvid);
            assert_eq!(is_upower_play, upower_play, "bvid: {}", bvid);
        }
        Ok(())
    }

    #[ignore = "only for manual test"]
    #[tokio::test]
    async fn test_ep_parse() -> Result<()> {
        VersionedConfig::init_for_test(&setup_database(Path::new("./test.sqlite")).await?).await?;
        let credential = &VersionedConfig::get().read().credential;
        let bili_client = BiliClient::new();
        let mixin_key = bili_client
            .wbi_img(credential)
            .await?
            .into_mixin_key()
            .context("no mixin key")?;
        set_global_mixin_key(mixin_key);
        for (bvid, redirect_is_none) in [
            ("BV1SF411g796", false), // EP
            ("BV13xtnzPEye", false), // 番剧
            ("BV1kT4NzTEZj", true),  // 普通视频
        ] {
            let video = Video::new(&bili_client, bvid.to_string(), credential);
            let info = video.get_view_info().await?;
            let VideoInfo::Detail { redirect_url, .. } = info else {
                unreachable!();
            };
            assert_eq!(redirect_url.is_none(), redirect_is_none, "bvid: {}", bvid);
        }
        Ok(())
    }

    #[test]
    fn test_wbi_key() -> Result<()> {
        let key = WbiImg {
            img_url: "https://i0.hdslb.com/bfs/wbi/7cd084941338484aae1ad9425b84077c.png".to_string(),
            sub_url: "https://i0.hdslb.com/bfs/wbi/4932caff0ff746eab6f01bf08b70ac45.png".to_string(),
        };
        let key = key.into_mixin_key().context("no mixin key")?;
        assert_eq!(key.as_str(), "ea1db124af3c7062474693fa704f4ff8");
        let client = Client::new();
        let mut req = client
            .request(Method::GET, "https://www.baidu.com/", None)
            .query(&[("foo", "114"), ("bar", "514")])
            .query(&[("zab", "1919810")])
            .build()?;
        sign_request(&mut req, key.as_str(), 1702204169).unwrap();
        let query: Vec<_> = req.url().query_pairs().collect();
        assert_eq!(
            query,
            vec![
                ("foo".into(), "114".into()),
                ("bar".into(), "514".into()),
                ("zab".into(), "1919810".into()),
                ("w_rid".into(), "8f6f2b5b3d485fe1886cec6a0be8c5d4".into()),
                ("wts".into(), "1702204169".into()),
            ]
        );
        let key = WbiImg {
            img_url: "https://i0.hdslb.com/bfs/wbi/7cd084941338484aae1ad9425b84077c.png".to_string(),
            sub_url: "https://i0.hdslb.com/bfs/wbi/4932caff0ff746eab6f01bf08b70ac45.png".to_string(),
        };
        let key = key.into_mixin_key().context("no mixin key")?;
        let mut req = client
            .request(Method::GET, "https://www.baidu.com/", None)
            .query(&[("mid", "11997177"), ("token", "")])
            .query(&[("platform", "web"), ("web_location", "1550101")])
            .build()?;
        sign_request(&mut req, key.as_str(), 1703513649).unwrap();
        let query: Vec<_> = req.url().query_pairs().collect();
        assert_eq!(
            query,
            vec![
                ("mid".into(), "11997177".into()),
                ("token".into(), "".into()),
                ("platform".into(), "web".into()),
                ("web_location".into(), "1550101".into()),
                ("w_rid".into(), "7d4428b3f2f9ee2811e116ec6fd41a4f".into()),
                ("wts".into(), "1703513649".into()),
            ]
        );
        Ok(())
    }
}
