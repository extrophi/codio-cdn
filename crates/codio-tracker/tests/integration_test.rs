use codio_common::{ContentId, PeerId};
use codio_tracker::{AvailabilityTracker, PeerMetrics, TrackerConfig};
use std::time::Duration;

#[test]
fn test_reputation_calculation() {
    let config = TrackerConfig::default();
    let peer_id = PeerId("test_peer".to_string());
    let mut metrics = PeerMetrics::new(peer_id);

    // High success rate: 100 successes, 10 failures
    metrics.chunks_served = 100;
    metrics.chunks_failed = 10;
    metrics.update_reputation(&config);

    let reputation = metrics.calculate_reputation(&config);

    // Success rate = 100/110 = 0.909
    // With default weights (0.7 success, 0.3 uptime)
    // reputation = 0.909 * 0.7 + uptime_factor * 0.3
    assert!(
        reputation > 0.6,
        "Reputation should be > 0.6 for high success rate"
    );
    assert!(reputation <= 1.0, "Reputation should be <= 1.0");
}

#[test]
fn test_reputation_calculation_zero_chunks() {
    let config = TrackerConfig::default();
    let peer_id = PeerId("new_peer".to_string());
    let metrics = PeerMetrics::new(peer_id);

    let reputation = metrics.calculate_reputation(&config);

    // New peer with no chunks should have neutral-ish reputation
    assert!(reputation >= 0.0);
    assert!(reputation <= 1.0);
}

#[test]
fn test_reputation_calculation_poor_performance() {
    let config = TrackerConfig::default();
    let peer_id = PeerId("bad_peer".to_string());
    let mut metrics = PeerMetrics::new(peer_id);

    // Low success rate: 10 successes, 90 failures
    metrics.chunks_served = 10;
    metrics.chunks_failed = 90;
    metrics.update_reputation(&config);

    let reputation = metrics.calculate_reputation(&config);

    // Success rate = 10/100 = 0.1
    assert!(
        reputation < 0.3,
        "Reputation should be < 0.3 for poor performance"
    );
}

#[tokio::test]
async fn test_load_balancing() {
    let config = TrackerConfig::default();
    let tracker = AvailabilityTracker::new(config).await.unwrap();
    let cid = ContentId::new(b"test content for load balancing");

    // Create 5 peers with different performance characteristics
    let peers = vec![
        ("peer_excellent", 100, 0, 5_000_000), // 100% success, 5 MB/s
        ("peer_good", 80, 20, 3_000_000),      // 80% success, 3 MB/s
        ("peer_average", 60, 40, 2_000_000),   // 60% success, 2 MB/s
        ("peer_poor", 30, 70, 1_000_000),      // 30% success, 1 MB/s
        ("peer_terrible", 10, 90, 500_000),    // 10% success, 500 KB/s
    ];

    for (name, success, failure, speed) in peers {
        let peer_id = PeerId(name.to_string());
        tracker.record_peer_online(peer_id.clone()).await;
        tracker
            .record_content_available(cid.clone(), peer_id.clone())
            .await;

        // Record success/failure pattern
        for _ in 0..success {
            tracker.record_upload_success(peer_id.clone()).await;
        }
        for _ in 0..failure {
            tracker.record_upload_failure(peer_id.clone()).await;
        }

        // Record download speed
        tracker.record_download_speed(peer_id.clone(), speed).await;
    }

    // Select best 3 peers
    let best_peers = tracker.select_best_peers(&cid, 3).await;

    assert_eq!(best_peers.len(), 3, "Should select exactly 3 peers");

    // Verify selection is based on reputation (highest reputation first)
    let best_peer_ids: Vec<String> = best_peers.iter().map(|p| p.peer_id.0.clone()).collect();

    // The best peer should be "peer_excellent"
    assert_eq!(
        best_peer_ids[0], "peer_excellent",
        "Best peer should be peer_excellent"
    );

    // Poor and terrible peers should not be in top 3
    assert!(
        !best_peer_ids.contains(&"peer_terrible".to_string()),
        "Terrible peer should not be selected"
    );
    assert!(
        !best_peer_ids.contains(&"peer_poor".to_string()),
        "Poor peer should not be selected"
    );
}

#[tokio::test]
async fn test_load_balancing_with_offline_peers() {
    let config = TrackerConfig::default();
    let tracker = AvailabilityTracker::new(config).await.unwrap();
    let cid = ContentId::new(b"test content");

    // Create 3 peers
    let peer1 = PeerId("peer1".to_string());
    let peer2 = PeerId("peer2".to_string());
    let peer3 = PeerId("peer3".to_string());

    for peer in [&peer1, &peer2, &peer3] {
        tracker.record_peer_online(peer.clone()).await;
        tracker
            .record_content_available(cid.clone(), peer.clone())
            .await;
        tracker.record_upload_success(peer.clone()).await;
    }

    // Mark peer2 as offline
    tracker.record_peer_offline(peer2.clone()).await;

    let best_peers = tracker.select_best_peers(&cid, 3).await;

    // Should only get 2 peers (peer1 and peer3), not peer2
    assert_eq!(best_peers.len(), 2);
    assert!(!best_peers.iter().any(|p| p.peer_id == peer2));
}

#[tokio::test]
async fn test_peer_timeout() {
    let mut config = TrackerConfig::default();
    config.peer_timeout = Duration::from_millis(200); // 200ms timeout

    let tracker = AvailabilityTracker::new(config).await.unwrap();
    let peer_id = PeerId("timeout_peer".to_string());

    // Record peer online
    tracker.record_peer_online(peer_id.clone()).await;

    // Verify peer exists
    let metrics = tracker.get_peer_metrics(&peer_id).await;
    assert!(metrics.is_some(), "Peer should exist initially");

    // Wait for timeout period
    tokio::time::sleep(Duration::from_millis(250)).await;

    // Clean up stale peers
    tracker.cleanup_stale_peers().await;

    // Verify peer is removed
    let metrics = tracker.get_peer_metrics(&peer_id).await;
    assert!(metrics.is_none(), "Peer should be removed after timeout");
}

#[tokio::test]
async fn test_peer_timeout_with_activity() {
    let mut config = TrackerConfig::default();
    config.peer_timeout = Duration::from_millis(300);

    let tracker = AvailabilityTracker::new(config).await.unwrap();
    let peer_id = PeerId("active_peer".to_string());

    // Record peer online
    tracker.record_peer_online(peer_id.clone()).await;

    // Keep peer active by recording success
    for _ in 0..5 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        tracker.record_upload_success(peer_id.clone()).await;
    }

    // Clean up stale peers
    tracker.cleanup_stale_peers().await;

    // Verify peer still exists (was active)
    let metrics = tracker.get_peer_metrics(&peer_id).await;
    assert!(metrics.is_some(), "Active peer should not be removed");
}

#[tokio::test]
async fn test_content_replication_tracking() {
    let config = TrackerConfig::default();
    let tracker = AvailabilityTracker::new(config).await.unwrap();
    let cid = ContentId::new(b"replicated content");

    // Add multiple peers with the same content
    for i in 0..5 {
        let peer_id = PeerId(format!("peer_{}", i));
        tracker.record_content_available(cid.clone(), peer_id).await;
    }

    let available_peers = tracker.get_available_peers(&cid).await;
    assert_eq!(available_peers.len(), 5, "Should have 5 peers with content");
}

#[tokio::test]
async fn test_gossip_protocol() {
    let config = TrackerConfig::default();
    let tracker1 = AvailabilityTracker::new(config.clone()).await.unwrap();
    let tracker2 = AvailabilityTracker::new(config).await.unwrap();

    // Add peers to tracker1
    let peer1 = PeerId("peer1".to_string());
    let peer2 = PeerId("peer2".to_string());

    tracker1.record_peer_online(peer1.clone()).await;
    tracker1.record_peer_online(peer2.clone()).await;
    tracker1.record_upload_success(peer1.clone()).await;
    tracker1.record_upload_success(peer2.clone()).await;

    // Get gossip from tracker1
    let gossip = tracker1.get_gossip_update().await;
    assert!(
        !gossip.peer_summaries.is_empty(),
        "Gossip should contain peer summaries"
    );

    // Apply gossip to tracker2
    tracker2.apply_gossip_update(gossip).await;

    // Verify tracker2 learned about the peers
    let metrics = tracker2.get_peer_metrics(&peer1).await;
    assert!(
        metrics.is_some(),
        "Tracker2 should know about peer1 via gossip"
    );
}

#[tokio::test]
async fn test_bandwidth_rolling_average() {
    let config = TrackerConfig::default();
    let tracker = AvailabilityTracker::new(config).await.unwrap();
    let peer_id = PeerId("speed_test_peer".to_string());

    // Record multiple speed samples
    let speeds = vec![1_000_000, 2_000_000, 3_000_000, 4_000_000, 5_000_000];
    for speed in speeds {
        tracker.record_download_speed(peer_id.clone(), speed).await;
    }

    let metrics = tracker.get_peer_metrics(&peer_id).await.unwrap();

    // Rolling average of all 5 samples
    let expected_avg = (1_000_000 + 2_000_000 + 3_000_000 + 4_000_000 + 5_000_000) / 5;
    assert_eq!(
        metrics.download_speed, expected_avg,
        "Should calculate correct rolling average"
    );
}

#[tokio::test]
async fn test_tracker_stats() {
    let config = TrackerConfig::default();
    let tracker = AvailabilityTracker::new(config).await.unwrap();

    // Add some peers
    let peer1 = PeerId("peer1".to_string());
    let peer2 = PeerId("peer2".to_string());
    let peer3 = PeerId("peer3".to_string());

    tracker.record_peer_online(peer1.clone()).await;
    tracker.record_peer_online(peer2.clone()).await;
    tracker.record_peer_online(peer3.clone()).await;

    // Make peer1 reliable
    for _ in 0..100 {
        tracker.record_upload_success(peer1.clone()).await;
    }

    // Mark peer3 as offline
    tracker.record_peer_offline(peer3).await;

    // Add some content
    let cid1 = ContentId::new(b"content1");
    let cid2 = ContentId::new(b"content2");
    tracker.record_content_available(cid1, peer1.clone()).await;
    tracker.record_content_available(cid2, peer1).await;

    let stats = tracker.get_stats().await;

    assert_eq!(stats.total_peers, 3, "Should have 3 total peers");
    assert_eq!(stats.online_peers, 2, "Should have 2 online peers");
    assert_eq!(stats.total_content, 2, "Should have 2 content items");
    assert!(
        stats.avg_reputation > 0.0,
        "Average reputation should be > 0"
    );
}

#[tokio::test]
async fn test_select_best_peers_respects_count() {
    let config = TrackerConfig::default();
    let tracker = AvailabilityTracker::new(config).await.unwrap();
    let cid = ContentId::new(b"test content");

    // Create 10 peers
    for i in 0..10 {
        let peer_id = PeerId(format!("peer_{}", i));
        tracker.record_peer_online(peer_id.clone()).await;
        tracker
            .record_content_available(cid.clone(), peer_id.clone())
            .await;
        tracker.record_upload_success(peer_id).await;
    }

    // Request only 3 peers
    let best_peers = tracker.select_best_peers(&cid, 3).await;
    assert_eq!(best_peers.len(), 3, "Should return exactly 3 peers");

    // Request more peers than available
    let many_peers = tracker.select_best_peers(&cid, 20).await;
    assert_eq!(many_peers.len(), 10, "Should return all 10 available peers");
}

#[tokio::test]
async fn test_min_reputation_filter() {
    let mut config = TrackerConfig::default();
    config.min_reputation = 0.5; // Set higher threshold

    let tracker = AvailabilityTracker::new(config).await.unwrap();
    let cid = ContentId::new(b"test content");

    // Create peer with low reputation
    let bad_peer = PeerId("bad_peer".to_string());
    tracker.record_peer_online(bad_peer.clone()).await;
    tracker
        .record_content_available(cid.clone(), bad_peer.clone())
        .await;

    // Give it a bad reputation
    tracker.record_upload_success(bad_peer.clone()).await;
    for _ in 0..20 {
        tracker.record_upload_failure(bad_peer.clone()).await;
    }

    // Create peer with good reputation
    let good_peer = PeerId("good_peer".to_string());
    tracker.record_peer_online(good_peer.clone()).await;
    tracker
        .record_content_available(cid.clone(), good_peer.clone())
        .await;

    for _ in 0..20 {
        tracker.record_upload_success(good_peer.clone()).await;
    }

    let best_peers = tracker.select_best_peers(&cid, 10).await;

    // Should only get good_peer, not bad_peer
    assert_eq!(
        best_peers.len(),
        1,
        "Should only select peer with reputation >= 0.5"
    );
    assert_eq!(best_peers[0].peer_id.0, "good_peer");
}
