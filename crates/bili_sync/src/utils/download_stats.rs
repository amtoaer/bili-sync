use std::collections::VecDeque;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;
use serde::Serialize;

const SPEED_WINDOW_MILLIS: i64 = 10_000;

static DOWNLOAD_STATS: LazyLock<DownloadStatsManager> = LazyLock::new(DownloadStatsManager::new);

#[derive(Debug, Clone, Copy, Serialize)]
pub struct DownloadStats {
    pub timestamp: i64,
    pub current_speed_bytes_per_sec: u64,
    pub task_downloaded_bytes: u64,
    pub active_videos: u64,
    pub active_pages: u64,
    pub active_fragments: u64,
}

#[derive(Debug, Default)]
pub struct DownloadStatsManager {
    task_downloaded_bytes: AtomicU64,
    active_videos: AtomicU64,
    active_pages: AtomicU64,
    active_fragments: AtomicU64,
    recent_bytes: Mutex<VecDeque<ByteSample>>,
}

#[derive(Debug, Clone, Copy)]
struct ByteSample {
    timestamp: i64,
    bytes: u64,
}

impl DownloadStatsManager {
    pub fn get() -> &'static Self {
        &DOWNLOAD_STATS
    }

    fn new() -> Self {
        Self::default()
    }

    pub fn reset_task(&self) {
        self.task_downloaded_bytes.store(0, Ordering::Relaxed);
        self.recent_bytes.lock().clear();
    }

    pub fn record_bytes(&self, bytes: u64) {
        self.record_bytes_at(chrono::Utc::now().timestamp_millis(), bytes);
    }

    pub fn track_video(&self) -> DownloadStatsGuard<'_> {
        self.active_videos.fetch_add(1, Ordering::Relaxed);
        DownloadStatsGuard {
            counter: &self.active_videos,
        }
    }

    pub fn track_page(&self) -> DownloadStatsGuard<'_> {
        self.active_pages.fetch_add(1, Ordering::Relaxed);
        DownloadStatsGuard {
            counter: &self.active_pages,
        }
    }

    pub fn track_fragment(&self) -> DownloadStatsGuard<'_> {
        self.active_fragments.fetch_add(1, Ordering::Relaxed);
        DownloadStatsGuard {
            counter: &self.active_fragments,
        }
    }

    pub fn snapshot(&self) -> DownloadStats {
        self.snapshot_at(chrono::Utc::now().timestamp_millis())
    }

    fn record_bytes_at(&self, timestamp: i64, bytes: u64) {
        if bytes == 0 {
            return;
        }
        self.task_downloaded_bytes.fetch_add(bytes, Ordering::Relaxed);
        let mut recent_bytes = self.recent_bytes.lock();
        recent_bytes.push_back(ByteSample { timestamp, bytes });
        Self::prune_old_samples(&mut recent_bytes, timestamp);
    }

    fn snapshot_at(&self, timestamp: i64) -> DownloadStats {
        let current_speed_bytes_per_sec = {
            let mut recent_bytes = self.recent_bytes.lock();
            Self::prune_old_samples(&mut recent_bytes, timestamp);
            recent_bytes.iter().map(|sample| sample.bytes).sum::<u64>() / (SPEED_WINDOW_MILLIS as u64 / 1000)
        };
        DownloadStats {
            timestamp,
            current_speed_bytes_per_sec,
            task_downloaded_bytes: self.task_downloaded_bytes.load(Ordering::Relaxed),
            active_videos: self.active_videos.load(Ordering::Relaxed),
            active_pages: self.active_pages.load(Ordering::Relaxed),
            active_fragments: self.active_fragments.load(Ordering::Relaxed),
        }
    }

    fn prune_old_samples(recent_bytes: &mut VecDeque<ByteSample>, now: i64) {
        while recent_bytes
            .front()
            .is_some_and(|sample| sample.timestamp < now - SPEED_WINDOW_MILLIS)
        {
            recent_bytes.pop_front();
        }
    }

    #[cfg(test)]
    fn new_for_test() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct DownloadStatsGuard<'a> {
    counter: &'a AtomicU64,
}

impl Drop for DownloadStatsGuard<'_> {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_reset_keeps_current_gauges_and_clears_task_bytes() {
        let stats = DownloadStatsManager::new_for_test();
        let _video = stats.track_video();
        let _page = stats.track_page();
        let _fragment = stats.track_fragment();

        stats.record_bytes_at(10_000, 1024);
        stats.reset_task();

        let snapshot = stats.snapshot_at(10_000);
        assert_eq!(snapshot.task_downloaded_bytes, 0);
        assert_eq!(snapshot.active_videos, 1);
        assert_eq!(snapshot.active_pages, 1);
        assert_eq!(snapshot.active_fragments, 1);
    }

    #[test]
    fn speed_uses_recent_byte_samples() {
        let stats = DownloadStatsManager::new_for_test();

        stats.record_bytes_at(1_000, 1024);
        stats.record_bytes_at(2_000, 1024);
        stats.record_bytes_at(12_500, 4096);

        let snapshot = stats.snapshot_at(12_500);
        assert_eq!(snapshot.current_speed_bytes_per_sec, 409);
        assert_eq!(snapshot.task_downloaded_bytes, 6144);
    }

    #[test]
    fn active_guards_decrement_on_drop() {
        let stats = DownloadStatsManager::new_for_test();
        {
            let _video = stats.track_video();
            let _page = stats.track_page();
            let _fragment = stats.track_fragment();

            let snapshot = stats.snapshot_at(1_000);
            assert_eq!(snapshot.active_videos, 1);
            assert_eq!(snapshot.active_pages, 1);
            assert_eq!(snapshot.active_fragments, 1);
        }

        let snapshot = stats.snapshot_at(1_000);
        assert_eq!(snapshot.active_videos, 0);
        assert_eq!(snapshot.active_pages, 0);
        assert_eq!(snapshot.active_fragments, 0);
    }
}
