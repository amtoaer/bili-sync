mod info;
mod message;

use std::collections::HashMap;

use anyhow::{Result, bail};
use futures::future;
pub use info::DownloadNotifyInfo;
pub use message::Message;
use reqwest::header;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

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
    #[serde(rename = "serverChan3", alias = "serverchan3")]
    ServerChan3 {
        #[serde(flatten)]
        config: ServerChan3Config,
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
            Notifier::ServerChan3 { config } => {
                let send_url = serverchan3_send_url(&config.sendkey)?;
                let title = message.serverchan3_title();
                let desp = message.serverchan3_desp();
                let payload = serde_json::json!({
                    "title": title.as_ref(),
                    "desp": desp.as_ref(),
                });
                client.post(send_url).json(&payload).send().await?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerChan3Config {
    pub sendkey: String,
}

impl<'de> Deserialize<'de> for ServerChan3Config {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawServerChan3Config {
            sendkey: Option<String>,
            #[serde(alias = "sendUrl", alias = "send_url")]
            send_url: Option<String>,
        }

        let raw = RawServerChan3Config::deserialize(deserializer)?;
        if let Some(sendkey) = raw.sendkey {
            return Ok(Self { sendkey });
        }
        if let Some(send_url) = raw.send_url
            && let Some(sendkey) = extract_serverchan3_sendkey(&send_url)
        {
            return Ok(Self { sendkey });
        }
        Err(D::Error::custom("missing valid Server酱³ sendkey"))
    }
}

fn extract_serverchan3_sendkey(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if is_valid_serverchan3_sendkey(input) {
        return Some(input.to_owned());
    }

    let lower = input.to_ascii_lowercase();
    let send_path = "/send/";
    if let Some(start) = lower.find(send_path) {
        let after = &input[start + send_path.len()..];
        let end = after.find(".send").unwrap_or(after.len());
        let candidate = after[..end].trim();
        if is_valid_serverchan3_sendkey(candidate) {
            return Some(candidate.to_owned());
        }
    }

    let host = ".push.ft07.com/send";
    if let Some(end) = lower.find(host) {
        let candidate = input[..end]
            .rsplit_once("://")
            .map(|(_, value)| value)
            .unwrap_or(&input[..end])
            .trim();
        if is_valid_serverchan3_sendkey(candidate) {
            return Some(candidate.to_owned());
        }
    }

    None
}

fn is_valid_serverchan3_sendkey(sendkey: &str) -> bool {
    let lower = sendkey.to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("sctp") else {
        return false;
    };
    let Some(uid_end) = rest.find('t') else {
        return false;
    };
    uid_end > 0
        && rest[..uid_end].chars().all(|c| c.is_ascii_digit())
        && !rest[uid_end + 1..].is_empty()
        && rest[uid_end + 1..].chars().all(|c| c.is_ascii_alphanumeric())
}

fn serverchan3_send_url(sendkey: &str) -> Result<String> {
    let sendkey = extract_serverchan3_sendkey(sendkey).ok_or_else(|| anyhow::anyhow!("无效的 Server酱³ SendKey"))?;
    let lower = sendkey.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("sctp")
        .ok_or_else(|| anyhow::anyhow!("无效的 Server酱³ SendKey"))?;
    let uid_end = rest
        .find('t')
        .ok_or_else(|| anyhow::anyhow!("无效的 Server酱³ SendKey"))?;
    if uid_end == 0 || !rest[..uid_end].chars().all(|c| c.is_ascii_digit()) {
        bail!("无效的 Server酱³ SendKey");
    }
    let uid = &rest[..uid_end];
    Ok(format!("https://{uid}.push.ft07.com/send/{sendkey}.send"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serverchan3_send_url_is_derived_from_sendkey() {
        let sendkey = "sctp123456tABCdef";
        assert_eq!(
            serverchan3_send_url(sendkey).unwrap(),
            "https://123456.push.ft07.com/send/sctp123456tABCdef.send"
        );
    }

    #[test]
    fn serverchan3_send_url_rejects_invalid_sendkey() {
        assert!(serverchan3_send_url("invalid").is_err());
        assert!(serverchan3_send_url("sctptfoobar").is_err());
    }

    #[test]
    fn extract_serverchan3_sendkey_accepts_old_full_url() {
        assert_eq!(
            extract_serverchan3_sendkey("https://123456.push.ft07.com/send/sctp123456tabcDEF.send"),
            Some("sctp123456tabcDEF".to_owned())
        );
        assert_eq!(
            extract_serverchan3_sendkey("https://sctp123456tabcDEF.push.ft07.com/send"),
            Some("sctp123456tabcDEF".to_owned())
        );
    }
}
