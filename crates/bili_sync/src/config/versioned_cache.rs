use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use arc_swap::{ArcSwap, Guard};

use crate::config::Config;

pub struct VersionedCache<T> {
    inner: ArcSwap<T>,
    version: AtomicU64,
    builder: fn(&Config) -> Result<T>,
    mutex: parking_lot::Mutex<()>,
}

impl<T> VersionedCache<T> {
    pub fn new(builder: fn(&Config) -> Result<T>, initial_config: &Config) -> Result<Self> {
        let initial_version = initial_config.version;
        let initial_value = builder(initial_config)?;
        Ok(Self {
            inner: ArcSwap::from_pointee(initial_value),
            version: AtomicU64::new(initial_version),
            builder,
            mutex: parking_lot::Mutex::new(()),
        })
    }

    /// 获取当前的值，不检查版本
    pub fn load(&self) -> Guard<Arc<T>> {
        self.inner.load()
    }

    /// 获取当前的值，确保版本不低于指定配置的版本
    pub fn load_full_with_update(&self, new_config: &Config) -> Result<Arc<T>> {
        if self.version.load(Ordering::Relaxed) >= new_config.version {
            return Ok(self.inner.load_full());
        }
        let _lock = self.mutex.lock();
        if self.version.load(Ordering::Relaxed) >= new_config.version {
            return Ok(self.inner.load_full());
        }
        let new_value = (self.builder)(new_config).context("Failed to reload versioned cache")?;
        let new_value = Arc::new(new_value);
        self.inner.store(new_value.clone());
        self.version.store(new_config.version, Ordering::Relaxed);
        Ok(new_value)
    }
}
