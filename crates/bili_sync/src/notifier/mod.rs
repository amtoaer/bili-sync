mod info;
mod message;

use std::collections::HashMap;

use anyhow::Result;
use futures::future;
pub use info::DownloadNotifyInfo;
pub use message::Message;
use reqwest::header;
use serde::{Deserialize, Serialize};

use crate::config::TEMPLATE;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Notifier {
    Telegram {
        bot_token: String,
        chat_id: String,
        #[serde(default)]
        skip_image: bool,
    },
    Webhook {
        url: String,
        template: Option<String>,
        #[serde(default)]
        headers: Option<HashMap<String, String>>,
        #[serde(skip)]
        // 一个内部辅助字段，用于决定是否强制渲染当前模板，在测试时使用
        ignore_cache: Option<()>,
    },
}

pub fn webhook_template_key(url: &str) -> String {
    format!("payload_{}", url)
}

pub fn webhook_template_content(template: &Option<String>) -> &str {
    template
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or(r#"{"text": "{{{message}}}"}"#)
}

pub trait NotifierAllExt {
    async fn notify_all<'a>(&self, client: &reqwest::Client, message: impl Into<Message<'a>>) -> Result<()>;
}

impl NotifierAllExt for Vec<Notifier> {
    async fn notify_all<'a>(&self, client: &reqwest::Client, message: impl Into<Message<'a>>) -> Result<()> {
        let message = message.into();
        future::join_all(self.iter().map(|notifier| notifier.notify_internal(client, &message))).await;
        Ok(())
    }
}

impl Notifier {
    pub async fn notify<'a>(&self, client: &reqwest::Client, message: impl Into<Message<'a>>) -> Result<()> {
        self.notify_internal(client, &message.into()).await
    }

    async fn notify_internal<'a>(&self, client: &reqwest::Client, message: &Message<'a>) -> Result<()> {
        match self {
            Notifier::Telegram {
                bot_token,
                chat_id,
                skip_image,
            } => {
                if let Some(img_url) = &message.image_url
                    && !*skip_image
                {
                    let url = format!("https://api.telegram.org/bot{}/sendPhoto", bot_token);
                    let params = [
                        ("chat_id", chat_id.as_str()),
                        ("photo", img_url.as_str()),
                        ("caption", message.message.as_ref()),
                    ];
                    client.post(&url).form(&params).send().await?;
                } else {
                    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
                    let params = [("chat_id", chat_id.as_str()), ("text", message.message.as_ref())];
                    client.post(&url).form(&params).send().await?;
                }
            }
            Notifier::Webhook {
                url,
                template,
                headers,
                ignore_cache,
            } => {
                let key = webhook_template_key(url);
                let handlebar = TEMPLATE.read();
                let payload = match ignore_cache {
                    Some(_) => handlebar.render_template(webhook_template_content(template), &message)?,
                    None => handlebar.render(&key, &message)?,
                };
                let mut headers_map = header::HeaderMap::new();
                headers_map.insert(header::CONTENT_TYPE, "application/json".try_into()?);

                if let Some(custom_headers) = headers {
                    for (key, value) in custom_headers {
                        if let (Ok(key), Ok(value)) =
                            (header::HeaderName::try_from(key), header::HeaderValue::try_from(value))
                        {
                            headers_map.insert(key, value);
                        }
                    }
                }

                client.post(url).headers(headers_map).body(payload).send().await?;
            }
        }
        Ok(())
    }
}
