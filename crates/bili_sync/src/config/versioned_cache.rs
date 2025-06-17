use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::Result;
use arc_swap::{ArcSwap, Guard};

use crate::config::{Config, VersionedConfig};

pub struct VersionedCache<T> {
    inner: ArcSwap<T>,
    version: AtomicU64,
    builder: fn(&Config) -> Result<T>,
    mutex: parking_lot::Mutex<()>,
}

impl<T> VersionedCache<T> {
    pub fn new(builder: fn(&Config) -> Result<T>) -> Result<Self> {
        let current_config = VersionedConfig::get().load();
        let current_version = current_config.version;
        let initial_value = builder(&current_config)?;
        Ok(Self {
            inner: ArcSwap::from_pointee(initial_value),
            version: AtomicU64::new(current_version),
            builder,
            mutex: parking_lot::Mutex::new(()),
        })
    }

    pub fn load(&self) -> Guard<Arc<T>> {
        self.reload_if_needed();
        self.inner.load()
    }

    fn reload_if_needed(&self) {
        let current_config = VersionedConfig::get().load();
        let current_version = current_config.version;
        let version = self.version.load(Ordering::Relaxed);
        if version < current_version {
            let _lock = self.mutex.lock();
            if self.version.load(Ordering::Relaxed) >= current_version {
                return;
            }
            match (self.builder)(&current_config) {
                Err(e) => {
                    error!("Failed to rebuild versioned cache: {:?}", e);
                }
                Ok(new_value) => {
                    self.inner.store(Arc::new(new_value));
                    self.version.store(current_version, Ordering::Relaxed);
                }
            }
        }
    }
}
