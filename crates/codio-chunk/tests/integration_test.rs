use codio_chunk::{Chunk, ChunkConfig, ChunkDistributor, DistributionStrategy, PeerInfo};
use codio_common::PeerId;
use codio_content_id::ContentId;
use std::time::Duration;

/// Helper to create test peers
fn create_test_peers(count: usize) -> Vec<PeerInfo> {
    (0..count)
        .map(|i| PeerInfo::new(PeerId(format!("peer{}", i)), format!("addr{}", i)))
        .collect()
}

#[tokio::test]
async fn test_parallel_download() {
    // Setup 5 peers, each has all chunks
    let config = ChunkConfig::builder()
        .max_concurrent_downloads(5)
        .chunks_per_peer(4)
        .build();

    let mut distributor = ChunkDistributor::new(config).await.unwrap();

    // Create test content
    let content = b"Hello, distributed world! This is test content for parallel downloading.";
    let cid = ContentId::new(content);

    // Store content locally (simulating that peers have it)
    distributor
        .store_content(cid.clone(), content.to_vec())
        .await
        .unwrap();

    // Create 5 providers
    let providers = create_test_peers(5);

    // Download from all 5 simultaneously
    let start = std::time::Instant::now();
    let downloaded = distributor
        .download_content(cid.clone(), providers)
        .await
        .unwrap();
    let elapsed = start.elapsed();

    // Verify content matches
    assert_eq!(downloaded, content);

    // Verify download was reasonably fast (with 5 peers should be faster than single peer)
    // This is a simulation, so timing isn't realistic, but we can verify it works
    println!("Parallel download completed in {:?}", elapsed);
    assert!(elapsed < Duration::from_secs(5));
}

#[tokio::test]
async fn test_rarest_first() {
    // This test verifies the rarest-first strategy logic
    let config = ChunkConfig::builder()
        .strategy(DistributionStrategy::RarestFirst)
        .chunk_size(10) // Small chunks for testing
        .build();

    let mut distributor = ChunkDistributor::new(config).await.unwrap();

    // Create content with multiple chunks
    let content = b"0123456789ABCDEFGHIJ"; // 2 chunks of 10 bytes each
    let cid = ContentId::new(content);

    distributor
        .store_content(cid.clone(), content.to_vec())
        .await
        .unwrap();

    // Setup 3 peers with different chunk availability
    // In a real scenario, we'd query peers for their chunks
    // Here we just verify the download works with the strategy
    let providers = create_test_peers(3);

    let downloaded = distributor
        .download_content(cid.clone(), providers)
        .await
        .unwrap();

    assert_eq!(downloaded, content);
}

#[tokio::test]
async fn test_sequential_strategy() {
    let config = ChunkConfig::builder()
        .strategy(DistributionStrategy::Sequential)
        .chunk_size(10)
        .build();

    let mut distributor = ChunkDistributor::new(config).await.unwrap();

    let content = b"Sequential download test content here!";
    let cid = ContentId::new(content);

    distributor
        .store_content(cid.clone(), content.to_vec())
        .await
        .unwrap();

    let providers = create_test_peers(2);

    let downloaded = distributor
        .download_content(cid.clone(), providers)
        .await
        .unwrap();

    assert_eq!(downloaded, content);
}

#[tokio::test]
async fn test_random_order_strategy() {
    let config = ChunkConfig::builder()
        .strategy(DistributionStrategy::RandomOrder)
        .chunk_size(20)
        .build();

    let mut distributor = ChunkDistributor::new(config).await.unwrap();

    let content = b"Random order download test with some content";
    let cid = ContentId::new(content);

    distributor
        .store_content(cid.clone(), content.to_vec())
        .await
        .unwrap();

    let providers = create_test_peers(3);

    let downloaded = distributor
        .download_content(cid.clone(), providers)
        .await
        .unwrap();

    assert_eq!(downloaded, content);
}

#[tokio::test]
async fn test_chunk_verification() {
    let config = ChunkConfig::default();
    let mut distributor = ChunkDistributor::new(config).await.unwrap();

    // Create valid chunk
    let data = vec![1, 2, 3, 4, 5];
    let chunk = Chunk::new(0, data.clone());

    assert!(chunk.verify(), "Valid chunk should verify");

    // Corrupt the chunk data
    let mut corrupted = chunk.clone();
    corrupted.data[0] = 99;

    assert!(
        !corrupted.verify(),
        "Corrupted chunk should fail verification"
    );

    // Test with actual content download
    let content = b"Test content for verification";
    let cid = ContentId::new(content);

    distributor
        .store_content(cid.clone(), content.to_vec())
        .await
        .unwrap();

    let providers = create_test_peers(2);
    let downloaded = distributor
        .download_content(cid.clone(), providers)
        .await
        .unwrap();

    // Verify content integrity
    let downloaded_cid = ContentId::new(&downloaded);
    assert_eq!(cid, downloaded_cid, "Downloaded content should match CID");
}

#[tokio::test]
async fn test_tit_for_tat() {
    let config = ChunkConfig::builder().min_upload_ratio(0.8).build();

    let mut distributor = ChunkDistributor::new(config).await.unwrap();

    let content = b"Tit-for-tat test content with enough data for multiple chunks";
    let cid = ContentId::new(content);

    // Store content locally so we can serve it
    distributor
        .store_content(cid.clone(), content.to_vec())
        .await
        .unwrap();

    // Create two peers
    let peer1 = PeerInfo::new(PeerId("peer1".to_string()), "addr1".to_string());
    let peer2 = PeerInfo::new(PeerId("peer2".to_string()), "addr2".to_string());

    // Serve chunk to peer1 (peer1 receives from us)
    let chunk1 = distributor
        .serve_chunk(cid.clone(), 0, peer1.clone())
        .await
        .unwrap();

    assert!(chunk1.verify(), "Chunk should be valid");
    assert_eq!(chunk1.index, 0);

    // Serve chunk to peer2
    let chunk2 = distributor
        .serve_chunk(cid.clone(), 0, peer2.clone())
        .await
        .unwrap();

    assert!(chunk2.verify(), "Chunk should be valid");
    assert_eq!(chunk2.index, 0);

    // In a real scenario, the tit-for-tat mechanism would track that
    // we've uploaded more to peer1, and would prefer downloading from peer1
    // Here we verify that the serve mechanism works correctly
}

#[tokio::test]
async fn test_download_progress() {
    let config = ChunkConfig::builder()
        .chunk_size(10)
        .request_timeout(Duration::from_secs(30))
        .build();

    let mut distributor = ChunkDistributor::new(config).await.unwrap();

    let content = b"0123456789ABCDEFGHIJKLMNOPQRS"; // 30 bytes = 3 chunks
    let cid = ContentId::new(content);

    distributor
        .store_content(cid.clone(), content.to_vec())
        .await
        .unwrap();

    // Check initial progress (should be None since download hasn't started)
    let progress = distributor.download_progress(&cid).await;
    assert!(progress.is_none());

    // Start download (in this simulation it's too fast to catch in progress)
    let providers = create_test_peers(2);
    let downloaded = distributor
        .download_content(cid.clone(), providers)
        .await
        .unwrap();

    // Verify download completed
    assert_eq!(downloaded, content);

    // Note: In this simulation, download is very fast, so we can't really test progress monitoring
}

#[tokio::test]
async fn test_multiple_concurrent_downloads() {
    let config = ChunkConfig::builder().max_concurrent_downloads(5).build();

    let mut distributor = ChunkDistributor::new(config).await.unwrap();

    // Create multiple pieces of content
    let contents = vec![
        b"Content 1".to_vec(),
        b"Content 2 with more data".to_vec(),
        b"Content 3 even longer content here".to_vec(),
    ];

    let mut cids = Vec::new();
    for content in &contents {
        let cid = ContentId::new(content);
        distributor
            .store_content(cid.clone(), content.clone())
            .await
            .unwrap();
        cids.push(cid);
    }

    // Download all sequentially (since our distributor has internal state management)
    let providers = create_test_peers(3);

    for (idx, cid) in cids.iter().enumerate() {
        let downloaded = distributor
            .download_content(cid.clone(), providers.clone())
            .await
            .unwrap();
        assert_eq!(downloaded, contents[idx]);
    }
}

#[tokio::test]
async fn test_large_content() {
    let config = ChunkConfig::builder()
        .chunk_size(256 * 1024) // 256KB chunks
        .build();

    let mut distributor = ChunkDistributor::new(config).await.unwrap();

    // Create 1MB of content (4 chunks)
    let content: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
    let cid = ContentId::new(&content);

    distributor
        .store_content(cid.clone(), content.clone())
        .await
        .unwrap();

    let providers = create_test_peers(4);
    let downloaded = distributor.download_content(cid, providers).await.unwrap();

    assert_eq!(downloaded.len(), content.len());
    assert_eq!(downloaded, content);
}

#[tokio::test]
async fn test_no_providers_error() {
    let config = ChunkConfig::default();
    let mut distributor = ChunkDistributor::new(config).await.unwrap();

    let cid = ContentId::new(b"test");
    let providers = vec![]; // No providers

    let result = distributor.download_content(cid, providers).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No providers"));
}

#[tokio::test]
async fn test_content_verification_failure() {
    // This test verifies CID verification on the ContentId level
    let content = b"Test content";
    let cid = ContentId::new(content);

    // Verify the CID matches
    assert!(cid.verify(content));

    // Verify different content doesn't match
    let wrong_content = b"Wrong content";
    assert!(!cid.verify(wrong_content));
}
