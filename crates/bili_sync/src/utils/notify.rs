use crate::bilibili::BiliClient;
use crate::config::Config;
use crate::notifier::{Message, NotifierAllExt};

pub fn notify(config: &Config, bili_client: &BiliClient, msg: impl Into<Message<'static>>) {
    if let Some(notifiers) = &config.notifiers
        && !notifiers.is_empty()
    {
        let (notifiers, inner_client) = (notifiers.clone(), bili_client.inner_client().clone());
        let msg = msg.into();
        tokio::spawn(async move { notifiers.notify_all(&inner_client, msg).await });
    }
}

pub fn error_and_notify(config: &Config, bili_client: &BiliClient, msg: String) {
    error!("{msg}");
    if let Some(notifiers) = &config.notifiers
        && !notifiers.is_empty()
    {
        let (notifiers, inner_client) = (notifiers.clone(), bili_client.inner_client().clone());
        tokio::spawn(async move { notifiers.notify_all(&inner_client, msg).await });
    }
}
