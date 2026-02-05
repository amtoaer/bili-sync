use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::LazyLock;

use clap::Parser;

pub static ARGS: LazyLock<Args> = LazyLock::new(Args::parse);

#[derive(Parser)]
#[command(name = "Bili-Sync", version = detail_version(), about, long_about = None)]
pub struct Args {
    #[arg(short, long, env = "SCAN_ONLY")]
    pub scan_only: bool,

    #[arg(short, long, default_value = "None,bili_sync=info", env = "RUST_LOG")]
    pub log_level: String,

    #[arg(short, long, env = "DISABLE_CREDENTIAL_REFRESH")]
    pub disable_credential_refresh: bool,

    #[arg(short, long, env = "BILI_SYNC_CONFIG_DIR")]
    pub config_dir: Option<PathBuf>,

    #[arg(short, long, env = "BILI_SYNC_FFMPEG_PATH")]
    pub ffmpeg_path: Option<String>,
}

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn version() -> Cow<'static, str> {
    if let (Some(git_version), Some(git_dirty)) = (built_info::GIT_VERSION, built_info::GIT_DIRTY) {
        Cow::Owned(format!("{}{}", git_version, if git_dirty { "-dirty" } else { "" }))
    } else {
        Cow::Borrowed(built_info::PKG_VERSION)
    }
}

fn detail_version() -> String {
    format!(
        "{}
Architecture: {}-{}
Author: {}
Built Time: {}
Rustc Version: {}",
        version(),
        built_info::CFG_OS,
        built_info::CFG_TARGET_ARCH,
        built_info::PKG_AUTHORS,
        built_info::BUILT_TIME_UTC,
        built_info::RUSTC_VERSION,
    )
}
