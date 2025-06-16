use std::sync::atomic::AtomicBool;

pub static DOWNLOADER_RUNNING: AtomicBool = AtomicBool::new(false);
