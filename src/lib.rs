use std::error;

pub mod bilibili;
pub mod core;
pub mod database;
pub mod downloader;

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;
