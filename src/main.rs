use bili_sync::bilibili::BiliClient;
use bili_sync::core::command::process_favorite_list;
use bili_sync::database::{database_connection, migrate_database};
use log::error;

#[tokio::main]
async fn main() -> ! {
    env_logger::init();
    let mut anchor = chrono::Local::now().date_naive();
    let (credential, interval, favorites) = {
        let config = bili_sync::config::CONFIG.lock().unwrap();
        (config.credential.clone(), config.interval, config.favorite_list.clone())
    };
    let mut bili_client = BiliClient::new(credential);
    let connection = database_connection().await.unwrap();
    migrate_database(&connection).await.unwrap();
    loop {
        if anchor != chrono::Local::now().date_naive() {
            if let Err(e) = bili_client.check_refresh().await {
                error!("Error: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                continue;
            }
            anchor = chrono::Local::now().date_naive();
        }
        for (fid, path) in &favorites {
            let res = process_favorite_list(&bili_client, fid, path, &connection).await;
            if let Err(e) = res {
                error!("Error: {e}");
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
    }
}
