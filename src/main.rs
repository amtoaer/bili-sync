use std::sync::Arc;

use bili_sync::bilibili::BiliClient;
use bili_sync::core::command::process_favorite;
use bili_sync::database::database_connection;

#[tokio::main]
async fn main() -> ! {
    let connection = Arc::new(database_connection().await.unwrap());
    let bili_client = Arc::new(BiliClient::new(None));
    loop {
        for fid in [52642258] {
            let _ = process_favorite(bili_client.clone(), fid, connection.clone()).await;
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
