use anyhow::{Context, Result, ensure};
use reqwest::{Method, Url};

use super::bangumi::{BangumiPublish, BangumiRating};
use crate::bilibili::{BiliClient, Credential, ErrorForStatusExt, Validate};

pub struct Me<'a> {
    client: &'a BiliClient,
    credential: &'a Credential,
}

impl<'a> Me<'a> {
    pub fn new(client: &'a BiliClient, credential: &'a Credential) -> Self {
        Self { client, credential }
    }

    pub async fn get_created_favorites(&self) -> Result<Option<Vec<FavoriteItem>>> {
        ensure!(
            !self.mid().is_empty(),
            "未获取到用户 ID，请确保填写设置中的 B 站认证信息"
        );
        let mut resp = self
            .client
            .request(
                Method::GET,
                "https://api.bilibili.com/x/v3/fav/folder/created/list-all",
                self.credential,
            )
            .await
            .query(&[("up_mid", &self.mid())])
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(resp["data"]["list"].take())?)
    }

    pub async fn get_followed_collections(&self, page_num: i32, page_size: i32) -> Result<Collections> {
        ensure!(
            !self.mid().is_empty(),
            "未获取到用户 ID，请确保填写设置中的 B 站认证信息"
        );
        let mut resp = self
            .client
            .request(
                Method::GET,
                "https://api.bilibili.com/x/v3/fav/folder/collected/list",
                self.credential,
            )
            .await
            .query(&[("up_mid", self.mid()), ("platform", "web")])
            .query(&[("pn", page_num), ("ps", page_size)])
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(resp["data"].take())?)
    }

    pub async fn get_followed_uppers(
        &self,
        page_num: i32,
        page_size: i32,
        name: Option<&str>,
    ) -> Result<FollowedUppers> {
        ensure!(
            !self.mid().is_empty(),
            "未获取到用户 ID，请确保填写设置中的 B 站认证信息"
        );
        let url = if name.is_some() {
            "https://api.bilibili.com/x/relation/followings/search"
        } else {
            "https://api.bilibili.com/x/relation/followings"
        };
        let mut request = self
            .client
            .request(Method::GET, url, self.credential)
            .await
            .query(&[("vmid", self.mid())])
            .query(&[("pn", page_num), ("ps", page_size)]);
        if let Some(name) = name {
            request = request.query(&[("name", name)]);
        }
        let mut resp = request
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(resp["data"].take())?)
    }

    pub async fn get_followed_bangumi(&self, page_num: i32, page_size: i32) -> Result<FollowedBangumi> {
        ensure!(!self.mid().is_empty(), "未获取到用户 ID，请先在设置中登录 B 站账号");
        ensure!(page_num >= 1 && (1..=50).contains(&page_size), "番剧分页参数无效");
        let response = self
            .client
            .request(
                Method::GET,
                "https://api.bilibili.com/x/space/bangumi/follow/list",
                self.credential,
            )
            .await
            .query(&[("vmid", self.mid())])
            // type=1 includes both 番剧 and 国创; type=2 is 追剧.
            .query(&[("type", 1), ("follow_status", 0), ("pn", page_num), ("ps", page_size)])
            .send()
            .await?
            .error_for_status_ext()?
            .json::<serde_json::Value>()
            .await?;
        parse_followed_bangumi(response)
    }

    fn mid(&self) -> &str {
        &self.credential.dedeuserid
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct FavoriteItem {
    pub title: String,
    pub media_count: i64,
    pub id: i64,
    pub mid: i64,
}

#[derive(Debug, serde::Deserialize)]
pub struct CollectionItem {
    pub id: i64,
    pub fid: i64,
    pub mid: i64,
    pub state: i32,
    pub title: String,
    pub media_count: i64,
}

#[derive(Debug, serde::Deserialize)]
pub struct Collections {
    pub count: i64,
    pub list: Option<Vec<CollectionItem>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct FollowedUppers {
    pub total: i64,
    pub list: Vec<FollowedUpper>,
}

#[derive(Debug, serde::Deserialize)]
pub struct FollowedUpper {
    pub mid: i64,
    pub uname: String,
    pub face: String,
    pub sign: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct FollowedBangumi {
    pub total: i64,
    pub list: Option<Vec<FollowedBangumiItem>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct FollowedBangumiItem {
    pub season_id: i64,
    pub title: String,
    #[serde(default)]
    pub cover: String,
    pub total_count: Option<i64>,
    pub new_ep: Option<FollowedBangumiEpisode>,
    pub progress: Option<String>,
    pub publish: Option<BangumiPublish>,
    pub rating: Option<BangumiRating>,
}

#[derive(Debug, serde::Deserialize)]
pub struct FollowedBangumiEpisode {
    pub index_show: Option<String>,
}

fn parse_followed_bangumi(response: serde_json::Value) -> Result<FollowedBangumi> {
    let mut response = response.validate()?;
    ensure!(response["data"].get("list").is_some(), "追番接口未返回有效列表");
    let mut followed: FollowedBangumi =
        serde_json::from_value(response["data"].take()).context("追番接口未返回有效列表")?;
    ensure!(followed.total >= 0, "追番接口返回无效总数");
    for item in followed.list.iter_mut().flatten() {
        let cover = if item.cover.starts_with("//") {
            format!("https:{}", item.cover)
        } else {
            item.cover.replacen("http://", "https://", 1)
        };
        item.cover = Url::parse(&cover)
            .ok()
            .filter(|url| url.scheme() == "https" && url.host_str().is_some())
            .map(|url| url.to_string())
            .unwrap_or_default();
        item.total_count = item.total_count.filter(|count| *count > 0);
        item.progress = item.progress.take().filter(|progress| !progress.trim().is_empty());
        if let Some(episode) = &mut item.new_ep {
            episode.index_show = episode.index_show.take().filter(|text| !text.trim().is_empty());
        }
    }
    Ok(followed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn followed_bangumi_parses_public_api_fields_without_treating_vip_as_invalid() {
        let data = parse_followed_bangumi(json!({"code":0,"data":{"total":2,"list":[
            {"season_id":40551,"title":"奶龙","cover":"http://i0.hdslb.com/cover.png",
             "total_count":150,"season_status":13,"badge":"大会员",
             "publish":{"pub_time_show":"2022年1月1日", "pub_time":"2022-01-01 00:00:01"},
             "rating":{"score":9.8},
             "new_ep":{"index_show":"更新至第248话"},"progress":"看到第1话"},
            {"season_id":1733,"title":"罗小黑战记","cover":"//i0.hdslb.com/cover.png",
             "total_count":0,"new_ep":{"index_show":" "},"progress":""}
        ]}}))
        .unwrap();
        assert_eq!(data.total, 2);
        let list = data.list.unwrap();
        assert_eq!(list[0].season_id, 40551);
        assert_eq!(list[0].cover, "https://i0.hdslb.com/cover.png");
        assert_eq!(list[0].total_count, Some(150));
        assert_eq!(list[0].progress.as_deref(), Some("看到第1话"));
        assert_eq!(
            list[0]
                .publish
                .as_ref()
                .and_then(BangumiPublish::display_time)
                .as_deref(),
            Some("2022年1月1日")
        );
        assert_eq!(list[0].rating.as_ref().and_then(BangumiRating::value), Some(9.8));
        assert_eq!(
            list[0].new_ep.as_ref().unwrap().index_show.as_deref(),
            Some("更新至第248话")
        );
        assert_eq!(list[1].total_count, None);
        assert_eq!(list[1].progress, None);
        assert!(list[1].publish.is_none());
        assert!(list[1].rating.is_none());
        assert_eq!(list[1].cover, "https://i0.hdslb.com/cover.png");
        assert_eq!(list[1].new_ep.as_ref().unwrap().index_show, None);
    }

    #[test]
    fn followed_bangumi_distinguishes_empty_list_from_upstream_failure() {
        for list in [json!([]), serde_json::Value::Null] {
            let data = parse_followed_bangumi(json!({"code":0,"data":{"total":0,"list":list}})).unwrap();
            assert_eq!(data.total, 0);
            assert!(data.list.unwrap_or_default().is_empty());
        }
        assert!(parse_followed_bangumi(json!({"code":-101,"message":"账号未登录"})).is_err());
        assert!(parse_followed_bangumi(json!({"code":-352,"message":"风控校验失败"})).is_err());
        assert!(parse_followed_bangumi(json!({"code":0,"data":null})).is_err());
        assert!(parse_followed_bangumi(json!({"code":0,"data":{"total":0}})).is_err());
    }

    #[tokio::test]
    async fn followed_bangumi_rejects_missing_account_before_network() {
        let client = BiliClient::new();
        let credential = Credential::default();
        assert!(Me::new(&client, &credential).get_followed_bangumi(1, 20).await.is_err());
    }

    #[tokio::test]
    #[ignore = "reads the public follow list of UID 2; requires Bilibili network access"]
    async fn live_followed_bangumi_public_list() -> Result<()> {
        let client = BiliClient::new();
        let credential = Credential {
            dedeuserid: "2".to_owned(),
            ..Default::default()
        };
        let list = Me::new(&client, &credential).get_followed_bangumi(1, 2).await?;
        assert!(list.total > 0);
        let items = list.list.unwrap_or_default();
        assert!(!items.is_empty() && items.len() <= 2);
        for item in items {
            assert!(item.season_id > 0);
            assert!(!item.title.is_empty());
            assert!(item.cover.is_empty() || item.cover.starts_with("https://"));
            assert!(item.publish.as_ref().and_then(BangumiPublish::display_time).is_some());
        }
        Ok(())
    }
}
