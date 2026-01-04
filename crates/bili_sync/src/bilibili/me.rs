use anyhow::{Result, ensure};
use reqwest::Method;

use crate::bilibili::{BiliClient, Credential, Validate};

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
            .error_for_status()?
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
            .error_for_status()?
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
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(resp["data"].take())?)
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
