use codio_content_id::ContentId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory content storage
/// Phase 1: Simple storage, Phase 2+ will add distributed storage
#[derive(Clone)]
pub struct ContentStorage {
    store: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl ContentStorage {
    /// Create new content storage
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store content and return CID
    pub async fn store(&self, content: Vec<u8>) -> anyhow::Result<ContentId> {
        let cid = ContentId::new(&content);
        let cid_str = cid.as_str().to_string();

        self.store.write().await.insert(cid_str, content);

        Ok(cid)
    }

    /// Retrieve content by CID
    pub async fn retrieve(&self, cid: &ContentId) -> Option<Vec<u8>> {
        let cid_str = cid.as_str();
        self.store.read().await.get(cid_str).cloned()
    }

    /// Check if content exists
    pub async fn contains(&self, cid: &ContentId) -> bool {
        let cid_str = cid.as_str();
        self.store.read().await.contains_key(cid_str)
    }

    /// Get total stored items
    pub async fn len(&self) -> usize {
        self.store.read().await.len()
    }

    /// Check if storage is empty
    pub async fn is_empty(&self) -> bool {
        self.store.read().await.is_empty()
    }
}

impl Default for ContentStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let storage = ContentStorage::new();
        let content = b"Hello, CDN!".to_vec();

        let cid = storage.store(content.clone()).await.unwrap();
        let retrieved = storage.retrieve(&cid).await.unwrap();

        assert_eq!(content, retrieved);
    }

    #[tokio::test]
    async fn test_contains() {
        let storage = ContentStorage::new();
        let content = b"Test content".to_vec();

        let cid = storage.store(content).await.unwrap();

        assert!(storage.contains(&cid).await);
    }

    #[tokio::test]
    async fn test_retrieve_missing() {
        let storage = ContentStorage::new();
        let cid = ContentId::new(b"Non-existent");

        assert!(storage.retrieve(&cid).await.is_none());
    }
}
