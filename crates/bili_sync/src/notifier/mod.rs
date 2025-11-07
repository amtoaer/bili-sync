use anyhow::Result;
use futures::future;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Notifier {
    Telegram { bot_token: String, chat_id: String },
    Webhook { url: String },
}

#[derive(Serialize)]
struct WebhookPayload<'a> {
    text: &'a str,
}

pub trait NotifierAllExt {
    async fn notify_all(&self, client: &reqwest::Client, message: &str) -> Result<()>;
}

impl NotifierAllExt for Option<Vec<Notifier>> {
    async fn notify_all(&self, client: &reqwest::Client, message: &str) -> Result<()> {
        let Some(notifiers) = self else {
            return Ok(());
        };
        future::join_all(notifiers.iter().map(|notifier| notifier.notify(client, message))).await;
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
            Notifier::Webhook { url } => {
                let payload = WebhookPayload { text: message };
                client.post(url).json(&payload).send().await?;
            }
        }
        Ok(())
    }
}
