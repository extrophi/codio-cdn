use codio_content_id::ContentId;
use codio_gateway::{ContentCache, ContentStorage, Metrics};

#[tokio::test]
async fn test_storage_roundtrip() {
    let storage = ContentStorage::new();
    let content = b"Hello, Codio CDN!".to_vec();

    // Store content
    let cid = storage.store(content.clone()).await.unwrap();

    // Retrieve content
    let retrieved = storage.retrieve(&cid).await.unwrap();

    assert_eq!(content, retrieved);
}

#[tokio::test]
async fn test_cache_performance() {
    let cache = ContentCache::new(1024 * 1024); // 1MB cache
    let content = b"Cached content for performance test".to_vec();
    let cid = ContentId::new(&content);

    // First access - cache miss
    assert!(cache.get(&cid).await.is_none());

    // Put in cache
    cache.put(&cid, content.clone()).await;

    // Second access - cache hit
    let cached = cache.get(&cid).await;
    assert!(cached.is_some());
    assert_eq!(cached.unwrap(), content);
}

#[tokio::test]
async fn test_metrics_tracking() {
    let metrics = Metrics::new();

    // Record various events
    metrics.record_upload().await;
    metrics.record_download(1024).await;
    metrics.record_cache_hit().await;
    metrics.record_cache_miss().await;

    // Verify metrics
    let response = metrics.get().await;
    assert_eq!(response.uploads, 1);
    assert_eq!(response.downloads, 1);
    assert_eq!(response.cache_hits, 1);
    assert_eq!(response.cache_misses, 1);
    assert_eq!(response.total_bytes_served, 1024);
}

#[tokio::test]
async fn test_cid_verification() {
    let storage = ContentStorage::new();
    let content = b"Content for verification".to_vec();

    // Store content
    let cid = storage.store(content.clone()).await.unwrap();

    // Verify CID is correct
    assert!(cid.verify(&content));
    assert!(!cid.verify(b"Different content"));
}

#[tokio::test]
async fn test_cache_eviction() {
    let cache = ContentCache::new(200); // Very small cache

    let content1 = vec![0u8; 100];
    let content2 = vec![1u8; 100];
    let content3 = vec![2u8; 100];

    let cid1 = ContentId::new(&content1);
    let cid2 = ContentId::new(&content2);
    let cid3 = ContentId::new(&content3);

    // Fill cache
    cache.put(&cid1, content1).await;
    cache.put(&cid2, content2).await;

    // This should trigger eviction
    cache.put(&cid3, content3.clone()).await;

    // Most recent should be available
    assert!(cache.get(&cid3).await.is_some());
}

#[tokio::test]
async fn test_multiple_uploads() {
    let storage = ContentStorage::new();

    let contents = vec![
        b"Content 1".to_vec(),
        b"Content 2".to_vec(),
        b"Content 3".to_vec(),
    ];

    let mut cids = Vec::new();

    // Upload all contents
    for content in &contents {
        let cid = storage.store(content.clone()).await.unwrap();
        cids.push(cid);
    }

    // Verify all can be retrieved
    for (i, cid) in cids.iter().enumerate() {
        let retrieved = storage.retrieve(cid).await.unwrap();
        assert_eq!(retrieved, contents[i]);
    }
}

#[tokio::test]
async fn test_same_content_same_cid() {
    let storage = ContentStorage::new();
    let content = b"Duplicate content".to_vec();

    // Store twice
    let cid1 = storage.store(content.clone()).await.unwrap();
    let cid2 = storage.store(content).await.unwrap();

    // Should produce same CID
    assert_eq!(cid1, cid2);
}
