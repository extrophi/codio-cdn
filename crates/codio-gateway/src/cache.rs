use codio_content_id::ContentId;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;

/// LRU cache for hot content
#[derive(Clone)]
pub struct ContentCache {
    cache: Arc<RwLock<LruCache<String, Vec<u8>>>>,
    max_size: usize,
    current_size: Arc<RwLock<usize>>,
}

impl ContentCache {
    /// Create new cache with size limit in bytes
    pub fn new(max_size: usize) -> Self {
        // Calculate approximate capacity based on average item size
        // Assume average item size of 100KB
        let avg_item_size = 100 * 1024;
        let capacity = (max_size / avg_item_size).max(10);
        let capacity = NonZeroUsize::new(capacity).unwrap();

        Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
            max_size,
            current_size: Arc::new(RwLock::new(0)),
        }
    }

    /// Put content in cache
    pub async fn put(&self, cid: &ContentId, content: Vec<u8>) {
        let cid_str = cid.as_str().to_string();
        let content_size = content.len();

        // Evict items if needed
        while *self.current_size.read().await + content_size > self.max_size {
            let mut cache = self.cache.write().await;
            if let Some((_, evicted)) = cache.pop_lru() {
                let mut size = self.current_size.write().await;
                *size = size.saturating_sub(evicted.len());
            } else {
                break; // Cache is empty
            }
        }

        // Insert new item
        self.cache.write().await.put(cid_str, content.clone());
        let mut size = self.current_size.write().await;
        *size += content_size;
    }

    /// Get content from cache
    pub async fn get(&self, cid: &ContentId) -> Option<Vec<u8>> {
        let cid_str = cid.as_str();
        self.cache.write().await.get(cid_str).cloned()
    }

    /// Check if content is in cache
    pub async fn contains(&self, cid: &ContentId) -> bool {
        let cid_str = cid.as_str();
        self.cache.read().await.contains(cid_str)
    }

    /// Get cache size in bytes
    pub async fn size(&self) -> usize {
        *self.current_size.read().await
    }

    /// Get number of cached items
    pub async fn len(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Check if cache is empty
    pub async fn is_empty(&self) -> bool {
        self.cache.read().await.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_put_get() {
        let cache = ContentCache::new(1024 * 1024); // 1MB
        let content = b"Cached content".to_vec();
        let cid = ContentId::new(&content);

        cache.put(&cid, content.clone()).await;
        let retrieved = cache.get(&cid).await.unwrap();

        assert_eq!(content, retrieved);
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let cache = ContentCache::new(100); // Very small cache
        let content1 = vec![0u8; 60]; // 60 bytes
        let content2 = vec![1u8; 60]; // 60 bytes
        let cid1 = ContentId::new(&content1);
        let cid2 = ContentId::new(&content2);

        cache.put(&cid1, content1).await;
        cache.put(&cid2, content2).await; // Should evict content1

        assert!(cache.get(&cid2).await.is_some());
    }

    #[tokio::test]
    async fn test_cache_contains() {
        let cache = ContentCache::new(1024 * 1024);
        let content = b"Test".to_vec();
        let cid = ContentId::new(&content);

        assert!(!cache.contains(&cid).await);
        cache.put(&cid, content).await;
        assert!(cache.contains(&cid).await);
    }
}
