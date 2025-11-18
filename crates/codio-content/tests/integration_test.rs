//! Integration tests for codio-content library
//!
//! These tests verify the end-to-end functionality of the content addressing system.

use codio_content::{
    config::ContentConfig, hash_sha256, hash_sha256_hex, verify_hash, Chunk, Chunker, Content,
    ContentId, MerkleDAG, DEFAULT_CHUNK_SIZE,
};

// ============================================================================
// ContentId Tests
// ============================================================================

#[test]
fn test_cid_generation() {
    let content = b"Hello, Codio CDN!";
    let cid = ContentId::new(content);

    // Should start with Qm (IPFS CIDv0 format)
    assert!(cid.to_string().starts_with("Qm"));

    // Should verify correctly
    assert!(cid.verify(content));
}

#[test]
fn test_cid_deterministic() {
    let content = b"Test content";
    let cid1 = ContentId::new(content);
    let cid2 = ContentId::new(content);

    // Same content should produce same CID
    assert_eq!(cid1, cid2);
    assert_eq!(cid1.to_string(), cid2.to_string());
}

#[test]
fn test_cid_different_content() {
    let cid1 = ContentId::new(b"Content A");
    let cid2 = ContentId::new(b"Content B");

    // Different content should produce different CIDs
    assert_ne!(cid1, cid2);
    assert_ne!(cid1.to_string(), cid2.to_string());
}

#[test]
fn test_cid_roundtrip() {
    let original = ContentId::new(b"roundtrip test");
    let cid_string = original.to_string();
    let restored = ContentId::from_string(&cid_string).unwrap();

    assert_eq!(original, restored);
    assert_eq!(original.hash(), restored.hash());
}

#[test]
fn test_cid_invalid_prefix() {
    let result = ContentId::from_string("Zz123456789");
    assert!(result.is_err());
}

#[test]
fn test_cid_hash_hex() {
    let cid = ContentId::new(b"test");
    let hex = cid.hash_hex();

    assert_eq!(hex.len(), 64); // 32 bytes = 64 hex chars
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
}

// ============================================================================
// Chunking Tests
// ============================================================================

#[test]
fn test_chunking() {
    let data = vec![0u8; 5 * 1024 * 1024]; // 5MB
    let chunker = Chunker::new(1024 * 1024);
    let chunks = chunker.chunk(&data);

    assert_eq!(chunks.len(), 5);

    // Verify each chunk
    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.index, i as u32);
        assert_eq!(chunk.size(), 1024 * 1024);
        assert!(chunk.verify());
    }

    // Reconstruct and verify
    let reconstructed = chunker.reconstruct(chunks).unwrap();
    assert_eq!(data, reconstructed);
}

#[test]
fn test_chunking_partial_last_chunk() {
    let data = vec![0u8; 2500]; // 2.5KB
    let chunker = Chunker::new(1024); // 1KB chunks
    let chunks = chunker.chunk(&data);

    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].size(), 1024);
    assert_eq!(chunks[1].size(), 1024);
    assert_eq!(chunks[2].size(), 452); // Partial last chunk
}

#[test]
fn test_chunking_exact_boundary() {
    let data = vec![0u8; 3 * 1024]; // Exactly 3KB
    let chunker = Chunker::new(1024);
    let chunks = chunker.chunk(&data);

    assert_eq!(chunks.len(), 3);
    for chunk in &chunks {
        assert_eq!(chunk.size(), 1024);
    }
}

#[test]
fn test_chunking_empty_data() {
    let chunker = Chunker::new(1024);
    let chunks = chunker.chunk(&[]);
    assert_eq!(chunks.len(), 0);
}

#[test]
fn test_chunking_smaller_than_chunk_size() {
    let data = vec![0u8; 512];
    let chunker = Chunker::new(1024);
    let chunks = chunker.chunk(&data);

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].size(), 512);
}

#[test]
fn test_chunk_verification() {
    let mut chunk = Chunk::new(0, vec![1, 2, 3, 4, 5]);
    assert!(chunk.verify());

    // Tamper with data
    chunk.data[0] = 99;
    assert!(!chunk.verify());
}

#[test]
fn test_chunker_num_chunks() {
    let chunker = Chunker::new(1024);
    assert_eq!(chunker.num_chunks(0), 0);
    assert_eq!(chunker.num_chunks(512), 1);
    assert_eq!(chunker.num_chunks(1024), 1);
    assert_eq!(chunker.num_chunks(1025), 2);
    assert_eq!(chunker.num_chunks(2500), 3);
}

// ============================================================================
// Merkle DAG Tests
// ============================================================================

#[test]
fn test_merkle_dag() {
    let data = vec![0u8; 3 * 1024 * 1024];
    let chunker = Chunker::new(1024 * 1024);
    let chunks = chunker.chunk(&data);

    let dag = MerkleDAG::from_chunks(&chunks);

    assert_eq!(dag.num_children(), 3);
    assert!(dag.verify(&chunks));
    assert!(dag.verify_structure());
}

#[test]
fn test_merkle_dag_single_chunk() {
    let data = vec![0u8; 1024];
    let chunker = Chunker::new(1024);
    let chunks = chunker.chunk(&data);

    let dag = MerkleDAG::from_chunks(&chunks);

    assert_eq!(dag.num_children(), 1);
    assert!(dag.verify(&chunks));
}

#[test]
fn test_merkle_dag_root_cid() {
    let data = vec![0u8; 2048];
    let chunker = Chunker::new(1024);
    let chunks = chunker.chunk(&data);

    let dag = MerkleDAG::from_chunks(&chunks);
    let root_cid = dag.root_cid();

    assert!(root_cid.to_string().starts_with("Qm"));
}

#[test]
fn test_merkle_dag_children() {
    let data = vec![0u8; 2048];
    let chunker = Chunker::new(1024);
    let chunks = chunker.chunk(&data);

    let dag = MerkleDAG::from_chunks(&chunks);
    let children = dag.children();

    assert_eq!(children.len(), 2);
    assert_eq!(children[0], chunks[0].cid);
    assert_eq!(children[1], chunks[1].cid);
}

#[test]
fn test_merkle_dag_verify_tampered_chunk() {
    let data = vec![0u8; 2048];
    let chunker = Chunker::new(1024);
    let mut chunks = chunker.chunk(&data);

    let dag = MerkleDAG::from_chunks(&chunks);

    // Tamper with a chunk
    chunks[0].data[0] = 99;

    // Verification should fail
    assert!(!dag.verify(&chunks));
}

// ============================================================================
// Corruption Detection Tests
// ============================================================================

#[test]
fn test_corruption_detection() {
    let content = b"Original content";
    let cid = ContentId::new(content);

    let tampered = b"Tampered content";
    assert!(!cid.verify(tampered));
}

#[test]
fn test_corruption_detection_single_bit() {
    let mut data = vec![0u8; 1024];
    let cid = ContentId::new(&data);

    // Flip a single bit
    data[512] ^= 0x01;

    assert!(!cid.verify(&data));
}

#[test]
fn test_corruption_detection_in_chunks() {
    let data = vec![0u8; 2048];
    let chunker = Chunker::new(1024);
    let mut chunks = chunker.chunk(&data);

    // Corrupt second chunk
    chunks[1].data[0] = 42;

    // Individual chunk verification should fail
    assert!(!chunks[1].verify());

    // Reconstruction should fail
    let result = chunker.reconstruct(chunks);
    assert!(result.is_err());
}

// ============================================================================
// Content API Tests
// ============================================================================

#[test]
fn test_content_new() {
    let data = vec![0u8; 5 * 1024 * 1024]; // 5MB
    let content = Content::new(data.clone(), DEFAULT_CHUNK_SIZE);

    assert_eq!(content.size(), data.len());
    assert_eq!(content.num_chunks(), 5);
    assert!(content.verify());
}

#[test]
fn test_content_reconstruct() {
    let original = vec![0u8; 2 * 1024 * 1024];
    let content = Content::new(original.clone(), DEFAULT_CHUNK_SIZE);

    let reconstructed = content.reconstruct().unwrap();
    assert_eq!(original, reconstructed);
}

#[test]
fn test_content_get_chunk() {
    let data = vec![0u8; 3 * 1024 * 1024];
    let content = Content::new(data, DEFAULT_CHUNK_SIZE);

    let chunk = content.get_chunk(1).unwrap();
    assert_eq!(chunk.index, 1);
    assert!(chunk.verify());

    let result = content.get_chunk(999);
    assert!(result.is_err());
}

#[test]
fn test_content_chunk_cids() {
    let data = vec![0u8; 2 * 1024 * 1024];
    let content = Content::new(data, DEFAULT_CHUNK_SIZE);

    let cids = content.chunk_cids();
    assert_eq!(cids.len(), 2);

    for cid in cids {
        assert!(cid.to_string().starts_with("Qm"));
    }
}

// ============================================================================
// Large File Tests
// ============================================================================

#[test]
fn test_large_file_chunking() {
    // 100MB file
    let data = vec![0u8; 100 * 1024 * 1024];
    let chunker = Chunker::new(DEFAULT_CHUNK_SIZE);
    let chunks = chunker.chunk(&data);

    assert_eq!(chunks.len(), 100);

    // Verify all chunks
    for chunk in &chunks {
        assert!(chunk.verify());
    }
}

#[test]
fn test_large_file_reconstruction() {
    // 50MB file
    let original = vec![0u8; 50 * 1024 * 1024];
    let content = Content::new(original.clone(), DEFAULT_CHUNK_SIZE);

    let reconstructed = content.reconstruct().unwrap();
    assert_eq!(original, reconstructed);
}

// ============================================================================
// Different Content Types Tests
// ============================================================================

#[test]
fn test_text_content() {
    let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(1000);
    let content = Content::new(text.as_bytes().to_vec(), 1024);

    assert!(content.verify());
    let reconstructed = content.reconstruct().unwrap();
    assert_eq!(text.as_bytes(), &reconstructed[..]);
}

#[test]
fn test_binary_content() {
    let mut data = Vec::new();
    for i in 0..10000 {
        data.push((i % 256) as u8);
    }

    let content = Content::new(data.clone(), 1024);
    assert!(content.verify());

    let reconstructed = content.reconstruct().unwrap();
    assert_eq!(data, reconstructed);
}

#[test]
fn test_random_content() {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};

    let mut data = Vec::new();
    let hasher_builder = RandomState::new();

    for i in 0..10000 {
        let mut hasher = hasher_builder.build_hasher();
        i.hash(&mut hasher);
        data.push((hasher.finish() % 256) as u8);
    }

    let content = Content::new(data.clone(), 1024);
    let reconstructed = content.reconstruct().unwrap();
    assert_eq!(data, reconstructed);
}

// ============================================================================
// ContentConfig Tests
// ============================================================================

#[test]
fn test_content_config_default() {
    let config = ContentConfig::default();
    assert_eq!(config.chunk_size, DEFAULT_CHUNK_SIZE);
    assert!(!config.enable_compression);
}

#[test]
fn test_content_config_custom() {
    let config = ContentConfig::new(512 * 1024);
    assert_eq!(config.chunk_size, 512 * 1024);
}

#[test]
fn test_content_config_validate() {
    let config = ContentConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_content_config_estimate_chunks() {
    let config = ContentConfig::new(1024 * 1024);
    assert_eq!(config.estimate_chunks(5 * 1024 * 1024), 5);
    assert_eq!(config.estimate_chunks(5 * 1024 * 1024 + 1), 6);
}

// ============================================================================
// Utility Function Tests
// ============================================================================

#[test]
fn test_hash_sha256() {
    let hash1 = hash_sha256(b"test");
    let hash2 = hash_sha256(b"test");
    assert_eq!(hash1, hash2);

    let hash3 = hash_sha256(b"different");
    assert_ne!(hash1, hash3);
}

#[test]
fn test_hash_sha256_hex() {
    let hex = hash_sha256_hex(b"test data");
    assert_eq!(hex.len(), 64);
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_verify_hash() {
    let data = b"test data";
    let hash = hash_sha256(data);
    assert!(verify_hash(data, &hash));
    assert!(!verify_hash(b"wrong data", &hash));
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_empty_content() {
    let content = Content::new(vec![], DEFAULT_CHUNK_SIZE);
    assert_eq!(content.size(), 0);
    assert_eq!(content.num_chunks(), 0);

    let reconstructed = content.reconstruct().unwrap();
    assert_eq!(reconstructed.len(), 0);
}

#[test]
fn test_single_byte_content() {
    let content = Content::new(vec![42], DEFAULT_CHUNK_SIZE);
    assert_eq!(content.size(), 1);
    assert_eq!(content.num_chunks(), 1);

    let reconstructed = content.reconstruct().unwrap();
    assert_eq!(reconstructed, vec![42]);
}

#[test]
fn test_chunk_boundary_minus_one() {
    let chunker = Chunker::new(1024);
    let data = vec![0u8; 1023];
    let chunks = chunker.chunk(&data);

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].size(), 1023);
}

#[test]
fn test_chunk_boundary_plus_one() {
    let chunker = Chunker::new(1024);
    let data = vec![0u8; 1025];
    let chunks = chunker.chunk(&data);

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].size(), 1024);
    assert_eq!(chunks[1].size(), 1);
}

// ============================================================================
// IPFS Compatibility Tests
// ============================================================================

#[test]
fn test_ipfs_cid_format() {
    let cid = ContentId::new(b"test");
    let cid_str = cid.to_string();

    // Must start with Qm
    assert!(cid_str.starts_with("Qm"));

    // Must be base58 encoded
    assert!(cid_str
        .chars()
        .all(|c| "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".contains(c)));

    // Length should be reasonable (typically 46 chars for CIDv0)
    assert!(cid_str.len() >= 40 && cid_str.len() <= 50);
}

#[test]
fn test_ipfs_cid_decode() {
    let original = ContentId::new(b"IPFS compatibility test");
    let encoded = original.to_string();
    let decoded = ContentId::from_string(&encoded).unwrap();

    assert_eq!(original.hash(), decoded.hash());
}

// ============================================================================
// Performance Smoke Tests
// ============================================================================

#[test]
fn test_performance_cid_generation() {
    let data = vec![0u8; 10 * 1024]; // 10KB

    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _cid = ContentId::new(&data);
    }
    let duration = start.elapsed();

    // Should complete 1000 CID generations in reasonable time
    assert!(duration.as_secs() < 5);
}

#[test]
fn test_performance_chunking() {
    let data = vec![0u8; 100 * 1024 * 1024]; // 100MB
    let chunker = Chunker::new(DEFAULT_CHUNK_SIZE);

    let start = std::time::Instant::now();
    let _chunks = chunker.chunk(&data);
    let duration = start.elapsed();

    // Should chunk 100MB in reasonable time
    assert!(duration.as_secs() < 10);
}
