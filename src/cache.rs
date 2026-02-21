use std::path::{Path, PathBuf};

use log::debug;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::Result;

/// File-based cache for API responses, enabling resume on interruption.
///
/// Keys use `/` as directory separators, e.g. `484174/collections/0`
/// maps to `.bgm_cache/484174/collections/0.json`.
///
/// Empty results are recorded as zero-byte files to avoid re-fetching.
pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    pub fn new(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        Ok(Self {
            dir: dir.to_path_buf(),
        })
    }

    /// Build a file path from a cache key.
    /// `/` in the key becomes a directory separator.
    fn path(&self, key: &str) -> PathBuf {
        let mut p = self.dir.clone();
        for part in key.split('/') {
            p.push(part);
        }
        p.set_extension("json");
        p
    }

    /// Check if a key exists in the cache (file exists).
    pub fn has(&self, key: &str) -> bool {
        self.path(key).exists()
    }

    /// Try to load a cached value. Returns `None` on miss, empty file, or deserialization failure.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let path = self.path(key);
        let data = std::fs::read_to_string(&path).ok()?;
        if data.is_empty() {
            debug!("Cache hit (empty marker): {}", key);
            return None;
        }
        match serde_json::from_str(&data) {
            Ok(val) => {
                debug!("Cache hit: {}", key);
                Some(val)
            }
            Err(e) => {
                debug!("Cache parse error for {}: {}", key, e);
                None
            }
        }
    }

    /// Store a value in the cache.
    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let path = self.path(key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string(value)?;
        std::fs::write(&path, data)?;
        debug!("Cache write: {}", key);
        Ok(())
    }

    /// Write an empty marker file to record that the key was fetched but had no data.
    pub fn set_empty(&self, key: &str) -> Result<()> {
        let path = self.path(key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, "")?;
        debug!("Cache write (empty): {}", key);
        Ok(())
    }

    /// Remove the entire cache directory.
    pub fn clear(&self) -> Result<()> {
        if self.dir.exists() {
            std::fs::remove_dir_all(&self.dir)?;
        }
        Ok(())
    }
}
