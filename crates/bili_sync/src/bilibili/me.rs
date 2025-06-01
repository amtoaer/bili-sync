#![allow(unused)]

use anyhow::Result;
use reqwest::Method;

use crate::bilibili::{BiliClient, Validate};
use crate::config::CONFIG;
pub struct Me<'a> {
    client: &'a BiliClient,
    mid: String,
}

impl<'a> Me<'a> {
    pub fn new(client: &'a BiliClient) -> Self {
        Self {
            client,
            mid: Self::my_id(),
        }
    }

    pub async fn get_created_favorites(&self) -> Result<Option<Vec<FavoriteItem>>> {
        let mut resp = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/v3/fav/folder/created/list-all")
            .await
            .query(&[("up_mid", &self.mid)])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(resp["data"]["list"].take())?)
    }

    pub async fn get_followed_collections(&self, page: i32) -> Result<Collections> {
        let mut resp = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/v3/fav/folder/collected/list")
            .await
            .query(&[
                ("up_mid", self.mid.as_ref()),
                ("pn", page.to_string().as_ref()),
                ("ps", "20"),
                ("platform", "web"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(resp["data"].take())?)
    }

    pub async fn get_followed_uppers(&self, page: i32) -> Result<FollowedUppers> {
        let mut resp = self
            .client
            .request(Method::GET, "https://api.bilibili.com/x/relation/followings")
            .await
            .query(&[
                ("vmid", self.mid.as_ref()),
                ("pn", page.to_string().as_ref()),
                ("ps", "20"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(resp["data"].take())?)
    }

    fn my_id() -> String {
        CONFIG
            .credential
            .load()
            .as_deref()
            .map(|c| c.dedeuserid.clone())
            .unwrap()
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct FavoriteItem {
    pub title: String,
    pub media_count: i64,
    pub fid: i64,
    pub mid: i64,
}

#[derive(Debug, serde::Deserialize)]
pub struct CollectionItem {
    pub id: i64,
    pub mid: i64,
    pub state: i32,
    pub title: String,
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
