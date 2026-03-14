use std::borrow::Cow;

use itertools::Itertools;
use serde::Serialize;

use crate::notifier::DownloadNotifyInfo;

#[derive(Serialize)]
pub struct Message<'a> {
    pub message: Cow<'a, str>,
    pub image_url: Option<String>,
}

impl<'a> From<&'a str> for Message<'a> {
    fn from(message: &'a str) -> Self {
        Self {
            message: Cow::Borrowed(message),
            image_url: None,
        }
    }
}

impl From<String> for Message<'_> {
    fn from(message: String) -> Self {
        Self {
            message: message.into(),
            image_url: None,
        }
    }
}

impl From<DownloadNotifyInfo> for Message<'_> {
    fn from(info: DownloadNotifyInfo) -> Self {
        match info {
            DownloadNotifyInfo::List {
                source,
                img_url,
                titles,
            } => Self {
                message: format!(
                    "{}的 {} 条新视频已入库：\n{}",
                    source,
                    titles.len(),
                    titles
                        .into_iter()
                        .enumerate()
                        .map(|(i, title)| format!("{}. {title}", i + 1))
                        .join("\n")
                )
                .into(),
                image_url: img_url,
            },
            DownloadNotifyInfo::Summary { source, img_url, count } => Self {
                message: format!("{}的 {} 条新视频已入库，快去看看吧！", source, count).into(),
                image_url: img_url,
            },
        }
    }
}
