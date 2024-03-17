mod analyzer;
mod client;
mod favorite_list;
mod video;

use std::error;

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

pub use analyzer::{AudioQuality, PageAnalyzer, VideoCodecs, VideoQuality};
pub use client::{BiliClient, Credential};
pub use favorite_list::FavoriteList;
pub use video::Video;
