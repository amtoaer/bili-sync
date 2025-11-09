use crate::bilibili::BiliClient;
use crate::config::Config;
use crate::notifier::NotifierAllExt;

pub fn error_and_notify(config: &Config, bili_client: &BiliClient, msg: String) {
    error!("{msg}");
    if let Some(notifiers) = &config.notifiers
        && !notifiers.is_empty()
    {
        let (notifiers, inner_client) = (notifiers.clone(), bili_client.inner_client().clone());
        tokio::spawn(async move { notifiers.notify_all(&inner_client, msg.as_str()).await });
    }
}
