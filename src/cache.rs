use anyhow::{Context, Result};
use bincode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};

/// A single cached file entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub mtime: u64,                      // File modification time
    pub size: u64,                       // File size in bytes
    pub content: String,                 // Cached content (compressed or full)
    pub mode: Option<String>,            // "full" or "compressed"
    pub config_hash: String,             // Config hash at time of caching
}

/// Thread-safe cache for flattened file content
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Cache {
    entries: HashMap<String, CacheEntry>,
}

impl Cache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Get a cached entry (immutable read - thread-safe)
    pub fn get(
        &self,
        path: &str,
        mtime: u64,
        size: u64,
        config_hash: &str,
    ) -> Option<CacheEntry> {
        if let Some(entry) = self.entries.get(path) {
            // Validate cache entry: mtime, size, and config must match
            if entry.mtime == mtime && entry.size == size && entry.config_hash == config_hash {
                return Some(entry.clone());
            }
        }
        None
    }

    /// Insert a cache entry (mutable write)
    pub fn insert(&mut self, path: String, entry: CacheEntry) {
        self.entries.insert(path, entry);
    }

    /// Prune stale entries (files that no longer exist)
    pub fn prune_stale(&mut self, existing_paths: &[&str]) {
        let existing_set: std::collections::HashSet<_> = existing_paths.iter().copied().collect();
        self.entries.retain(|path, _| existing_set.contains(path.as_str()));
    }

    /// Load cache from disk
    pub fn load(cache_path: &Path) -> Result<Self> {
        if !cache_path.exists() {
            return Ok(Cache::new());
        }

        let data = fs::read(cache_path)
            .context("Failed to read cache file")?;
        let cache = bincode::deserialize(&data)
            .context("Failed to deserialize cache")?;
        Ok(cache)
    }

    /// Save cache to disk
    pub fn save(&self, cache_path: &Path) -> Result<()> {
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create cache directory")?;
        }

        let data = bincode::serialize(self)
            .context("Failed to serialize cache")?;
        fs::write(cache_path, data)
            .context("Failed to write cache file")?;
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, u64) {
        let count = self.entries.len();
        let size_bytes = bincode::serialize(self)
            .map(|d| d.len() as u64)
            .unwrap_or(0);
        (count, size_bytes)
    }
}

/// Generate a config hash based on compression and filter settings
pub fn generate_config_hash(
    compress: bool,
    max_file_size: u64,
    include_extensions: &Option<Vec<String>>,
    exclude_extensions: &Option<Vec<String>>,
) -> String {
    let mut hasher = Sha256::new();

    // Hash compression flag
    hasher.update(format!("compress:{}", compress).as_bytes());

    // Hash max file size
    hasher.update(format!("max_size:{}", max_file_size).as_bytes());

    // Hash include extensions
    if let Some(ref exts) = include_extensions {
        hasher.update(b"include:");
        for ext in exts {
            hasher.update(ext.as_bytes());
            hasher.update(b",");
        }
    }

    // Hash exclude extensions
    if let Some(ref exts) = exclude_extensions {
        hasher.update(b"exclude:");
        for ext in exts {
            hasher.update(ext.as_bytes());
            hasher.update(b",");
        }
    }

    format!("{:x}", hasher.finalize())
}

/// Get the cache directory for a repository
pub fn get_cache_dir(repo_path: &Path) -> Result<PathBuf> {
    let repo_hash = get_repo_hash(repo_path)?;
    let cache_dir = dirs::cache_dir()
        .context("Failed to get cache directory")?
        .join("flat")
        .join(&repo_hash);
    Ok(cache_dir)
}

/// Get a hash of the repository path (for cache namespacing)
pub fn get_repo_hash(repo_path: &Path) -> Result<String> {
    let canonical = repo_path.canonicalize()
        .context("Failed to canonicalize repository path")?;
    let path_str = canonical.to_string_lossy();

    let mut hasher = Sha256::new();
    hasher.update(path_str.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

/// Get file metadata for cache validation
pub fn get_file_metadata(path: &Path) -> Result<(u64, u64)> {
    let metadata = fs::metadata(path)
        .context("Failed to get file metadata")?;

    let mtime = metadata.modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let size = metadata.len();
    Ok((mtime, size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_new() {
        let cache = Cache::new();
        assert_eq!(cache.entries.len(), 0);
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = Cache::new();
        let entry = CacheEntry {
            mtime: 100,
            size: 1000,
            content: "test content".to_string(),
            mode: Some("compressed".to_string()),
            config_hash: "abc123".to_string(),
        };

        cache.insert("test.rs".to_string(), entry.clone());
        let retrieved = cache.get("test.rs", 100, 1000, "abc123");
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_cache_invalidation_on_mtime_change() {
        let mut cache = Cache::new();
        let entry = CacheEntry {
            mtime: 100,
            size: 1000,
            content: "test content".to_string(),
            mode: Some("compressed".to_string()),
            config_hash: "abc123".to_string(),
        };

        cache.insert("test.rs".to_string(), entry);
        let retrieved = cache.get("test.rs", 101, 1000, "abc123"); // Different mtime
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_cache_invalidation_on_size_change() {
        let mut cache = Cache::new();
        let entry = CacheEntry {
            mtime: 100,
            size: 1000,
            content: "test content".to_string(),
            mode: Some("compressed".to_string()),
            config_hash: "abc123".to_string(),
        };

        cache.insert("test.rs".to_string(), entry);
        let retrieved = cache.get("test.rs", 100, 2000, "abc123"); // Different size
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_cache_invalidation_on_config_change() {
        let mut cache = Cache::new();
        let entry = CacheEntry {
            mtime: 100,
            size: 1000,
            content: "test content".to_string(),
            mode: Some("compressed".to_string()),
            config_hash: "abc123".to_string(),
        };

        cache.insert("test.rs".to_string(), entry);
        let retrieved = cache.get("test.rs", 100, 1000, "different_hash"); // Different config
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_config_hash_generation() {
        let hash1 = generate_config_hash(true, 1024, &None, &None);
        let hash2 = generate_config_hash(true, 1024, &None, &None);
        assert_eq!(hash1, hash2);

        let hash3 = generate_config_hash(false, 1024, &None, &None);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_prune_stale_entries() {
        let mut cache = Cache::new();
        cache.insert("exists.rs".to_string(), CacheEntry {
            mtime: 100,
            size: 1000,
            content: "content1".to_string(),
            mode: None,
            config_hash: "hash1".to_string(),
        });
        cache.insert("deleted.rs".to_string(), CacheEntry {
            mtime: 100,
            size: 1000,
            content: "content2".to_string(),
            mode: None,
            config_hash: "hash1".to_string(),
        });

        cache.prune_stale(&["exists.rs"]);
        assert_eq!(cache.entries.len(), 1);
        assert!(cache.entries.contains_key("exists.rs"));
        assert!(!cache.entries.contains_key("deleted.rs"));
    }
}
