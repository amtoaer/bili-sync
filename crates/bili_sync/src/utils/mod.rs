pub mod convert;
pub mod model;
pub mod nfo;
pub mod status;

use chrono::{DateTime, Utc};
use rand::Rng;
use tokio::time;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::Delay;

pub fn init_logger(log_level: &str) {
    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::builder().parse_lossy(log_level))
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
            "%Y-%m-%d %H:%M:%S%.3f".to_owned(),
        ))
        .finish()
        .try_init()
        .expect("初始化日志失败");
}

/// 生成视频的唯一标记，均由 bvid 和时间戳构成
pub fn id_time_key(bvid: &String, time: &DateTime<Utc>) -> String {
    format!("{}-{}", bvid, time.timestamp())
}

pub(crate) async fn delay(delay: Option<&Delay>) {
    match delay {
        None => {}
        Some(Delay::Random { min, max }) => {
            let delay = {
                let mut rng = rand::thread_rng();
                rng.gen_range(*min..=*max)
            };
            time::sleep(time::Duration::from_millis(delay)).await;
        }
        Some(Delay::Fixed(delay)) => {
            time::sleep(time::Duration::from_millis(*delay)).await;
        }
    }
}
