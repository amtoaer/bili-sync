use bili_sync::bilibili::BiliClient;
use bili_sync::core::command::process_favorite;
use bili_sync::database::database_connection;
use log::error;

#[tokio::main]
async fn main() -> ! {
    env_logger::init();
    let mut today = chrono::Local::now().date_naive();
    let mut bili_client = BiliClient::new(None);
    let connection = database_connection().await.unwrap();
    loop {
        if today != chrono::Local::now().date_naive() {
            if let Err(e) = bili_client.check_refresh().await {
                error!("Error: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(600)).await;
                continue;
            }
            today = chrono::Local::now().date_naive();
        }
        for fid in ["52642258"] {
            let res = process_favorite(&bili_client, fid, &connection).await;
            if let Err(e) = res {
                error!("Error: {e}");
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(600)).await;
    }
}
