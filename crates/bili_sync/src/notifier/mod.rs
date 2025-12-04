use anyhow::Result;
use futures::future;
use reqwest::header;
use serde::{Deserialize, Serialize};

use crate::config::TEMPLATE;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Notifier {
    Telegram {
        bot_token: String,
        chat_id: String,
    },
    Webhook {
        url: String,
        template: Option<String>,
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
    async fn notify_all(&self, client: &reqwest::Client, message: &str) -> Result<()>;
}

impl NotifierAllExt for Vec<Notifier> {
    async fn notify_all(&self, client: &reqwest::Client, message: &str) -> Result<()> {
        future::join_all(self.iter().map(|notifier| notifier.notify(client, message))).await;
        Ok(())
    }
}

impl Notifier {
    pub async fn notify(&self, client: &reqwest::Client, message: &str) -> Result<()> {
        match self {
            Notifier::Telegram { bot_token, chat_id } => {
                let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
                let params = [("chat_id", chat_id.as_str()), ("text", message)];
                client.post(&url).form(&params).send().await?;
            }
            Notifier::Webhook {
                url,
                template,
                ignore_cache,
            } => {
                let key = webhook_template_key(url);
                let data = serde_json::json!(
                    {
                        "message": message,
                    }
                );
                let handlebar = TEMPLATE.read();
                let payload = match ignore_cache {
                    Some(_) => handlebar.render_template(webhook_template_content(template), &data)?,
                    None => handlebar.render(&key, &data)?,
                };
                client
                    .post(url)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(payload)
                    .send()
                    .await?;
            }
        }
        Ok(())
    }
}
