pub mod convert;
pub mod filenamify;
pub mod format_arg;
pub mod model;
pub mod nfo;
pub mod status;

use tracing_subscriber::util::SubscriberInitExt;

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
