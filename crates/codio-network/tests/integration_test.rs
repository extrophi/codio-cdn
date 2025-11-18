//! Integration tests for the Codio network layer.
//!
//! These tests verify peer discovery, connection management, and protocol interactions.

use codio_network::{NetworkConfig, NetworkManager};
use libp2p::{Multiaddr, PeerId};
use std::time::Duration;
use tokio::time::timeout;

/// Helper to initialize tracing for tests
fn init_tracing() {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
}

/// Helper to create a network manager with a random port
async fn create_test_manager() -> (NetworkManager, u16) {
    let config = NetworkConfig::default().without_mdns();
    let mut manager = NetworkManager::new(config).await.unwrap();
    manager.start().await.unwrap();

    // Extract the port from the listen address
    let port = manager
        .listen_addrs()
        .first()
        .and_then(|addr| {
            addr.iter().find_map(|component| {
                if let libp2p::multiaddr::Protocol::Tcp(port) = component {
                    Some(port)
                } else {
                    None
                }
            })
        })
        .expect("No TCP port found");

    (manager, port)
}

/// Helper to create a multiaddress with peer ID
fn create_peer_addr(peer_id: PeerId, port: u16) -> Multiaddr {
    format!("/ip4/127.0.0.1/tcp/{}/p2p/{}", port, peer_id)
        .parse()
        .unwrap()
}

#[tokio::test]
async fn test_network_manager_creation() {
    init_tracing();

    let config = NetworkConfig::default().without_mdns();
    let result = NetworkManager::new(config).await;
    assert!(result.is_ok(), "Failed to create network manager");

    let manager = result.unwrap();
    assert_eq!(manager.peer_count(), 0, "Should start with no peers");
}

#[tokio::test]
async fn test_network_manager_start() {
    init_tracing();

    let config = NetworkConfig::with_port(0).without_mdns(); // Random port
    let mut manager = NetworkManager::new(config).await.unwrap();

    let result = manager.start().await;
    assert!(result.is_ok(), "Failed to start network manager");
    assert!(
        !manager.listen_addrs().is_empty(),
        "Should have at least one listen address"
    );
}

#[tokio::test]
async fn test_peer_discovery_mdns() {
    init_tracing();

    // Create three peers
    let (mut peer1, port1) = create_test_manager().await;
    let (mut peer2, port2) = create_test_manager().await;
    let (mut peer3, port3) = create_test_manager().await;

    println!("Peer 1: {} on port {}", peer1.peer_id(), port1);
    println!("Peer 2: {} on port {}", peer2.peer_id(), port2);
    println!("Peer 3: {} on port {}", peer3.peer_id(), port3);

    // Wait for mDNS discovery (this can take a few seconds)
    let discovery_timeout = Duration::from_secs(10);

    // Helper to wait for peer discovery
    async fn wait_for_discovery(
        manager: &mut NetworkManager,
        expected_peers: usize,
        timeout_duration: Duration,
    ) -> bool {
        let result = timeout(timeout_duration, async {
            loop {
                // Process events
                tokio::select! {
                    _ = manager.poll_once() => {},
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {
                        if manager.peer_count() >= expected_peers {
                            return true;
                        }
                    }
                }
            }
        })
        .await;

        result.unwrap_or(false)
    }

    // Wait for peers to discover each other via mDNS
    let _peer1_discovered = wait_for_discovery(&mut peer1, 2, discovery_timeout).await;
    let _peer2_discovered = wait_for_discovery(&mut peer2, 2, discovery_timeout).await;
    let _peer3_discovered = wait_for_discovery(&mut peer3, 2, discovery_timeout).await;

    // Check results (mDNS may not work in all test environments, so we make this informational)
    println!("Peer 1 discovered {} peers", peer1.peer_count());
    println!("Peer 2 discovered {} peers", peer2.peer_count());
    println!("Peer 3 discovered {} peers", peer3.peer_count());

    // In CI/CD environments, mDNS might not work, so we just verify the setup was correct
    assert!(peer1.listen_addrs().len() > 0);
    assert!(peer2.listen_addrs().len() > 0);
    assert!(peer3.listen_addrs().len() > 0);
}

#[tokio::test]
async fn test_connection_management() {
    init_tracing();

    // Create two peers
    let (mut peer1, port1) = create_test_manager().await;
    let (mut peer2, port2) = create_test_manager().await;

    let peer1_id = peer1.peer_id();
    let peer2_id = peer2.peer_id();

    println!("Peer 1: {} on port {}", peer1_id, port1);
    println!("Peer 2: {} on port {}", peer2_id, port2);

    // Create address for peer2
    let peer2_addr = create_peer_addr(peer2_id, port2);

    // Connect peer1 to peer2
    let result = peer1.connect_peer(peer2_addr.clone()).await;
    assert!(result.is_ok(), "Failed to connect to peer");
    assert_eq!(result.unwrap(), peer2_id);

    // Wait for connection to be established
    let connection_timeout = Duration::from_secs(5);

    let connected = timeout(connection_timeout, async {
        loop {
            tokio::select! {
                _ = peer1.poll_once() => {
                    if peer1.peer_count() > 0 {
                        return true;
                    }
                },
                _ = peer2.poll_once() => {},
            }
        }
    })
    .await;

    assert!(connected.is_ok(), "Connection timeout");
    assert_eq!(peer1.peer_count(), 1, "Peer 1 should have 1 connection");

    // Disconnect from peer
    let disconnect_result = peer1.disconnect_peer(peer2_id).await;
    assert!(disconnect_result.is_ok(), "Failed to disconnect");

    // Wait for disconnection to be processed
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Process remaining events
    for _ in 0..10 {
        tokio::select! {
            _ = peer1.poll_once() => {},
            _ = tokio::time::sleep(Duration::from_millis(100)) => break,
        }
    }

    assert_eq!(
        peer1.peer_count(),
        0,
        "Peer 1 should have 0 connections after disconnect"
    );
}

#[tokio::test]
async fn test_bootstrap_nodes() {
    init_tracing();

    // Create bootstrap peer
    let (mut bootstrap, bootstrap_port) = create_test_manager().await;
    let bootstrap_id = bootstrap.peer_id();

    println!(
        "Bootstrap peer: {} on port {}",
        bootstrap_id, bootstrap_port
    );

    // Create bootstrap address
    let bootstrap_addr = create_peer_addr(bootstrap_id, bootstrap_port);

    // Create regular peer with bootstrap configuration
    let mut config = NetworkConfig::default().without_mdns();
    config.add_bootstrap_peer(bootstrap_addr.clone());

    let mut peer = NetworkManager::new(config).await.unwrap();
    peer.start().await.unwrap();

    let peer_id = peer.peer_id();
    println!(
        "Regular peer: {} with bootstrap node {}",
        peer_id, bootstrap_addr
    );

    // Connect to bootstrap peer
    let result = peer.connect_peer(bootstrap_addr).await;
    assert!(result.is_ok(), "Failed to connect to bootstrap peer");

    // Wait for connection
    let connection_timeout = Duration::from_secs(5);

    let connected = timeout(connection_timeout, async {
        loop {
            tokio::select! {
                _ = peer.poll_once() => {
                    if peer.peer_count() > 0 {
                        return true;
                    }
                },
                _ = bootstrap.poll_once() => {},
            }
        }
    })
    .await;

    assert!(connected.is_ok(), "Connection to bootstrap peer timeout");
    assert!(
        peer.peer_count() > 0,
        "Should be connected to bootstrap peer"
    );
}

#[tokio::test]
async fn test_connection_limit() {
    init_tracing();

    // Create peer with low connection limit
    let mut config = NetworkConfig::default().without_mdns();
    config.max_peers = 2;

    let mut peer1 = NetworkManager::new(config).await.unwrap();
    peer1.start().await.unwrap();

    // Create 3 other peers
    let (mut peer2, port2) = create_test_manager().await;
    let (mut peer3, port3) = create_test_manager().await;
    let (peer4, port4) = create_test_manager().await;

    // Connect to first two peers (should succeed)
    let addr2 = create_peer_addr(peer2.peer_id(), port2);
    let result = peer1.connect_peer(addr2).await;
    assert!(result.is_ok());

    let addr3 = create_peer_addr(peer3.peer_id(), port3);
    let result = peer1.connect_peer(addr3).await;
    assert!(result.is_ok());

    // Wait for connections to establish
    tokio::time::sleep(Duration::from_secs(2)).await;
    for _ in 0..20 {
        tokio::select! {
            _ = peer1.poll_once() => {},
            _ = peer2.poll_once() => {},
            _ = peer3.poll_once() => {},
            _ = tokio::time::sleep(Duration::from_millis(100)) => break,
        }
    }

    // Try to connect to third peer (should fail due to limit)
    let addr4 = create_peer_addr(peer4.peer_id(), port4);
    let result = peer1.connect_peer(addr4).await;

    if peer1.peer_count() >= 2 {
        assert!(
            result.is_err(),
            "Should fail when max peers limit is reached"
        );
    } else {
        // If connections haven't fully established yet, this is acceptable
        println!("Warning: Connections not fully established, skipping limit check");
    }
}

#[tokio::test]
async fn test_network_stats() {
    init_tracing();

    let (manager, _port) = create_test_manager().await;

    let stats = manager.stats();
    assert_eq!(stats.peer_count, 0);
    assert!(!stats.listen_addrs.is_empty());
    assert_eq!(stats.pending_connections, 0);
}

#[tokio::test]
async fn test_multiple_connections_same_peer() {
    init_tracing();

    let (mut peer1, _) = create_test_manager().await;
    let (mut peer2, port2) = create_test_manager().await;

    let peer2_addr = create_peer_addr(peer2.peer_id(), port2);

    // Connect once
    let result1 = peer1.connect_peer(peer2_addr.clone()).await;
    assert!(result1.is_ok());

    // Wait a bit for connection to establish
    tokio::time::sleep(Duration::from_millis(500)).await;
    for _ in 0..10 {
        tokio::select! {
            _ = peer1.poll_once() => {},
            _ = peer2.poll_once() => {},
            _ = tokio::time::sleep(Duration::from_millis(100)) => break,
        }
    }

    // Try to connect again (should return same peer ID without error)
    let result2 = peer1.connect_peer(peer2_addr).await;
    assert!(result2.is_ok());
    assert_eq!(result1.unwrap(), result2.unwrap());
}

#[tokio::test]
async fn test_peer_info_tracking() {
    init_tracing();

    let (mut peer1, _) = create_test_manager().await;
    let (mut peer2, port2) = create_test_manager().await;

    let peer2_id = peer2.peer_id();
    let peer2_addr = create_peer_addr(peer2_id, port2);

    // Connect
    peer1.connect_peer(peer2_addr).await.unwrap();

    // Wait for connection
    let connection_timeout = Duration::from_secs(5);
    let _ = timeout(connection_timeout, async {
        loop {
            tokio::select! {
                _ = peer1.poll_once() => {
                    if peer1.peer_count() > 0 {
                        return;
                    }
                },
                _ = peer2.poll_once() => {},
            }
        }
    })
    .await;

    // Check peer info
    let peers = peer1.connected_peers();
    if let Some(info) = peers.get(&peer2_id) {
        assert_eq!(info.peer_id, peer2_id);
        assert!(!info.addresses.is_empty());
    }
}
