use std::str::FromStr;

use anyhow::{Context, Result, bail, ensure};
use bili_sync_entity::upper_vec::Upper;
use chrono::{DateTime, Utc};
use reqwest::{Method, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::analyzer::PageAnalyzer;
use super::{BiliClient, Credential, ErrorForStatusExt, PageInfo, Validate, VideoInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BangumiTarget {
    Episode(i64),
    Season(i64),
}

impl FromStr for BangumiTarget {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self> {
        let input = input.trim();
        let id = if input.contains("://") {
            let url = Url::parse(input).context("无效的番剧链接")?;
            ensure!(
                matches!(url.scheme(), "https" | "http")
                    && matches!(
                        url.host_str(),
                        Some("www.bilibili.com" | "bilibili.com" | "m.bilibili.com")
                    )
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.port().is_none(),
                "请输入 bilibili.com 的番剧播放链接"
            );
            url.path()
                .strip_prefix("/bangumi/play/")
                .context("番剧链接应包含 /bangumi/play/ep 或 /bangumi/play/ss")?
                .trim_end_matches('/')
                .to_owned()
        } else {
            input.to_owned()
        };
        let (prefix, number) = if let Some(number) = id.strip_prefix("ep") {
            ("ep", number)
        } else if let Some(number) = id.strip_prefix("ss") {
            ("ss", number)
        } else {
            bail!("请输入 ep32367、ss1733 或完整番剧播放链接");
        };
        ensure!(
            !number.is_empty() && number.bytes().all(|c| c.is_ascii_digit()),
            "番剧编号必须是正整数"
        );
        let number: i64 = number.parse().context("番剧编号超出范围")?;
        ensure!(number > 0, "番剧编号必须是正整数");
        Ok(if prefix == "ep" {
            Self::Episode(number)
        } else {
            Self::Season(number)
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct BangumiSeason {
    pub season_id: i64,
    pub title: String,
    #[serde(default)]
    pub episodes: Vec<BangumiEpisode>,
    #[serde(default)]
    evaluate: String,
    #[serde(default)]
    cover: String,
    up_info: Option<BangumiUpper>,
    #[serde(default)]
    section: Vec<BangumiSection>,
    total: Option<u32>,
    stat: Option<BangumiStat>,
    rating: Option<BangumiRating>,
    publish: Option<BangumiPublish>,
    new_ep: Option<BangumiUpdate>,
    // Count before appending a directly requested extra episode to the download list.
    #[serde(skip)]
    published_episodes: usize,
}

#[derive(Debug, Deserialize)]
struct BangumiStat {
    views: Option<u64>,
    favorites: Option<u64>,
    follow_text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BangumiRating {
    score: Option<f64>,
}

impl BangumiRating {
    pub fn value(&self) -> Option<f64> {
        self.score
            .filter(|score| score.is_finite() && (0.0..=10.0).contains(score))
    }
}

#[derive(Debug, Deserialize)]
pub struct BangumiPublish {
    pub_time_show: Option<String>,
    pub_time: Option<String>,
}

impl BangumiPublish {
    /// B 站提供的是开播当地时间文案，保留原文，不按浏览器时区换算。
    pub fn display_time(&self) -> Option<String> {
        [self.pub_time_show.as_deref(), self.pub_time.as_deref()]
            .into_iter()
            .flatten()
            .map(str::trim)
            .find(|text| !text.starts_with("0000") && text.chars().any(|c| matches!(c, '1'..='9')))
            .map(str::to_owned)
    }
}

#[derive(Debug, Deserialize)]
struct BangumiUpdate {
    desc: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BangumiPreview {
    pub season_id: i64,
    pub title: String,
    pub cover: String,
    pub description: String,
    pub total_episodes: Option<u32>,
    pub published_episodes: usize,
    pub views: Option<u64>,
    pub followers: Option<u64>,
    pub follow_text: Option<String>,
    pub rating: Option<f64>,
    pub pub_time: Option<String>,
    pub status: Option<String>,
    pub episode: Option<BangumiPreviewEpisode>,
    pub url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BangumiPreviewEpisode {
    pub id: i64,
    pub title: String,
    pub long_title: String,
}

impl BangumiSeason {
    fn preview(&self, target: BangumiTarget) -> Result<BangumiPreview> {
        let (episode, url) = match target {
            BangumiTarget::Episode(id) => {
                let episode = self
                    .episodes
                    .iter()
                    .find(|episode| episode.id == id)
                    .context("番剧分集不存在或已下架")?;
                (
                    Some(BangumiPreviewEpisode {
                        id: episode.id,
                        title: episode.title.clone(),
                        long_title: episode.long_title.clone(),
                    }),
                    format!("https://www.bilibili.com/bangumi/play/ep{id}"),
                )
            }
            BangumiTarget::Season(id) => (None, format!("https://www.bilibili.com/bangumi/play/ss{id}")),
        };
        let cover = if self.cover.starts_with("//") {
            format!("https:{}", self.cover)
        } else {
            self.cover.replacen("http://", "https://", 1)
        };
        let cover = Url::parse(&cover)
            .ok()
            .filter(|url| url.scheme() == "https" && url.host_str().is_some())
            .map(|url| url.to_string())
            .unwrap_or_default();
        Ok(BangumiPreview {
            season_id: self.season_id,
            title: self.title.clone(),
            cover,
            description: self.evaluate.clone(),
            // 连载番剧 total=0 表示总集数尚未确定，并不代表该季没有分集。
            total_episodes: self.total.filter(|total| *total > 0),
            published_episodes: self.published_episodes,
            views: self.stat.as_ref().and_then(|stat| stat.views),
            followers: self.stat.as_ref().and_then(|stat| stat.favorites),
            follow_text: self
                .stat
                .as_ref()
                .and_then(|stat| stat.follow_text.clone())
                .filter(|text| !text.trim().is_empty()),
            rating: self.rating.as_ref().and_then(BangumiRating::value),
            pub_time: self.publish.as_ref().and_then(BangumiPublish::display_time),
            status: self
                .new_ep
                .as_ref()
                .and_then(|update| update.desc.as_ref())
                .filter(|description| !description.trim().is_empty())
                .cloned(),
            episode,
            url,
        })
    }
}

#[derive(Debug, Deserialize)]
struct BangumiUpper {
    mid: i64,
    uname: String,
    avatar: String,
}

#[derive(Debug, Deserialize)]
struct BangumiSection {
    #[serde(default)]
    episodes: Vec<BangumiEpisode>,
}

#[derive(Debug, Deserialize)]
pub struct BangumiEpisode {
    pub id: i64,
    #[serde(default)]
    pub bvid: String,
    #[serde(default)]
    pub cid: i64,
    pub title: String,
    #[serde(default)]
    pub long_title: String,
    #[serde(default)]
    pub cover: String,
    /// PGC 元信息中的时长单位为毫秒。
    #[serde(default)]
    pub duration: u64,
    #[serde(default)]
    pub pub_time: i64,
    #[serde(default)]
    section_type: i32,
}

impl BangumiEpisode {
    pub fn to_video_info(&self, season: &BangumiSeason) -> Result<VideoInfo> {
        ensure!(
            self.id > 0 && self.cid > 0 && !self.bvid.is_empty(),
            "番剧 ep{} 尚未发布或缺少播放信息",
            self.id
        );
        let pubtime = DateTime::<Utc>::from_timestamp(self.pub_time, 0).context("番剧发布时间无效")?;
        let episode_name = if self.title.bytes().all(|c| c.is_ascii_digit()) && !self.title.is_empty() {
            format!("第{}话 {}", self.title, self.long_title).trim().to_owned()
        } else {
            format!("{} {}", self.title, self.long_title).trim().to_owned()
        };
        let title = format!("{} {}", season.title, episode_name);
        let cover = if self.cover.is_empty() {
            &season.cover
        } else {
            &self.cover
        }
        .clone();
        let upper = match &season.up_info {
            Some(upper) => Upper {
                mid: upper.mid,
                name: upper.uname.clone(),
                face: upper.avatar.clone(),
                title: None,
            },
            None => Upper {
                mid: 0,
                name: season.title.clone(),
                face: season.cover.clone(),
                title: None,
            },
        };
        Ok(VideoInfo::Detail {
            title: title.clone(),
            bvid: self.bvid.clone(),
            intro: season.evaluate.clone(),
            cover: cover.clone(),
            upper,
            staff: None,
            ctime: pubtime,
            pubtime,
            is_upower_exclusive: false,
            is_upower_play: false,
            redirect_url: Some(format!("https://www.bilibili.com/bangumi/play/ep{}", self.id)),
            pages: vec![PageInfo {
                cid: self.cid,
                page: 1,
                name: title,
                duration: self.duration.div_ceil(1000).try_into().context("番剧时长超出范围")?,
                first_frame: Some(cover),
                dimension: None,
            }],
            state: 0,
        })
    }
}

pub struct Bangumi<'a> {
    client: &'a BiliClient,
    credential: &'a Credential,
}

impl<'a> Bangumi<'a> {
    pub fn new(client: &'a BiliClient, credential: &'a Credential) -> Self {
        Self { client, credential }
    }

    pub async fn get_preview(&self, target: BangumiTarget) -> Result<BangumiPreview> {
        self.get_season(target).await?.preview(target)
    }

    pub async fn get_season(&self, target: BangumiTarget) -> Result<BangumiSeason> {
        let query = match target {
            BangumiTarget::Episode(id) => ("ep_id", id),
            BangumiTarget::Season(id) => ("season_id", id),
        };
        ensure!(query.1 > 0, "番剧编号必须是正整数");
        let response = self
            .client
            .request(
                Method::GET,
                "https://api.bilibili.com/pgc/view/web/season",
                self.credential,
            )
            .await
            .query(&[query])
            .send()
            .await?
            .error_for_status_ext()?
            .json::<Value>()
            .await?;
        parse_season(response, target)
    }

    pub async fn get_episode_detail(&self, ep_id: i64) -> Result<VideoInfo> {
        let season = self.get_season(BangumiTarget::Episode(ep_id)).await?;
        season
            .episodes
            .iter()
            .find(|ep| ep.id == ep_id)
            .context("番剧分集不存在或已下架")?
            .to_video_info(&season)
    }

    pub async fn get_page_analyzer(&self, ep_id: i64, cid: i64) -> Result<PageAnalyzer> {
        ensure!(ep_id > 0 && cid > 0, "番剧分集编号无效");
        let response = self
            .client
            .request(
                Method::GET,
                "https://api.bilibili.com/pgc/player/web/playurl",
                self.credential,
            )
            .await
            .query(&[("ep_id", ep_id), ("cid", cid)])
            .query(&[("qn", "127"), ("otype", "json"), ("fnval", "4048"), ("fourk", "1")])
            .send()
            .await?
            .error_for_status_ext()?
            .json::<Value>()
            .await?;
        parse_playurl(response)
    }
}

fn parse_season(response: Value, target: BangumiTarget) -> Result<BangumiSeason> {
    let mut response = response.validate()?;
    let mut season: BangumiSeason =
        serde_json::from_value(response["result"].take()).context("番剧详情缺少有效 result")?;
    ensure!(season.season_id > 0, "番剧 season_id 无效");
    if let BangumiTarget::Season(id) = target {
        ensure!(season.season_id == id, "番剧接口返回的季编号与请求不一致");
    }
    season.published_episodes = season
        .episodes
        .iter()
        .filter(|episode| episode.id > 0 && episode.cid > 0 && !episode.bvid.is_empty() && episode.section_type != 1)
        .map(|episode| episode.id)
        .collect::<std::collections::HashSet<_>>()
        .len();
    if let BangumiTarget::Episode(id) = target {
        // 整季只包含正片；直接指定花絮 EP 时仍可单独下载。
        if !season.episodes.iter().any(|ep| ep.id == id) {
            let episode = season
                .section
                .iter_mut()
                .flat_map(|section| section.episodes.drain(..))
                .find(|ep| ep.id == id)
                .context("番剧分集不存在或已下架")?;
            season.episodes.push(episode);
        }
    }
    let mut seen = std::collections::HashSet::new();
    season.episodes.retain(|ep| seen.insert(ep.id));
    Ok(season)
}

fn is_enabled(value: &Value) -> bool {
    value.as_bool() == Some(true) || value.as_i64().is_some_and(|value| value != 0)
}

fn parse_playurl(response: Value) -> Result<PageAnalyzer> {
    let mut response = response.validate()?;
    let mut info = response["result"].take();
    ensure!(info.is_object(), "番剧播放接口缺少 result");
    for field in ["code", "error_code"] {
        ensure!(
            info[field].as_i64().is_none_or(|code| code == 0),
            "番剧无法播放：{}",
            info["message"]
        );
    }
    ensure!(
        !is_enabled(&info["is_preview"]),
        "番剧仅返回试看内容，请检查账号会员、购买或地区权限"
    );
    ensure!(
        !is_enabled(&info["is_drm"]) && !is_enabled(&info["drm_tech_type"]),
        "暂不支持 DRM 加密番剧"
    );
    // 现有下载器仅支持单个混合流，避免把多段 FLV 的第一段误记为完整一集。
    ensure!(
        info["durl"].as_array().is_none_or(|parts| parts.len() <= 1),
        "暂不支持多段番剧流，请使用 DASH 格式"
    );
    ensure!(
        info["dash"]["video"]
            .as_array()
            .is_some_and(|streams| !streams.is_empty())
            || info["durl"].as_array().is_some_and(|streams| !streams.is_empty()),
        "番剧未返回可用播放流，请检查账号会员、购买或地区权限"
    );
    // PGC 可能在 DASH 响应中保留空 durl；通用分析器按字段存在性判断混合流。
    if info["durl"].is_null() || info["durl"].as_array().is_some_and(Vec::is_empty) {
        info.as_object_mut().unwrap().remove("durl");
    }
    for path in ["/dash/video", "/dash/audio", "/dash/dolby/audio"] {
        if let Some(streams) = info.pointer_mut(path).and_then(Value::as_array_mut) {
            for stream in streams {
                normalize_stream_urls(stream);
            }
        }
    }
    if let Some(stream) = info.pointer_mut("/dash/flac/audio") {
        normalize_stream_urls(stream);
    }
    Ok(PageAnalyzer::new(info))
}

fn normalize_stream_urls(stream: &mut Value) {
    if let Some(stream) = stream.as_object_mut() {
        for (snake_case, camel_case) in [("base_url", "baseUrl"), ("backup_url", "backupUrl")] {
            if !stream.contains_key(camel_case)
                && let Some(value) = stream.get(snake_case).cloned()
            {
                stream.insert(camel_case.to_owned(), value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_bangumi_target() {
        for input in [
            "ep32367",
            " https://www.bilibili.com/bangumi/play/ep32367?from=search#reply ",
        ] {
            assert_eq!(input.parse::<BangumiTarget>().unwrap(), BangumiTarget::Episode(32367));
        }
        assert_eq!(
            "http://m.bilibili.com/bangumi/play/ss1733/"
                .parse::<BangumiTarget>()
                .unwrap(),
            BangumiTarget::Season(1733)
        );
        for input in [
            "32367",
            "ep0",
            "ep-1",
            "ss",
            "ep32367junk",
            "ep9223372036854775808",
            "https://evil.example/bangumi/play/ep32367",
            "https://www.bilibili.com/video/ep32367",
            "https://user@bilibili.com/bangumi/play/ep32367",
        ] {
            assert!(input.parse::<BangumiTarget>().is_err(), "accepted {input}");
        }
    }

    fn season_response() -> Value {
        json!({"code": 0, "result": {"season_id": 1733, "title": "罗小黑战记", "episodes": [{
            "id": 32367, "ep_id": 32367, "bvid": "BV1Rs411f7ZM", "cid": 34474885773_i64,
            "title": "8", "long_title": "深山", "duration": 417666, "pub_time": 1378868400
        }]}})
    }

    #[test]
    fn preview_exposes_season_stats_and_selected_episode() {
        let mut response = season_response();
        response["result"]["cover"] = json!("//i0.hdslb.com/bfs/bangumi/cover.png");
        response["result"]["evaluate"] = json!("番剧简介");
        response["result"]["total"] = json!(0);
        response["result"]["stat"] =
            json!({"views": 1002161540_u64, "favorites": 8853571, "follow_text": "981.9万系列追番"});
        response["result"]["rating"] = json!({"score": 9.9});
        response["result"]["publish"] = json!({
            "pub_time_show": "2011年03月17日00:00", "pub_time": "2011-03-17 00:00:00"
        });
        response["result"]["new_ep"] = json!({"desc": "每周五12:00更新1集"});
        let preview = parse_season(response, BangumiTarget::Episode(32367))
            .unwrap()
            .preview(BangumiTarget::Episode(32367))
            .unwrap();
        let preview = serde_json::to_value(preview).unwrap();
        assert_eq!(preview["seasonId"], 1733);
        assert_eq!(preview["title"], "罗小黑战记");
        assert_eq!(preview["cover"], "https://i0.hdslb.com/bfs/bangumi/cover.png");
        assert_eq!(preview["description"], "番剧简介");
        assert_eq!(preview["views"], 1002161540_u64);
        assert_eq!(preview["followers"], 8853571);
        assert_eq!(preview["followText"], "981.9万系列追番");
        assert_eq!(preview["rating"], 9.9);
        assert_eq!(preview["pubTime"], "2011年03月17日00:00");
        assert_eq!(preview["status"], "每周五12:00更新1集");
        assert!(preview["totalEpisodes"].is_null());
        assert_eq!(preview["publishedEpisodes"], 1);
        assert_eq!(preview["episode"]["id"], 32367);
        assert_eq!(preview["episode"]["longTitle"], "深山");
        assert_eq!(preview["url"], "https://www.bilibili.com/bangumi/play/ep32367");
    }

    #[test]
    fn preview_distinguishes_missing_statistics_from_zero() {
        let preview = parse_season(season_response(), BangumiTarget::Season(1733))
            .unwrap()
            .preview(BangumiTarget::Season(1733))
            .unwrap();
        assert_eq!(preview.views, None);
        assert_eq!(preview.followers, None);
        assert_eq!(preview.rating, None);
        assert_eq!(preview.pub_time, None);
        assert_eq!(preview.total_episodes, None);
        assert!(preview.episode.is_none());
        assert!(preview.cover.is_empty());
        assert_eq!(preview.url, "https://www.bilibili.com/bangumi/play/ss1733");

        let mut response = season_response();
        response["result"]["stat"] = json!({"views": 0, "favorites": 0, "follow_text": ""});
        response["result"]["new_ep"] = json!({"desc": " "});
        response["result"]["cover"] = json!("javascript:alert(1)");
        let preview = parse_season(response, BangumiTarget::Season(1733))
            .unwrap()
            .preview(BangumiTarget::Season(1733))
            .unwrap();
        assert_eq!(preview.views, Some(0));
        assert_eq!(preview.followers, Some(0));
        assert_eq!(preview.follow_text, None);
        assert_eq!(preview.status, None);
        assert!(preview.cover.is_empty());
    }

    #[test]
    fn publish_time_falls_back_and_unknown_values_remain_missing() {
        for (publish, expected) in [
            (
                json!({"pub_time_show":" 2025年5月23日 ", "pub_time":"2025-05-23 11:00:00"}),
                Some("2025年5月23日"),
            ),
            (
                json!({"pub_time_show":" ", "pub_time":"2025-05-23 11:00:00"}),
                Some("2025-05-23 11:00:00"),
            ),
            (
                json!({"pub_time_show":"0000年00月00日", "pub_time":"2025-05-23 11:00:00"}),
                Some("2025-05-23 11:00:00"),
            ),
            (json!({"pub_time_show":"待定", "pub_time":"0000-00-00 00:00:00"}), None),
            (json!({"pub_time_show":"", "pub_time":null}), None),
            (json!({}), None),
        ] {
            let publish: BangumiPublish = serde_json::from_value(publish).unwrap();
            assert_eq!(publish.display_time().as_deref(), expected);
        }
        for (score, expected) in [
            (Some(9.8), Some(9.8)),
            (Some(0.0), Some(0.0)),
            (None, None),
            (Some(-1.0), None),
            (Some(10.1), None),
        ] {
            assert_eq!(BangumiRating { score }.value(), expected);
        }
    }

    #[test]
    fn preview_counts_published_main_list_without_trailers_or_duplicate_extras() {
        let mut response = season_response();
        let episode = response["result"]["episodes"][0].clone();
        let mut trailer = episode.clone();
        trailer["id"] = json!(7340794);
        trailer["section_type"] = json!(1);
        let mut future = episode.clone();
        future["id"] = json!(999999);
        future["cid"] = json!(0);
        let mut extra = episode.clone();
        extra["id"] = json!(394759);
        response["result"]["episodes"] = json!([episode, episode, trailer, future]);
        response["result"]["section"] = json!([{"episodes": [extra]}]);
        response["result"]["total"] = json!(12);
        let preview = parse_season(response, BangumiTarget::Episode(394759))
            .unwrap()
            .preview(BangumiTarget::Episode(394759))
            .unwrap();
        assert_eq!(preview.total_episodes, Some(12));
        assert_eq!(preview.published_episodes, 1);
        assert_eq!(preview.episode.unwrap().id, 394759);
    }

    #[tokio::test]
    #[ignore = "requires Bilibili network access; reads preview metadata only"]
    async fn live_bangumi_preview_ep32367() -> Result<()> {
        let client = BiliClient::new();
        let credential = Credential::default();
        let preview = Bangumi::new(&client, &credential)
            .get_preview(BangumiTarget::Episode(32367))
            .await?;
        assert_eq!(preview.season_id, 1733);
        assert_eq!(preview.title, "罗小黑战记");
        assert!(preview.cover.starts_with("https://"));
        assert!(preview.views.is_some_and(|views| views > 0));
        assert!(preview.followers.is_some_and(|followers| followers > 0));
        assert!(preview.published_episodes > 0);
        assert!(preview.pub_time.is_some_and(|time| !time.is_empty()));
        assert_eq!(preview.episode.unwrap().long_title, "深山");
        Ok(())
    }

    #[test]
    fn maps_episode_metadata_without_truncating_cid_or_duration() {
        let season = parse_season(season_response(), BangumiTarget::Episode(32367)).unwrap();
        let VideoInfo::Detail {
            title,
            pages,
            redirect_url,
            ..
        } = season.episodes[0].to_video_info(&season).unwrap()
        else {
            panic!()
        };
        assert_eq!(title, "罗小黑战记 第8话 深山");
        assert_eq!(pages[0].cid, 34474885773);
        assert_eq!(pages[0].duration, 418);
        assert_eq!(
            redirect_url.as_deref(),
            Some("https://www.bilibili.com/bangumi/play/ep32367")
        );
        assert!(parse_season(season_response(), BangumiTarget::Episode(999)).is_err());
        assert!(parse_season(season_response(), BangumiTarget::Season(999)).is_err());
    }

    #[test]
    fn seasonal_download_excludes_extras_but_direct_episode_can_select_them() {
        let mut response = season_response();
        let mut extra = response["result"]["episodes"][0].clone();
        extra["id"] = json!(394759);
        response["result"]["section"] = json!([{"episodes": [extra]}]);
        assert_eq!(
            parse_season(response.clone(), BangumiTarget::Season(1733))
                .unwrap()
                .episodes
                .len(),
            1
        );
        let season = parse_season(response, BangumiTarget::Episode(394759)).unwrap();
        assert!(season.episodes.iter().any(|ep| ep.id == 394759));
    }

    #[test]
    fn rejects_partial_encrypted_and_unavailable_streams() {
        let success = json!({"code": 0, "result": {"code": 0, "is_preview": 0, "is_drm": false, "dash": {"video": [{"id": 32}]}}});
        assert!(parse_playurl(success.clone()).is_ok());
        for (field, value) in [
            ("is_preview", json!(1)),
            ("is_preview", json!(true)),
            ("is_drm", json!(true)),
            ("drm_tech_type", json!(2)),
            ("code", json!(-10403)),
            ("error_code", json!(-10403)),
        ] {
            let mut response = success.clone();
            response["result"][field] = value;
            assert!(parse_playurl(response).is_err(), "accepted {field}");
        }
        assert!(parse_playurl(json!({"code": -10403, "message": "大会员专享限制"})).is_err());
        assert!(parse_playurl(json!({"code": 0, "result": {"dash": {"video": []}}})).is_err());
        assert!(parse_playurl(json!({"code": 0, "result": {"durl": [{"url":"part1"}, {"url":"part2"}]}})).is_err());
    }

    #[test]
    fn selects_pgc_dash_streams_with_empty_durl_and_snake_case_urls() {
        use super::super::analyzer::{AudioQuality, BestStream, FilterOption, Stream, VideoQuality};

        for empty_durl in [Value::Null, json!([])] {
            let response = json!({"code": 0, "result": {
                "code": 0, "error_code": 0, "format": "flv480", "is_preview": 0, "is_drm": false,
                "durl": empty_durl, "dash": {
                    "duration": 418,
                    "video": [
                        {"id": 16, "codecid": 7, "base_url": "https://cdn.example/360.m4s"},
                        {"id": 32, "codecid": 7, "base_url": "https://cdn.example/video.m4s",
                         "backup_url": ["https://backup.example/video.m4s"]}
                    ],
                    "audio": [{"id": 30280, "base_url": "https://cdn.example/audio.m4s",
                               "backup_url": ["https://backup.example/audio.m4s"]}]
                }
            }});
            let best = parse_playurl(response)
                .unwrap()
                .best_stream(&FilterOption::default())
                .unwrap();
            let BestStream::VideoAudio {
                video,
                audio: Some(audio),
            } = best
            else {
                panic!("expected DASH video and audio")
            };
            assert!(matches!(
                &video,
                Stream::DashVideo {
                    quality: VideoQuality::Quality480p,
                    ..
                }
            ));
            assert!(matches!(
                &audio,
                Stream::DashAudio {
                    quality: AudioQuality::Quality192k,
                    ..
                }
            ));
            assert_eq!(
                video.urls(false),
                vec!["https://cdn.example/video.m4s", "https://backup.example/video.m4s"]
            );
            assert_eq!(
                audio.urls(false),
                vec!["https://cdn.example/audio.m4s", "https://backup.example/audio.m4s"]
            );
        }
    }

    #[test]
    fn preserves_camel_case_urls_and_normalizes_optional_audio() {
        use super::super::analyzer::{AudioQuality, BestStream, FilterOption, Stream};

        let response = json!({"code": 0, "result": {"dash": {
            "video": [{"id": 32, "codecid": 7, "baseUrl": "https://camel.example/video.m4s",
                       "base_url": "https://snake.example/video.m4s", "backupUrl": [],
                       "backup_url": ["https://snake.example/backup.m4s"]}],
            "audio": null,
            "flac": {"audio": {"id": 30251, "base_url": "https://cdn.example/flac.m4s"}},
            "dolby": {"audio": [{"id": 30250, "base_url": "https://cdn.example/dolby.m4s"}]}
        }}});
        let best = parse_playurl(response.clone())
            .unwrap()
            .best_stream(&FilterOption::default())
            .unwrap();
        let BestStream::VideoAudio {
            video,
            audio: Some(audio),
        } = best
        else {
            panic!("expected DASH video and audio")
        };
        assert_eq!(video.urls(false), vec!["https://camel.example/video.m4s"]);
        assert!(matches!(
            audio,
            Stream::DashAudio {
                quality: AudioQuality::QualityHiRES,
                ..
            }
        ));
        let best = parse_playurl(response)
            .unwrap()
            .best_stream(&FilterOption {
                no_hires: true,
                ..Default::default()
            })
            .unwrap();
        let BestStream::VideoAudio { audio: Some(audio), .. } = best else {
            panic!("expected Dolby audio")
        };
        assert!(matches!(
            &audio,
            Stream::DashAudio {
                quality: AudioQuality::QualityDolby,
                ..
            }
        ));
        assert_eq!(audio.urls(false), vec!["https://cdn.example/dolby.m4s"]);
    }
}
