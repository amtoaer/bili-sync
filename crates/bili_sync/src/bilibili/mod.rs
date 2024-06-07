pub use analyzer::{BestStream, FilterOption};
use anyhow::{bail, Result};
pub use client::{BiliClient, Client};
pub use credential::Credential;
pub use danmaku::DanmakuOption;
pub use error::BiliError;
pub use favorite_list::{FavoriteList, FavoriteListInfo, VideoInfo};
pub use video::{Dimension, PageInfo, Video};

mod analyzer;
mod client;
mod credential;
mod danmaku;
mod error;
mod favorite_list;
mod video;

pub(crate) trait Validate {
    type Output;

    fn validate(self) -> Result<Self::Output>;
}

impl Validate for serde_json::Value {
    type Output = serde_json::Value;

    fn validate(self) -> Result<Self::Output> {
        let (code, msg) = match (self["code"].as_i64(), self["message"].as_str()) {
            (Some(code), Some(msg)) => (code, msg),
            _ => bail!("no code or message found"),
        };
        if code != 0 {
            bail!(BiliError::RequestFailed(code, msg.to_owned()));
        }
        Ok(self)
    }
}
