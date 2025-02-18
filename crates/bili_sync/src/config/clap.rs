use clap::Parser;

#[derive(Parser)]
#[command(name = "Bili-Sync", version = version(), about, long_about = None)]
pub struct Args {
    #[arg(short, long, env = "SCAN_ONLY")]
    pub scan_only: bool,

    #[arg(short, long, default_value = "None,bili_sync=info", env = "RUST_LOG")]
    pub log_level: String,
}

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn version() -> String {
    let version = if let (Some(git_version), Some(git_dirty)) = (built_info::GIT_VERSION, built_info::GIT_DIRTY) {
        format!("{}{}", git_version, if git_dirty { "-dirty" } else { "" })
    } else {
        built_info::PKG_VERSION.to_owned()
    };
    format!(
        "
{}
Architecture: {}-{}
Author: {}
Built Time: {}
Rustc Version: {}
     ",
        version,
        built_info::CFG_OS,
        built_info::CFG_TARGET_ARCH,
        built_info::PKG_AUTHORS,
        built_info::BUILT_TIME_UTC,
        built_info::RUSTC_VERSION,
    )
    .trim()
    .to_owned()
}
