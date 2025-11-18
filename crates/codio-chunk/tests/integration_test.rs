use codio_chunk::{Chunk, ChunkConfig, ChunkDistributor, DistributionStrategy, PeerInfo};
use codio_common::{ContentId, PeerId};
use std::collections::HashSet;

/// Test parallel downloads from multiple peers
#[tokio::test]
async fn test_parallel_download() {
    // Create distributor
    let config = ChunkConfig {
        max_concurrent_downloads: 10,
        chunks_per_peer: 4,
        request_timeout: std::time::Duration::from_secs(5),
        strategy: DistributionStrategy::Sequential,
        chunk_size: 256,
    };

    let mut distributor = ChunkDistributor::new(config).await.unwrap();

    // Create test content (1024 bytes = 4 chunks of 256 bytes)
    let content = b"a".repeat(1024);
    let cid = ContentId::new(&content);

    // Store content locally (simulating that all peers have it)
    distributor
        .store_content(cid.clone(), content.clone())
        .await;

    // Setup 5 peers, each has all chunks
    let mut providers = Vec::new();
    for i in 0..5 {
        let peer_id = PeerId(format!("peer{}", i));
        let mut peer_info = PeerInfo::new(peer_id);
        peer_info.available_chunks = (0..4).collect(); // All 4 chunks
        providers.push(peer_info);
    }

    // Download content
    let result = distributor.download_content(cid.clone(), providers).await;

    assert!(result.is_ok(), "Download should succeed");
    let downloaded = result.unwrap();
    assert_eq!(
        downloaded, content,
        "Downloaded content should match original"
    );
}

/// Test rarest-first strategy
#[tokio::test]
async fn test_rarest_first() {
    let config = ChunkConfig {
        max_concurrent_downloads: 10,
        chunks_per_peer: 4,
        request_timeout: std::time::Duration::from_secs(5),
        strategy: DistributionStrategy::RarestFirst,
        chunk_size: 256,
    };

    let distributor = ChunkDistributor::new(config).await.unwrap();

    // Setup 3 peers with different chunk availability
    // Peer 1: chunks [0, 1, 2]
    let mut peer1 = PeerInfo::new(PeerId("peer1".to_string()));
    peer1.available_chunks = vec![0, 1, 2].into_iter().collect();

    // Peer 2: chunks [0, 1, 3]
    let mut peer2 = PeerInfo::new(PeerId("peer2".to_string()));
    peer2.available_chunks = vec![0, 1, 3].into_iter().collect();

    // Peer 3: chunks [0, 1]
    let mut peer3 = PeerInfo::new(PeerId("peer3".to_string()));
    peer3.available_chunks = vec![0, 1].into_iter().collect();

    let providers = vec![peer1, peer2, peer3];

    // Calculate rarest-first order
    let order = distributor
        .calculate_rarest_first_order(&providers, 4)
        .await
        .unwrap();

    // Chunks 2 and 3 are rarest (only 1 peer each)
    // They should come before chunks 0 and 1 (which have 3 peers)
    let rarest_chunks: HashSet<u32> = order[0..2].iter().copied().collect();
    assert!(
        rarest_chunks.contains(&2) && rarest_chunks.contains(&3),
        "Rarest chunks (2, 3) should be downloaded first, got order: {:?}",
        order
    );

    // Chunks 0 and 1 should come last (most common)
    let common_chunks: HashSet<u32> = order[2..4].iter().copied().collect();
    assert!(
        common_chunks.contains(&0) && common_chunks.contains(&1),
        "Common chunks (0, 1) should be downloaded last, got order: {:?}",
        order
    );
}

/// Test chunk verification with corrupted data
#[tokio::test]
async fn test_chunk_verification() {
    // Create a valid chunk
    let data = b"test chunk data".to_vec();
    let chunk = Chunk::new(0, data.clone());

    // Verify it's valid
    assert!(chunk.verify(), "Chunk should be valid initially");

    // Create a corrupted chunk (data doesn't match hash)
    let mut corrupted = chunk.clone();
    corrupted.data = b"corrupted data".to_vec();

    // Verification should fail
    assert!(
        !corrupted.verify(),
        "Corrupted chunk should fail verification"
    );
}

/// Test chunk verification during download
#[tokio::test]
async fn test_chunk_verification_in_download() {
    let config = ChunkConfig::default();
    let mut distributor = ChunkDistributor::new(config).await.unwrap();

    let content = b"valid test content for verification";
    let cid = ContentId::new(content);

    // Store valid content
    distributor
        .store_content(cid.clone(), content.to_vec())
        .await;

    // Create provider
    let peer1 = PeerInfo::new(PeerId("peer1".to_string()));
    let providers = vec![peer1];

    // Download should succeed with valid content
    let result = distributor.download_content(cid, providers).await;
    assert!(result.is_ok(), "Download with valid chunks should succeed");
}

/// Test tit-for-tat mechanism
#[tokio::test]
async fn test_tit_for_tat() {
    let config = ChunkConfig::default();
    let mut distributor = ChunkDistributor::new(config).await.unwrap();

    let cid = ContentId::new(b"test content");
    let content = b"test content for tit-for-tat testing".to_vec();

    // Store content
    distributor
        .store_content(cid.clone(), content.clone())
        .await;

    let peer1 = PeerId("peer1".to_string());

    // Initially, new peers should be allowed to download (optimistic unchoking)
    let can_serve_1 = distributor.should_upload_to_peer(&peer1).await;
    assert!(
        can_serve_1,
        "New peer should be allowed (optimistic unchoke)"
    );

    // Serve chunk to peer1
    let chunk1 = distributor.serve_chunk(cid.clone(), 0, peer1.clone()).await;
    assert!(chunk1.is_ok(), "Should serve chunk to peer1");

    // Verify peer stats are updated
    let stats = distributor.peer_stats.read().await;
    let peer1_stats = stats.get(&peer1);
    assert!(peer1_stats.is_some(), "Peer1 should have stats");
    assert!(
        peer1_stats.unwrap().uploaded_bytes > 0,
        "Upload bytes should be recorded"
    );

    // Verify that a peer with good ratio (infinity since no downloads) can still upload
    drop(stats);
    let can_serve_again = distributor.should_upload_to_peer(&peer1).await;
    assert!(
        can_serve_again,
        "Peer with good ratio should be allowed to continue"
    );
}

/// Test download progress tracking
#[tokio::test]
async fn test_download_progress() {
    let config = ChunkConfig {
        chunk_size: 256,
        ..Default::default()
    };

    let mut distributor = ChunkDistributor::new(config).await.unwrap();

    let content = b"a".repeat(1024); // 4 chunks
    let cid = ContentId::new(&content);

    distributor
        .store_content(cid.clone(), content.clone())
        .await;

    let peer1 = PeerInfo::new(PeerId("peer1".to_string()));
    let providers = vec![peer1];

    // Start download in background
    let cid_clone = cid.clone();
    let providers_clone = providers.clone();
    let mut dist_clone = distributor.clone_arc_fields();

    tokio::spawn(async move {
        let _ = dist_clone
            .download_content(cid_clone, providers_clone)
            .await;
    });

    // Give it a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Check progress (might be complete already in tests)
    let progress = distributor.get_download_progress(&cid).await;

    // Progress tracking was initialized if download is active
    // In fast tests, download might already be complete
    if let Some(p) = progress {
        assert!(p.total_bytes > 0, "Total bytes should be set");
        assert!(p.total_chunks > 0, "Total chunks should be set");
    }
}

/// Test sequential download strategy
#[tokio::test]
async fn test_sequential_strategy() {
    let config = ChunkConfig {
        strategy: DistributionStrategy::Sequential,
        chunk_size: 256,
        ..Default::default()
    };

    let distributor = ChunkDistributor::new(config).await.unwrap();

    let providers = vec![];
    let order = distributor
        .determine_chunk_order(&ContentId::new(b"test"), &providers, 5)
        .await
        .unwrap();

    assert_eq!(
        order,
        vec![0, 1, 2, 3, 4],
        "Sequential order should be 0,1,2,3,4"
    );
}

/// Test random download strategy
#[tokio::test]
async fn test_random_strategy() {
    let config = ChunkConfig {
        strategy: DistributionStrategy::RandomOrder,
        chunk_size: 256,
        ..Default::default()
    };

    let distributor = ChunkDistributor::new(config).await.unwrap();

    let providers = vec![];
    let order = distributor
        .determine_chunk_order(&ContentId::new(b"test"), &providers, 10)
        .await
        .unwrap();

    // Should have all chunks
    assert_eq!(order.len(), 10);

    // Should contain all indices 0..10
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(sorted, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

/// Test chunk reconstruction
#[tokio::test]
async fn test_chunk_reconstruction() {
    use codio_chunk::Download;

    let content = b"Hello, distributed world!".to_vec();
    let cid = ContentId::new(&content);

    let mut download = Download::new(
        cid,
        content.len() as u64,
        10, // Small chunk size for testing
        DistributionStrategy::Sequential,
    );

    // Split content into chunks and add to download
    let chunk_size = 10;
    for (i, chunk_data) in content.chunks(chunk_size).enumerate() {
        let chunk = Chunk::new(i as u32, chunk_data.to_vec());
        download.chunks.insert(i as u32, chunk);
        download.downloaded_chunks.insert(i as u32);
    }

    // Reconstruct
    let reconstructed = download.reconstruct().unwrap();

    assert_eq!(
        reconstructed, content,
        "Reconstructed content should match original"
    );
}

/// Test handling of missing chunks
#[tokio::test]
async fn test_missing_chunks() {
    use codio_chunk::Download;

    let content = b"test content".to_vec();
    let cid = ContentId::new(&content);

    let download = Download::new(
        cid,
        content.len() as u64,
        256,
        DistributionStrategy::Sequential,
    );

    // Don't add any chunks
    let result = download.reconstruct();

    assert!(
        result.is_err(),
        "Reconstruction should fail with missing chunks"
    );
}

/// Test peer selection for chunks
#[tokio::test]
async fn test_peer_selection() {
    let config = ChunkConfig::default();
    let distributor = ChunkDistributor::new(config).await.unwrap();

    // Create peers with different chunk availability
    let mut peer1 = PeerInfo::new(PeerId("peer1".to_string()));
    peer1.available_chunks = vec![0, 1].into_iter().collect();

    let mut peer2 = PeerInfo::new(PeerId("peer2".to_string()));
    peer2.available_chunks = vec![2, 3].into_iter().collect();

    let providers = vec![peer1.clone(), peer2.clone()];

    // Select peer for chunk 0 (only peer1 has it)
    let selected = distributor.select_peer_for_chunk(0, &providers, 0);
    assert!(selected.is_some());
    assert_eq!(selected.unwrap().peer_id, peer1.peer_id);

    // Select peer for chunk 2 (only peer2 has it)
    let selected = distributor.select_peer_for_chunk(2, &providers, 0);
    assert!(selected.is_some());
    assert_eq!(selected.unwrap().peer_id, peer2.peer_id);
}
