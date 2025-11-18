//! Integration tests for codio-content crate.
//!
//! These tests verify the complete workflow of content addressing,
//! chunking, and Merkle DAG construction.

use codio_content::{Chunker, ContentConfig, ContentId, MerkleDAG};

// ============================================================================
// CID Generation Tests
// ============================================================================

#[test]
fn test_cid_generation() {
    let content = b"Hello, Codio CDN!";
    let cid = ContentId::new(content);

    // CID must start with "Qm" (IPFS CIDv0)
    assert!(cid.to_string().starts_with("Qm"));

    // Verify content matches CID
    assert!(cid.verify(content));

    // Tampered content should fail verification
    assert!(!cid.verify(b"Tampered content"));
}

#[test]
fn test_cid_deterministic() {
    let content = b"Same content produces same CID";

    let cid1 = ContentId::new(content);
    let cid2 = ContentId::new(content);

    assert_eq!(cid1, cid2);
    assert_eq!(cid1.to_string(), cid2.to_string());
}

#[test]
fn test_cid_roundtrip() {
    let content = b"Roundtrip test content";
    let cid1 = ContentId::new(content);

    // Parse CID from string
    let cid2 = ContentId::from_string(&cid1.to_string()).unwrap();

    assert_eq!(cid1, cid2);
    assert_eq!(cid1.hash(), cid2.hash());
}

#[test]
fn test_cid_invalid_format() {
    // CID without "Qm" prefix
    let result = ContentId::from_string("Zm123456");
    assert!(result.is_err());

    // Invalid base58
    let result = ContentId::from_string("Qm!!!invalid!!!");
    assert!(result.is_err());
}

// ============================================================================
// Chunking Tests
// ============================================================================

#[test]
fn test_chunking() {
    let data = vec![0u8; 5 * 1024 * 1024]; // 5MB
    let chunker = Chunker::new(1024 * 1024); // 1MB chunks
    let chunks = chunker.chunk(&data);

    // Should create 5 chunks
    assert_eq!(chunks.len(), 5);

    // Verify all chunks
    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.index as usize, i);
        assert_eq!(chunk.size(), 1024 * 1024);
        assert!(chunk.verify());
    }

    // Reconstruct and verify
    let reconstructed = chunker.reconstruct(chunks).unwrap();
    assert_eq!(data, reconstructed);
}

#[test]
fn test_chunking_partial_last_chunk() {
    let data = vec![0u8; 2500 * 1024]; // 2.5MB
    let chunker = Chunker::new(1024 * 1024); // 1MB chunks
    let chunks = chunker.chunk(&data);

    // Should create 3 chunks (1MB, 1MB, ~452KB)
    // 2500KB = 2,560,000 bytes
    // 2,560,000 - 2*1,048,576 = 462,848 bytes
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].size(), 1024 * 1024);
    assert_eq!(chunks[1].size(), 1024 * 1024);
    assert_eq!(chunks[2].size(), 2500 * 1024 - 2 * 1024 * 1024);

    // All chunks should verify
    for chunk in &chunks {
        assert!(chunk.verify());
    }

    // Reconstruct should work
    let reconstructed = chunker.reconstruct(chunks).unwrap();
    assert_eq!(data, reconstructed);
}

#[test]
fn test_chunking_empty_data() {
    let data = vec![];
    let chunker = Chunker::new(1024 * 1024);
    let chunks = chunker.chunk(&data);

    assert_eq!(chunks.len(), 0);

    // Reconstruct empty data
    let reconstructed = chunker.reconstruct(chunks).unwrap();
    assert_eq!(reconstructed, Vec::<u8>::new());
}

#[test]
fn test_chunking_small_data() {
    let data = b"Small data less than chunk size".to_vec();
    let chunker = Chunker::new(1024 * 1024);
    let chunks = chunker.chunk(&data);

    // Should create single chunk
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].size(), data.len());

    let reconstructed = chunker.reconstruct(chunks).unwrap();
    assert_eq!(data, reconstructed);
}

#[test]
fn test_reconstruct_fails_out_of_order() {
    let data = vec![0u8; 2048];
    let chunker = Chunker::new(1024);
    let mut chunks = chunker.chunk(&data);

    // Swap chunks to create wrong order
    chunks.swap(0, 1);

    let result = chunker.reconstruct(chunks);
    assert!(result.is_err());
}

#[test]
fn test_reconstruct_fails_tampered() {
    let data = vec![0u8; 2048];
    let chunker = Chunker::new(1024);
    let mut chunks = chunker.chunk(&data);

    // Tamper with chunk data
    chunks[0].data[0] = 255;

    let result = chunker.reconstruct(chunks);
    assert!(result.is_err());
}

// ============================================================================
// Merkle DAG Tests
// ============================================================================

#[test]
fn test_merkle_dag() {
    let data = vec![0u8; 3 * 1024 * 1024]; // 3MB
    let chunker = Chunker::new(1024 * 1024);
    let chunks = chunker.chunk(&data);

    let dag = MerkleDAG::from_chunks(&chunks);

    // Verify DAG structure
    assert_eq!(dag.num_children(), 3);
    assert!(dag.root_cid().to_string().starts_with("Qm"));

    // Verify all chunks
    assert!(dag.verify(&chunks));
}

#[test]
fn test_merkle_dag_deterministic() {
    let data = vec![0u8; 2 * 1024 * 1024];
    let chunker = Chunker::new(1024 * 1024);

    let chunks1 = chunker.chunk(&data);
    let chunks2 = chunker.chunk(&data);

    let dag1 = MerkleDAG::from_chunks(&chunks1);
    let dag2 = MerkleDAG::from_chunks(&chunks2);

    // Same content should produce same root CID
    assert_eq!(dag1.root_cid(), dag2.root_cid());
}

#[test]
fn test_merkle_dag_different_content() {
    let chunker = Chunker::new(1024 * 1024);

    let data1 = vec![0u8; 2 * 1024 * 1024];
    let data2 = vec![1u8; 2 * 1024 * 1024];

    let chunks1 = chunker.chunk(&data1);
    let chunks2 = chunker.chunk(&data2);

    let dag1 = MerkleDAG::from_chunks(&chunks1);
    let dag2 = MerkleDAG::from_chunks(&chunks2);

    // Different content should produce different root CID
    assert_ne!(dag1.root_cid(), dag2.root_cid());
}

#[test]
fn test_merkle_dag_verify_fails_wrong_chunks() {
    let chunker = Chunker::new(1024);

    let data1 = vec![0u8; 2048];
    let data2 = vec![1u8; 2048];

    let chunks1 = chunker.chunk(&data1);
    let chunks2 = chunker.chunk(&data2);

    let dag = MerkleDAG::from_chunks(&chunks1);

    // Should fail with different chunks
    assert!(!dag.verify(&chunks2));
}

#[test]
fn test_merkle_dag_verify_individual_chunk() {
    let data = vec![0u8; 3 * 1024];
    let chunker = Chunker::new(1024);
    let chunks = chunker.chunk(&data);

    let dag = MerkleDAG::from_chunks(&chunks);

    // Verify each chunk individually
    for chunk in &chunks {
        assert!(dag.verify_chunk(chunk));
    }
}

#[test]
fn test_merkle_dag_contains_child() {
    let data = vec![0u8; 2048];
    let chunker = Chunker::new(1024);
    let chunks = chunker.chunk(&data);

    let dag = MerkleDAG::from_chunks(&chunks);

    // All chunk CIDs should be in the DAG
    for chunk in &chunks {
        assert!(dag.contains_child(&chunk.cid));
    }

    // Random CID should not be in the DAG
    let other_cid = ContentId::new(b"not in dag");
    assert!(!dag.contains_child(&other_cid));
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
    let mut content = vec![0u8; 1024];
    let cid = ContentId::new(&content);

    // Flip a single bit
    content[512] = 1;

    assert!(!cid.verify(&content));
}

#[test]
fn test_corruption_detection_in_chunks() {
    let data = vec![0u8; 2048];
    let chunker = Chunker::new(1024);
    let mut chunks = chunker.chunk(&data);

    // Tamper with one chunk
    chunks[0].data[0] = 255;

    // Reconstruction should fail
    let result = chunker.reconstruct(chunks);
    assert!(result.is_err());
}

#[test]
fn test_corruption_detection_in_dag() {
    let data = vec![0u8; 2048];
    let chunker = Chunker::new(1024);
    let mut chunks = chunker.chunk(&data);

    let dag = MerkleDAG::from_chunks(&chunks);

    // Tamper with chunk data
    chunks[0].data[0] = 255;

    // DAG verification should fail
    assert!(!dag.verify(&chunks));
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_content_config_default() {
    let config = ContentConfig::default();
    assert_eq!(config.chunk_size, 1024 * 1024);
    assert_eq!(config.enable_compression, false);
    assert!(config.validate().is_ok());
}

#[test]
fn test_content_config_custom() {
    let config = ContentConfig::new(512 * 1024);
    assert_eq!(config.chunk_size, 512 * 1024);
    assert!(config.validate().is_ok());
}

#[test]
fn test_content_config_with_compression() {
    let config = ContentConfig::with_compression(1024 * 1024);
    assert!(config.enable_compression);
}

// ============================================================================
// Large File Tests
// ============================================================================

#[test]
fn test_large_file_10mb() {
    let data = vec![42u8; 10 * 1024 * 1024]; // 10MB
    let chunker = Chunker::new(1024 * 1024); // 1MB chunks
    let chunks = chunker.chunk(&data);

    assert_eq!(chunks.len(), 10);

    // Verify all chunks
    for chunk in &chunks {
        assert!(chunk.verify());
    }

    // Build DAG
    let dag = MerkleDAG::from_chunks(&chunks);
    assert!(dag.verify(&chunks));

    // Reconstruct
    let reconstructed = chunker.reconstruct(chunks).unwrap();
    assert_eq!(data, reconstructed);
}

#[test]
fn test_large_file_50mb() {
    let data = vec![99u8; 50 * 1024 * 1024]; // 50MB
    let chunker = Chunker::new(1024 * 1024);
    let chunks = chunker.chunk(&data);

    assert_eq!(chunks.len(), 50);

    // Verify subset of chunks (to keep test fast)
    for chunk in chunks.iter().step_by(10) {
        assert!(chunk.verify());
    }

    // Build DAG
    let dag = MerkleDAG::from_chunks(&chunks);
    assert_eq!(dag.num_children(), 50);

    // Verify structure
    assert!(dag.verify(&chunks));
}

// ============================================================================
// Deduplication Tests
// ============================================================================

#[test]
fn test_deduplication_identical_content() {
    let content = b"Deduplicated content";

    let cid1 = ContentId::new(content);
    let cid2 = ContentId::new(content);

    assert_eq!(cid1, cid2);
    assert_eq!(cid1.to_string(), cid2.to_string());
}

#[test]
fn test_deduplication_identical_chunks() {
    let chunk_data = vec![0u8; 1024];

    // Create two files with identical chunks
    let mut file1 = chunk_data.clone();
    file1.extend_from_slice(&chunk_data);

    let mut file2 = chunk_data.clone();
    file2.extend_from_slice(&chunk_data);

    let chunker = Chunker::new(1024);
    let chunks1 = chunker.chunk(&file1);
    let chunks2 = chunker.chunk(&file2);

    // Both files should have identical chunk CIDs
    assert_eq!(chunks1[0].cid, chunks2[0].cid);
    assert_eq!(chunks1[1].cid, chunks2[1].cid);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_edge_case_exact_chunk_size() {
    let data = vec![0u8; 1024 * 1024]; // Exactly 1MB
    let chunker = Chunker::new(1024 * 1024);
    let chunks = chunker.chunk(&data);

    // Should create exactly 1 chunk
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].size(), 1024 * 1024);

    let reconstructed = chunker.reconstruct(chunks).unwrap();
    assert_eq!(data, reconstructed);
}

#[test]
fn test_edge_case_one_byte_over() {
    let data = vec![0u8; 1024 * 1024 + 1]; // 1MB + 1 byte
    let chunker = Chunker::new(1024 * 1024);
    let chunks = chunker.chunk(&data);

    // Should create 2 chunks
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].size(), 1024 * 1024);
    assert_eq!(chunks[1].size(), 1);

    let reconstructed = chunker.reconstruct(chunks).unwrap();
    assert_eq!(data, reconstructed);
}

#[test]
fn test_edge_case_single_byte() {
    let data = vec![42u8];
    let chunker = Chunker::new(1024 * 1024);
    let chunks = chunker.chunk(&data);

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].size(), 1);

    let reconstructed = chunker.reconstruct(chunks).unwrap();
    assert_eq!(data, reconstructed);
}

// ============================================================================
// Full Workflow Integration Test
// ============================================================================

#[test]
fn test_full_workflow_integration() {
    // Simulate a complete workflow: create content, chunk it, build DAG, verify, reconstruct

    let original_content = b"This is a test file for the Codio CDN. \
                             It contains multiple sentences and should be chunked properly. \
                             The content addressing system will generate CIDs for verification."
        .repeat(1000); // Make it larger to force chunking

    // Step 1: Generate CID for entire content
    let full_cid = ContentId::new(&original_content);
    assert!(full_cid.to_string().starts_with("Qm"));

    // Step 2: Chunk the content
    let config = ContentConfig::default();
    let chunker = Chunker::new(config.chunk_size);
    let chunks = chunker.chunk(&original_content);

    assert!(chunks.len() > 0);
    println!("Content split into {} chunks", chunks.len());

    // Step 3: Verify all chunks
    for chunk in &chunks {
        assert!(chunk.verify(), "Chunk {} failed verification", chunk.index);
    }

    // Step 4: Build Merkle DAG
    let dag = MerkleDAG::from_chunks(&chunks);
    assert!(dag.verify(&chunks), "DAG verification failed");

    println!("Root CID: {}", dag.root_cid());

    // Step 5: Reconstruct content
    let reconstructed = chunker.reconstruct(chunks).unwrap();
    assert_eq!(
        original_content, reconstructed,
        "Reconstructed content doesn't match original"
    );

    // Step 6: Verify reconstructed content matches original CID
    assert!(
        full_cid.verify(&reconstructed),
        "Reconstructed content doesn't match original CID"
    );

    println!("Full workflow integration test passed!");
}
