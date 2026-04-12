pub mod convert;
pub mod download_context;
pub mod filenamify;
pub mod format_arg;
pub mod model;
pub mod nfo;
pub mod notify;
pub mod rule;
pub mod signal;
pub mod status;
pub mod validation;
use std::path::Path;

use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::api::LogHelper;

pub fn init_logger(log_level: &str, log_writer: Option<LogHelper>) {
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

pub fn compact_log_text(text: &str, max_chars: usize) -> String {
    let text = text.trim();
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_owned();
    }
    if max_chars <= 1 {
        return "…".to_owned();
    }
    let head_len = (max_chars - 1) * 2 / 3;
    let tail_len = (max_chars - 1) - head_len;
    let head = text.chars().take(head_len).collect::<String>();
    let tail = text
        .chars()
        .skip(char_count.saturating_sub(tail_len))
        .collect::<String>();
    format!("{head}…{tail}")
}

pub fn compact_log_filename(filename: &str, max_chars: usize) -> String {
    let filename = filename.trim();
    if filename.chars().count() <= max_chars {
        return filename.to_owned();
    }
    if let Some((stem, ext)) = filename.rsplit_once('.')
        && !stem.is_empty()
        && !ext.is_empty()
    {
        let suffix = format!(".{ext}");
        let suffix_len = suffix.chars().count();
        if suffix_len < max_chars {
            return format!("{}{}", compact_log_text(stem, max_chars - suffix_len), suffix);
        }
    }
    compact_log_text(filename, max_chars)
}

pub fn compact_log_path(path: &str, max_chars: usize) -> String {
    let trimmed = path.trim().trim_matches('"').trim_matches('\'');
    let display = Path::new(trimmed)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(trimmed);
    compact_log_filename(display, max_chars)
}
