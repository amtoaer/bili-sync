use thiserror::Error;

#[derive(Error, Debug)]
#[error("Bilibili api request too frequently, abort all tasks and try again later")]
pub struct DownloadAbortError();
