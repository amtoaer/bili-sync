use anyhow::Result;
use futures::future;
use serde::{Deserialize, Serialize};

use crate::config::TEMPLATE;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Notifier {
    Telegram { bot_token: String, chat_id: String },
    Webhook { url: String, template: Option<String> },
}

pub const DEFAULT_WEBHOOK_PAYLOAD: &str = r#"{
    "text": "{{{message}}}"
}"#;

pub fn webhook_template_key(url: &str) -> String {
    format!("payload_{}", url)
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
            Notifier::Webhook { url, .. } => {
                let payload = TEMPLATE.read().render(
                    &webhook_template_key(url),
                    &serde_json::json!(
                        {
                            "message": message,
                        }
                    ),
                )?;
                client
                    .post(url)
                    .header("Content-Type", "application/json")
                    .body(payload)
                    .send()
                    .await?;
            }
        }
        Ok(())
    }
}
