pub mod convert;
pub mod filenamify;
pub mod format_arg;
pub mod model;
pub mod nfo;
pub mod signal;
pub mod status;
pub mod validation;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::api::MpscWriter;

pub fn init_logger(log_level: &str, log_writer: Option<MpscWriter>) {
    let log = tracing_subscriber::fmt::Subscriber::builder()
        .compact()
        .with_env_filter(tracing_subscriber::EnvFilter::builder().parse_lossy(log_level))
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
            "%b %d %H:%M:%S".to_owned(),
        ))
        .finish();
    if let Some(writer) = log_writer {
        log.with(
            fmt::layer()
                .with_ansi(false)
                .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
                    "%b %d %H:%M:%S".to_owned(),
                ))
                .json()
                .flatten_event(true)
                .with_writer(writer),
        )
        .try_init()
        .expect("初始化日志失败");
    } else {
        log.try_init().expect("初始化日志失败");
    }
}
