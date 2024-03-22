use std::error;

pub mod bilibili;
pub mod command;
pub mod database;
pub mod downloader;

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;
