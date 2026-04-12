use std::borrow::Cow;

use itertools::Itertools;
use serde::Serialize;

use crate::notifier::DownloadNotifyInfo;

#[derive(Serialize)]
pub struct Message<'a> {
    pub message: Cow<'a, str>,
    pub image_url: Option<String>,
}

impl Message<'_> {
    pub fn serverchan3_title(&self) -> Cow<'_, str> {
        let first_line = self
            .message
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(str::trim)
            .unwrap_or_else(|| self.message.trim());

        if first_line.is_empty() {
            "BiliSync 通知".into()
        } else {
            first_line.into()
        }
    }

    pub fn serverchan3_desp(&self) -> Cow<'_, str> {
        match &self.image_url {
            Some(image_url) if !image_url.trim().is_empty() => format!("{}\n\n![]({image_url})", self.message).into(),
            _ => Cow::Borrowed(self.message.as_ref()),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serverchan3_uses_first_non_empty_line_as_title() {
        let message = Message {
            message: Cow::Borrowed("\n下载失败\n详细错误"),
            image_url: None,
        };

        assert_eq!(message.serverchan3_title().as_ref(), "下载失败");
        assert_eq!(message.serverchan3_desp().as_ref(), "\n下载失败\n详细错误");
    }

    #[test]
    fn serverchan3_appends_image_markdown_when_image_exists() {
        let message = Message {
            message: Cow::Borrowed("新视频已入库"),
            image_url: Some("https://example.com/poster.jpg".to_owned()),
        };

        assert_eq!(message.serverchan3_title().as_ref(), "新视频已入库");
        assert_eq!(
            message.serverchan3_desp().as_ref(),
            "新视频已入库\n\n![](https://example.com/poster.jpg)"
        );
    }

    #[test]
    fn serverchan3_title_falls_back_when_message_is_blank() {
        let message = Message {
            message: Cow::Borrowed(" \n "),
            image_url: None,
        };

        assert_eq!(message.serverchan3_title().as_ref(), "BiliSync 通知");
    }
}
