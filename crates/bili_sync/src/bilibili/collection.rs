#![allow(dead_code)]

use anyhow::Result;
use async_stream::stream;
use futures::Stream;

use crate::bilibili::BiliClient;

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum CollectionItem {
    Series(String),
    Season(String),
}

pub struct Collection<'a> {
    client: &'a BiliClient,
    pub collection: &'a CollectionItem,
}

pub struct CollectionInfo {}

pub struct SimpleVideoInfo {}

impl<'a> Collection<'a> {
    pub fn new(client: &'a BiliClient, collection: &'a CollectionItem) -> Self {
        Self { client, collection }
    }

    pub fn get_info(&self) -> Result<CollectionInfo> {
        unimplemented!()
    }

    pub async fn into_simple_video_stream(self) -> impl Stream<Item = SimpleVideoInfo> + 'a {
        stream! {
            yield SimpleVideoInfo{}
        }
    }
}
