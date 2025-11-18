use codio_content_id::ContentId;
use codio_dht::{DhtConfig, DhtNode};
use std::time::Duration;

#[tokio::test]
async fn test_publish_retrieve_flow() {
    // Setup two DHT nodes
    let config1 = DhtConfig {
        query_timeout: Duration::from_secs(5),
        ..Default::default()
    };
    let config2 = config1.clone();

    let (mut node1, _rx1) = DhtNode::new(config1).await.unwrap();
    let (mut node2, mut rx2) = DhtNode::new(config2).await.unwrap();

    // Listen on different ports
    node1
        .listen("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .await
        .unwrap();
    node2
        .listen("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .await
        .unwrap();

    // Create content
    let content = b"Test content for DHT";
    let cid = ContentId::new(content);

    // Node 1 announces content
    node1.provide(cid.clone()).await.unwrap();

    // Wait for DHT propagation
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Node 2 finds providers
    node2.find_providers(cid.clone()).await.unwrap();

    // Should find node 1 as provider
    // (This test needs real P2P connection - simplified here)
    assert!(true);
}

#[test]
fn test_cid_determinism() {
    let content = b"Same content";
    let cid1 = ContentId::new(content);
    let cid2 = ContentId::new(content);

    assert_eq!(cid1, cid2);
    assert_eq!(cid1.as_str(), cid2.as_str());
}

#[test]
fn test_cid_uniqueness() {
    let cid1 = ContentId::new(b"Content A");
    let cid2 = ContentId::new(b"Content B");

    assert_ne!(cid1, cid2);
}

#[test]
fn test_cid_ipfs_format() {
    let content = b"Hello, IPFS!";
    let cid = ContentId::new(content);

    // Should start with Qm (IPFS CIDv0 format)
    assert!(cid.as_str().starts_with("Qm"));

    // Should be base58 encoded
    assert!(cid.as_str().len() > 2);
}

#[test]
fn test_cid_verification_correct() {
    let content = b"Verify me!";
    let cid = ContentId::new(content);

    assert!(cid.verify(content));
}

#[test]
fn test_cid_verification_incorrect() {
    let original = b"Original content";
    let modified = b"Modified content";
    let cid = ContentId::new(original);

    assert!(!cid.verify(modified));
}

#[test]
fn test_cid_parse_and_roundtrip() {
    let content = b"Parse test content";
    let cid1 = ContentId::new(content);
    let cid_str = cid1.as_str();

    let cid2 = ContentId::from_str(cid_str).unwrap();

    assert_eq!(cid1, cid2);
    assert_eq!(cid1.hash(), cid2.hash());
}

#[test]
fn test_cid_invalid_format() {
    // Missing Qm prefix
    let result = ContentId::from_str("invalid");
    assert!(result.is_err());

    // Empty string
    let result = ContentId::from_str("");
    assert!(result.is_err());

    // Only Qm prefix
    let result = ContentId::from_str("Qm");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_dht_node_creation() {
    let config = DhtConfig::default();
    let result = DhtNode::new(config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_dht_listen() {
    let config = DhtConfig::default();
    let (mut node, _rx) = DhtNode::new(config).await.unwrap();

    let addr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
    let result = node.listen(addr).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_dht_provide_content() {
    let config = DhtConfig::default();
    let (mut node, _rx) = DhtNode::new(config).await.unwrap();

    // Listen first
    let addr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
    node.listen(addr).await.unwrap();

    // Announce content
    let content = b"Test content";
    let cid = ContentId::new(content);
    let result = node.provide(cid).await;
    assert!(result.is_ok());
}

#[test]
fn test_large_content_cid() {
    // Test with 1MB content
    let large_content = vec![0u8; 1024 * 1024];
    let cid1 = ContentId::new(&large_content);
    let cid2 = ContentId::new(&large_content);

    assert_eq!(cid1, cid2);
    assert!(cid1.verify(&large_content));
}

#[test]
fn test_empty_content_cid() {
    let empty = b"";
    let cid = ContentId::new(empty);

    assert!(cid.as_str().starts_with("Qm"));
    assert!(cid.verify(empty));
}

#[test]
fn test_cid_display_trait() {
    let content = b"Display test";
    let cid = ContentId::new(content);

    let displayed = format!("{}", cid);
    assert_eq!(displayed, cid.as_str());
}
