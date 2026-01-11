//! File content caching system

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use lru::LruCache;
use std::num::NonZeroUsize;

/// File content cache
pub struct FileCache {
    cache: Arc<Mutex<LruCache<PathBuf, String>>>,
}

impl FileCache {
    /// Create a new file cache with specified capacity
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(100).unwrap());
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(cap))),
        }
    }

    /// Get cached content or None if not cached
    pub fn get(&self, path: &PathBuf) -> Option<String> {
        let mut cache = self.cache.lock().unwrap();
        cache.get(path).cloned()
    }

    /// Put content into cache
    pub fn put(&self, path: PathBuf, content: String) {
        let mut cache = self.cache.lock().unwrap();
        cache.put(path, content);
    }

    /// Clear the cache
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }

    /// Check if a file is cached
    pub fn contains(&self, path: &PathBuf) -> bool {
        let cache = self.cache.lock().unwrap();
        cache.contains(path)
    }
}

impl Default for FileCache {
    fn default() -> Self {
        Self::new(100)
    }
}

impl Clone for FileCache {
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let cache = FileCache::new(2);
        let path1 = PathBuf::from("file1.txt");
        let path2 = PathBuf::from("file2.txt");

        cache.put(path1.clone(), "content1".to_string());
        cache.put(path2.clone(), "content2".to_string());

        assert_eq!(cache.get(&path1), Some("content1".to_string()));
        assert_eq!(cache.get(&path2), Some("content2".to_string()));
    }

    #[test]
    fn test_cache_lru() {
        let cache = FileCache::new(2);
        let path1 = PathBuf::from("file1.txt");
        let path2 = PathBuf::from("file2.txt");
        let path3 = PathBuf::from("file3.txt");

        cache.put(path1.clone(), "content1".to_string());
        cache.put(path2.clone(), "content2".to_string());
        cache.put(path3.clone(), "content3".to_string()); // This should evict path1

        assert_eq!(cache.get(&path1), None); // Evicted
        assert_eq!(cache.get(&path2), Some("content2".to_string()));
        assert_eq!(cache.get(&path3), Some("content3".to_string()));
    }
}
