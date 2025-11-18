//! # Codio Content Addressing
//!
//! IPFS-style content addressing library with SHA-256 hashing, CID generation,
//! content chunking, and Merkle DAG construction.
//!
//! ## Features
//!
//! - **Content Identifiers (CID)**: IPFS CIDv0-compatible identifiers using SHA-256
//! - **Content Chunking**: Split large files into fixed-size chunks (1MB default)
//! - **Merkle DAG**: Hierarchical verification of chunked content
//! - **Cryptographic Integrity**: SHA-256 ensures content authenticity
//!
//! ## Example
//!
//! ```rust
//! use codio_content::{ContentId, Chunker, MerkleDAG};
//!
//! // Generate CID for content
//! let content = b"Hello, Codio CDN!";
//! let cid = ContentId::new(content);
//! assert!(cid.to_string().starts_with("Qm"));
//!
//! // Verify content integrity
//! assert!(cid.verify(content));
//!
//! // Chunk large content
//! let data = vec![0u8; 5 * 1024 * 1024]; // 5MB
//! let chunker = Chunker::new(1024 * 1024); // 1MB chunks
//! let chunks = chunker.chunk(&data);
//!
//! // Build Merkle DAG for verification
//! let dag = MerkleDAG::from_chunks(&chunks);
//! assert!(dag.verify(&chunks));
//! ```

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

pub mod config;
pub use config::ContentConfig;

// ============================================================================
// Content Identifier (CID)
// ============================================================================

/// Content Identifier using SHA-256 hash with IPFS CIDv0 compatibility.
///
/// CID format: `Qm<base58(sha256(content))>`
///
/// This provides:
/// - **Immutability**: Content cannot change without changing the CID
/// - **Verifiability**: Anyone can verify content matches the CID
/// - **Deduplication**: Identical content produces identical CIDs
///
/// # Example
///
/// ```rust
/// use codio_content::ContentId;
///
/// let content = b"Hello, decentralized world!";
/// let cid = ContentId::new(content);
///
/// // CID always starts with "Qm" (IPFS CIDv0)
/// assert!(cid.to_string().starts_with("Qm"));
///
/// // Verify content integrity
/// assert!(cid.verify(content));
/// assert!(!cid.verify(b"tampered content"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentId {
    /// Raw SHA-256 hash (32 bytes)
    hash: [u8; 32],
    /// Base58-encoded CID string (Qm prefix)
    multibase: String,
}

impl ContentId {
    /// Create a new ContentId from raw content bytes.
    ///
    /// This function:
    /// 1. Computes SHA-256 hash of the content
    /// 2. Encodes hash as base58
    /// 3. Prepends "Qm" for IPFS CIDv0 compatibility
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::ContentId;
    ///
    /// let cid = ContentId::new(b"test content");
    /// println!("CID: {}", cid);
    /// ```
    pub fn new(content: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash: [u8; 32] = hasher.finalize().into();

        // IPFS CIDv0 format: Qm + base58(hash)
        let multibase = format!("Qm{}", bs58::encode(&hash).into_string());

        ContentId { hash, multibase }
    }

    /// Parse a CID from its string representation.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - String doesn't start with "Qm"
    /// - Base58 decoding fails
    /// - Hash length is not 32 bytes
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::ContentId;
    ///
    /// let cid1 = ContentId::new(b"test");
    /// let cid2 = ContentId::from_string(cid1.to_string().as_str()).unwrap();
    /// assert_eq!(cid1, cid2);
    /// ```
    pub fn from_string(cid: &str) -> Result<Self> {
        if !cid.starts_with("Qm") {
            return Err(anyhow!("Invalid CID format: must start with 'Qm'"));
        }

        let hash_str = &cid[2..]; // Skip "Qm" prefix
        let hash_bytes = bs58::decode(hash_str)
            .into_vec()
            .map_err(|e| anyhow!("Invalid base58 encoding: {}", e))?;

        if hash_bytes.len() != 32 {
            return Err(anyhow!(
                "Invalid hash length: expected 32 bytes, got {}",
                hash_bytes.len()
            ));
        }

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hash_bytes);

        Ok(ContentId {
            hash,
            multibase: cid.to_string(),
        })
    }

    /// Verify that content matches this CID.
    ///
    /// Computes the hash of the provided content and compares it with
    /// the stored hash. Returns true if they match.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::ContentId;
    ///
    /// let content = b"original";
    /// let cid = ContentId::new(content);
    ///
    /// assert!(cid.verify(content));
    /// assert!(!cid.verify(b"tampered"));
    /// ```
    pub fn verify(&self, content: &[u8]) -> bool {
        let computed = Self::new(content);
        self.hash == computed.hash
    }

    /// Get the raw SHA-256 hash bytes.
    ///
    /// Returns a reference to the 32-byte hash array.
    pub fn hash(&self) -> &[u8; 32] {
        &self.hash
    }

    /// Get the CID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.multibase
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.multibase)
    }
}

// ============================================================================
// Content Chunking
// ============================================================================

/// A single chunk of content with its own CID.
///
/// Chunks are created by splitting large content into fixed-size pieces.
/// Each chunk has:
/// - An index indicating its position in the original content
/// - The chunk data (up to chunk_size bytes)
/// - A CID for verifying chunk integrity
///
/// # Example
///
/// ```rust
/// use codio_content::{Chunk, ContentId};
///
/// let data = vec![1, 2, 3, 4, 5];
/// let chunk = Chunk::new(0, data.clone());
///
/// assert_eq!(chunk.index, 0);
/// assert_eq!(chunk.data, data);
/// assert!(chunk.cid.verify(&data));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Position of this chunk in the original content (0-indexed)
    pub index: u32,
    /// Raw chunk data
    pub data: Vec<u8>,
    /// Content identifier for this chunk
    pub cid: ContentId,
}

impl Chunk {
    /// Create a new chunk from data.
    ///
    /// Automatically computes the CID for the chunk data.
    pub fn new(index: u32, data: Vec<u8>) -> Self {
        let cid = ContentId::new(&data);
        Chunk { index, data, cid }
    }

    /// Verify chunk integrity.
    ///
    /// Returns true if the chunk's data matches its CID.
    pub fn verify(&self) -> bool {
        self.cid.verify(&self.data)
    }

    /// Get chunk size in bytes.
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Content chunker that splits data into fixed-size pieces.
///
/// Chunking enables:
/// - **Parallel transfer**: Download/upload chunks concurrently
/// - **Deduplication**: Identical chunks share the same CID
/// - **Partial verification**: Verify individual chunks without downloading all data
/// - **Resume support**: Re-download only missing chunks
///
/// # Example
///
/// ```rust
/// use codio_content::Chunker;
///
/// let data = vec![0u8; 5 * 1024 * 1024]; // 5MB
/// let chunker = Chunker::new(1024 * 1024); // 1MB chunks
/// let chunks = chunker.chunk(&data);
///
/// assert_eq!(chunks.len(), 5);
/// for chunk in &chunks {
///     assert!(chunk.verify());
/// }
///
/// let reconstructed = chunker.reconstruct(chunks).unwrap();
/// assert_eq!(data, reconstructed);
/// ```
#[derive(Debug, Clone)]
pub struct Chunker {
    /// Size of each chunk in bytes (last chunk may be smaller)
    chunk_size: usize,
}

impl Chunker {
    /// Create a new chunker with specified chunk size.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Size of each chunk in bytes (typically 1MB)
    ///
    /// # Panics
    ///
    /// Panics if chunk_size is 0.
    pub fn new(chunk_size: usize) -> Self {
        assert!(chunk_size > 0, "Chunk size must be greater than 0");
        Chunker { chunk_size }
    }

    /// Split data into chunks.
    ///
    /// Creates fixed-size chunks, with the last chunk potentially being smaller.
    /// Each chunk is assigned an index starting from 0.
    ///
    /// # Arguments
    ///
    /// * `data` - The content to chunk
    ///
    /// # Returns
    ///
    /// A vector of chunks, each with its own CID.
    pub fn chunk(&self, data: &[u8]) -> Vec<Chunk> {
        if data.is_empty() {
            return vec![];
        }

        data.chunks(self.chunk_size)
            .enumerate()
            .map(|(index, chunk_data)| Chunk::new(index as u32, chunk_data.to_vec()))
            .collect()
    }

    /// Reconstruct original data from chunks.
    ///
    /// Chunks must be provided in order (sorted by index).
    /// Each chunk is verified before reconstruction.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Chunks are not in sequential order
    /// - Any chunk fails verification
    /// - Chunks are missing (gaps in indices)
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::Chunker;
    ///
    /// let original = vec![1, 2, 3, 4, 5];
    /// let chunker = Chunker::new(2);
    /// let chunks = chunker.chunk(&original);
    /// let reconstructed = chunker.reconstruct(chunks).unwrap();
    /// assert_eq!(original, reconstructed);
    /// ```
    pub fn reconstruct(&self, chunks: Vec<Chunk>) -> Result<Vec<u8>> {
        if chunks.is_empty() {
            return Ok(vec![]);
        }

        // Verify chunks are in order
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.index as usize != i {
                return Err(anyhow!(
                    "Chunks out of order: expected index {}, got {}",
                    i,
                    chunk.index
                ));
            }

            // Verify chunk integrity
            if !chunk.verify() {
                return Err(anyhow!("Chunk {} failed verification", chunk.index));
            }
        }

        // Concatenate chunk data
        let mut result = Vec::new();
        for chunk in chunks {
            result.extend_from_slice(&chunk.data);
        }

        Ok(result)
    }

    /// Get the configured chunk size.
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    /// Calculate how many chunks would be created for given data size.
    pub fn num_chunks(&self, data_size: usize) -> usize {
        if data_size == 0 {
            return 0;
        }
        data_size.div_ceil(self.chunk_size)
    }
}

impl Default for Chunker {
    fn default() -> Self {
        Chunker::new(1024 * 1024) // 1MB default
    }
}

// ============================================================================
// Merkle DAG
// ============================================================================

/// Merkle Directed Acyclic Graph for hierarchical content verification.
///
/// A Merkle DAG enables:
/// - **Hierarchical hashing**: Root CID computed from child CIDs
/// - **Efficient verification**: Verify entire structure with one root hash
/// - **Tamper detection**: Any change to any chunk invalidates the root
/// - **Selective verification**: Verify individual branches
///
/// The structure is:
/// ```text
///       Root CID
///      /   |   \
///   Chunk1 Chunk2 Chunk3
/// ```
///
/// # Example
///
/// ```rust
/// use codio_content::{Chunker, MerkleDAG};
///
/// let data = vec![0u8; 3 * 1024 * 1024]; // 3MB
/// let chunker = Chunker::new(1024 * 1024);
/// let chunks = chunker.chunk(&data);
///
/// let dag = MerkleDAG::from_chunks(&chunks);
/// assert!(dag.verify(&chunks));
///
/// // Root CID represents the entire content
/// println!("Root CID: {}", dag.root_cid());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleDAG {
    /// Root CID computed from all child CIDs
    root: ContentId,
    /// CIDs of all child chunks
    children: Vec<ContentId>,
}

impl MerkleDAG {
    /// Create a Merkle DAG from chunks.
    ///
    /// The root CID is computed by:
    /// 1. Concatenating all child CIDs
    /// 2. Hashing the concatenated CIDs
    /// 3. Creating a CID from the hash
    ///
    /// This ensures the root CID uniquely identifies the entire content
    /// and its structure.
    ///
    /// # Arguments
    ///
    /// * `chunks` - The chunks to build the DAG from
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::{Chunker, MerkleDAG};
    ///
    /// let chunker = Chunker::new(1024);
    /// let chunks = chunker.chunk(b"test data");
    /// let dag = MerkleDAG::from_chunks(&chunks);
    /// ```
    pub fn from_chunks(chunks: &[Chunk]) -> Self {
        let children: Vec<ContentId> = chunks.iter().map(|c| c.cid.clone()).collect();

        // Compute root hash from all child CIDs
        let mut hasher = Sha256::new();
        for child_cid in &children {
            hasher.update(child_cid.hash());
        }
        let root_hash: [u8; 32] = hasher.finalize().into();
        let root_multibase = format!("Qm{}", bs58::encode(&root_hash).into_string());
        let root = ContentId {
            hash: root_hash,
            multibase: root_multibase,
        };

        MerkleDAG { root, children }
    }

    /// Verify all chunks against the Merkle DAG.
    ///
    /// This ensures:
    /// - Number of chunks matches number of children
    /// - Each chunk's CID matches the corresponding child CID
    /// - Chunk order is preserved
    ///
    /// # Returns
    ///
    /// `true` if all chunks are valid and in correct order, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::{Chunker, MerkleDAG};
    ///
    /// let chunker = Chunker::new(1024);
    /// let chunks = chunker.chunk(b"test");
    /// let dag = MerkleDAG::from_chunks(&chunks);
    ///
    /// assert!(dag.verify(&chunks));
    /// ```
    pub fn verify(&self, chunks: &[Chunk]) -> bool {
        if chunks.len() != self.children.len() {
            return false;
        }

        // Verify each chunk's CID matches the DAG
        for (chunk, expected_cid) in chunks.iter().zip(self.children.iter()) {
            if &chunk.cid != expected_cid {
                return false;
            }
            // Also verify chunk integrity
            if !chunk.verify() {
                return false;
            }
        }

        // Verify root hash
        let mut hasher = Sha256::new();
        for child_cid in &self.children {
            hasher.update(child_cid.hash());
        }
        let computed_hash: [u8; 32] = hasher.finalize().into();

        computed_hash == self.root.hash
    }

    /// Get the root CID of the Merkle DAG.
    ///
    /// The root CID uniquely identifies the entire content and its structure.
    pub fn root_cid(&self) -> &ContentId {
        &self.root
    }

    /// Get all child CIDs.
    ///
    /// Returns a reference to the vector of child CIDs in order.
    pub fn children(&self) -> &[ContentId] {
        &self.children
    }

    /// Get the number of children (chunks) in the DAG.
    pub fn num_children(&self) -> usize {
        self.children.len()
    }

    /// Check if a specific CID is present in the children.
    pub fn contains_child(&self, cid: &ContentId) -> bool {
        self.children.iter().any(|c| c == cid)
    }

    /// Verify a single chunk belongs to this DAG.
    ///
    /// Returns true if:
    /// - The chunk's index is valid for this DAG
    /// - The chunk's CID matches the expected CID at that index
    /// - The chunk passes integrity verification
    pub fn verify_chunk(&self, chunk: &Chunk) -> bool {
        let index = chunk.index as usize;
        if index >= self.children.len() {
            return false;
        }

        chunk.cid == self.children[index] && chunk.verify()
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during content addressing operations.
#[derive(Debug, thiserror::Error)]
pub enum ContentError {
    /// Invalid CID format
    #[error("Invalid CID format: {0}")]
    InvalidCid(String),

    /// Chunk verification failed
    #[error("Chunk verification failed: chunk {0}")]
    ChunkVerificationFailed(u32),

    /// Chunks are not in sequential order
    #[error("Chunks out of order: expected {expected}, got {actual}")]
    ChunksOutOfOrder { expected: u32, actual: u32 },

    /// Merkle DAG verification failed
    #[error("Merkle DAG verification failed")]
    DagVerificationFailed,

    /// Invalid chunk size
    #[error("Invalid chunk size: {0}")]
    InvalidChunkSize(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Hash arbitrary data with SHA-256 and return raw bytes.
///
/// This is a utility function for cases where you need just the hash
/// without creating a full CID.
pub fn hash_content(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Calculate CID for content without creating a ContentId struct.
///
/// Useful for quick CID lookups or comparisons.
pub fn calculate_cid(data: &[u8]) -> String {
    let hash = hash_content(data);
    format!("Qm{}", bs58::encode(&hash).into_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ContentId Tests
    // ========================================================================

    #[test]
    fn test_cid_creation() {
        let content = b"Hello, Codio CDN!";
        let cid = ContentId::new(content);

        assert!(cid.to_string().starts_with("Qm"));
        assert_eq!(cid.hash().len(), 32);
    }

    #[test]
    fn test_cid_verification() {
        let content = b"Test content";
        let cid = ContentId::new(content);

        assert!(cid.verify(content));
        assert!(!cid.verify(b"Different content"));
    }

    #[test]
    fn test_cid_roundtrip() {
        let content = b"Roundtrip test";
        let cid1 = ContentId::new(content);
        let cid2 = ContentId::from_string(&cid1.to_string()).unwrap();

        assert_eq!(cid1, cid2);
        assert_eq!(cid1.hash(), cid2.hash());
    }

    #[test]
    fn test_same_content_same_cid() {
        let cid1 = ContentId::new(b"Same content");
        let cid2 = ContentId::new(b"Same content");

        assert_eq!(cid1, cid2);
    }

    #[test]
    fn test_different_content_different_cid() {
        let cid1 = ContentId::new(b"Content A");
        let cid2 = ContentId::new(b"Content B");

        assert_ne!(cid1, cid2);
    }

    #[test]
    fn test_cid_from_string_invalid_prefix() {
        let result = ContentId::from_string("Zm123456");
        assert!(result.is_err());
    }

    #[test]
    fn test_cid_from_string_invalid_base58() {
        let result = ContentId::from_string("Qm!!!invalid!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_cid_display() {
        let cid = ContentId::new(b"test");
        let display = format!("{}", cid);
        assert!(display.starts_with("Qm"));
    }

    // Chunker Tests
    // ========================================================================

    #[test]
    fn test_chunking_basic() {
        let data = vec![0u8; 5 * 1024 * 1024]; // 5MB
        let chunker = Chunker::new(1024 * 1024); // 1MB chunks
        let chunks = chunker.chunk(&data);

        assert_eq!(chunks.len(), 5);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index as usize, i);
            assert_eq!(chunk.size(), 1024 * 1024);
            assert!(chunk.verify());
        }
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
    fn test_chunking_empty_data() {
        let data = vec![];
        let chunker = Chunker::new(1024);
        let chunks = chunker.chunk(&data);

        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_chunking_small_data() {
        let data = vec![1, 2, 3, 4, 5];
        let chunker = Chunker::new(1024);
        let chunks = chunker.chunk(&data);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].size(), 5);
    }

    #[test]
    fn test_reconstruct_basic() {
        let original = vec![0u8; 3 * 1024]; // 3KB
        let chunker = Chunker::new(1024);
        let chunks = chunker.chunk(&original);
        let reconstructed = chunker.reconstruct(chunks).unwrap();

        assert_eq!(original, reconstructed);
    }

    #[test]
    fn test_reconstruct_empty() {
        let chunker = Chunker::new(1024);
        let chunks = vec![];
        let reconstructed = chunker.reconstruct(chunks).unwrap();

        assert_eq!(reconstructed, Vec::<u8>::new());
    }

    #[test]
    fn test_reconstruct_out_of_order() {
        let chunker = Chunker::new(1024);
        let mut chunks = chunker.chunk(&vec![0u8; 2048]);

        // Swap chunks
        chunks.swap(0, 1);

        let result = chunker.reconstruct(chunks);
        assert!(result.is_err());
    }

    #[test]
    fn test_reconstruct_tampered_chunk() {
        let chunker = Chunker::new(1024);
        let mut chunks = chunker.chunk(&vec![0u8; 2048]);

        // Tamper with chunk data
        chunks[0].data[0] = 255;

        let result = chunker.reconstruct(chunks);
        assert!(result.is_err());
    }

    #[test]
    fn test_chunker_default() {
        let chunker = Chunker::default();
        assert_eq!(chunker.chunk_size(), 1024 * 1024);
    }

    #[test]
    fn test_num_chunks() {
        let chunker = Chunker::new(1024);

        assert_eq!(chunker.num_chunks(0), 0);
        assert_eq!(chunker.num_chunks(512), 1);
        assert_eq!(chunker.num_chunks(1024), 1);
        assert_eq!(chunker.num_chunks(1025), 2);
        assert_eq!(chunker.num_chunks(2048), 2);
    }

    #[test]
    #[should_panic(expected = "Chunk size must be greater than 0")]
    fn test_chunker_zero_size() {
        Chunker::new(0);
    }

    // Chunk Tests
    // ========================================================================

    #[test]
    fn test_chunk_creation() {
        let data = vec![1, 2, 3, 4, 5];
        let chunk = Chunk::new(0, data.clone());

        assert_eq!(chunk.index, 0);
        assert_eq!(chunk.data, data);
        assert!(chunk.verify());
    }

    #[test]
    fn test_chunk_verification() {
        let data = vec![1, 2, 3];
        let mut chunk = Chunk::new(0, data);

        assert!(chunk.verify());

        // Tamper with data
        chunk.data[0] = 99;
        assert!(!chunk.verify());
    }

    // MerkleDAG Tests
    // ========================================================================

    #[test]
    fn test_merkle_dag_creation() {
        let data = vec![0u8; 3 * 1024 * 1024];
        let chunker = Chunker::new(1024 * 1024);
        let chunks = chunker.chunk(&data);

        let dag = MerkleDAG::from_chunks(&chunks);

        assert_eq!(dag.num_children(), 3);
        assert!(dag.root_cid().to_string().starts_with("Qm"));
    }

    #[test]
    fn test_merkle_dag_verification() {
        let data = vec![0u8; 3 * 1024 * 1024];
        let chunker = Chunker::new(1024 * 1024);
        let chunks = chunker.chunk(&data);

        let dag = MerkleDAG::from_chunks(&chunks);
        assert!(dag.verify(&chunks));
    }

    #[test]
    fn test_merkle_dag_verification_fails_wrong_chunks() {
        let data1 = vec![0u8; 2 * 1024];
        let data2 = vec![1u8; 2 * 1024];

        let chunker = Chunker::new(1024);
        let chunks1 = chunker.chunk(&data1);
        let chunks2 = chunker.chunk(&data2);

        let dag = MerkleDAG::from_chunks(&chunks1);
        assert!(!dag.verify(&chunks2));
    }

    #[test]
    fn test_merkle_dag_verification_fails_wrong_count() {
        let chunker = Chunker::new(1024);
        let chunks = chunker.chunk(&vec![0u8; 3 * 1024]);

        let dag = MerkleDAG::from_chunks(&chunks);
        let partial_chunks = vec![chunks[0].clone(), chunks[1].clone()];

        assert!(!dag.verify(&partial_chunks));
    }

    #[test]
    fn test_merkle_dag_contains_child() {
        let chunker = Chunker::new(1024);
        let chunks = chunker.chunk(&vec![0u8; 2048]);
        let dag = MerkleDAG::from_chunks(&chunks);

        for chunk in &chunks {
            assert!(dag.contains_child(&chunk.cid));
        }

        let other_cid = ContentId::new(b"not in dag");
        assert!(!dag.contains_child(&other_cid));
    }

    #[test]
    fn test_merkle_dag_verify_chunk() {
        let chunker = Chunker::new(1024);
        let chunks = chunker.chunk(&vec![0u8; 2048]);
        let dag = MerkleDAG::from_chunks(&chunks);

        for chunk in &chunks {
            assert!(dag.verify_chunk(chunk));
        }
    }

    #[test]
    fn test_merkle_dag_verify_chunk_wrong_index() {
        let chunker = Chunker::new(1024);
        let chunks = chunker.chunk(&vec![0u8; 2048]);
        let dag = MerkleDAG::from_chunks(&chunks);

        let mut wrong_chunk = chunks[0].clone();
        wrong_chunk.index = 99;

        assert!(!dag.verify_chunk(&wrong_chunk));
    }

    #[test]
    fn test_merkle_dag_same_content_same_root() {
        let chunker = Chunker::new(1024);
        let chunks1 = chunker.chunk(&vec![0u8; 2048]);
        let chunks2 = chunker.chunk(&vec![0u8; 2048]);

        let dag1 = MerkleDAG::from_chunks(&chunks1);
        let dag2 = MerkleDAG::from_chunks(&chunks2);

        assert_eq!(dag1.root_cid(), dag2.root_cid());
    }

    #[test]
    fn test_merkle_dag_different_content_different_root() {
        let chunker = Chunker::new(1024);
        let chunks1 = chunker.chunk(&vec![0u8; 2048]);
        let chunks2 = chunker.chunk(&vec![1u8; 2048]);

        let dag1 = MerkleDAG::from_chunks(&chunks1);
        let dag2 = MerkleDAG::from_chunks(&chunks2);

        assert_ne!(dag1.root_cid(), dag2.root_cid());
    }

    #[test]
    fn test_merkle_dag_empty_chunks() {
        let chunks: Vec<Chunk> = vec![];
        let dag = MerkleDAG::from_chunks(&chunks);

        assert_eq!(dag.num_children(), 0);
        assert!(dag.verify(&chunks));
    }

    // Utility Functions Tests
    // ========================================================================

    #[test]
    fn test_hash_content() {
        let data = b"test";
        let hash = hash_content(data);

        assert_eq!(hash.len(), 32);

        // Same content produces same hash
        let hash2 = hash_content(data);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_calculate_cid() {
        let data = b"test";
        let cid_str = calculate_cid(data);

        assert!(cid_str.starts_with("Qm"));

        // Should match ContentId::new
        let cid = ContentId::new(data);
        assert_eq!(cid_str, cid.to_string());
    }

    // Integration Tests
    // ========================================================================

    #[test]
    fn test_full_workflow_small_file() {
        // Simulate a small file
        let content = b"Hello, Codio CDN! This is a test file.";

        // Generate CID
        let cid = ContentId::new(content);
        assert!(cid.verify(content));

        // Chunk the content
        let chunker = Chunker::new(16); // Small chunks for testing
        let chunks = chunker.chunk(content);
        assert!(chunks.len() > 1);

        // Build Merkle DAG
        let dag = MerkleDAG::from_chunks(&chunks);
        assert!(dag.verify(&chunks));

        // Reconstruct
        let reconstructed = chunker.reconstruct(chunks).unwrap();
        assert_eq!(content.to_vec(), reconstructed);
    }

    #[test]
    fn test_full_workflow_large_file() {
        // Simulate a large file (10MB)
        let content = vec![42u8; 10 * 1024 * 1024];

        // Chunk the content
        let chunker = Chunker::new(1024 * 1024); // 1MB chunks
        let chunks = chunker.chunk(&content);
        assert_eq!(chunks.len(), 10);

        // Verify all chunks
        for chunk in &chunks {
            assert!(chunk.verify());
        }

        // Build Merkle DAG
        let dag = MerkleDAG::from_chunks(&chunks);
        assert!(dag.verify(&chunks));

        // Verify individual chunks
        for chunk in &chunks {
            assert!(dag.verify_chunk(chunk));
        }

        // Reconstruct
        let reconstructed = chunker.reconstruct(chunks).unwrap();
        assert_eq!(content, reconstructed);
    }

    #[test]
    fn test_corruption_detection() {
        let content = b"Original content";
        let cid = ContentId::new(content);

        let tampered = b"Tampered content";
        assert!(!cid.verify(tampered));
    }

    #[test]
    fn test_deduplication() {
        // Same content should produce same CID
        let content = b"Deduplicated content";

        let cid1 = ContentId::new(content);
        let cid2 = ContentId::new(content);

        assert_eq!(cid1, cid2);
        assert_eq!(cid1.to_string(), cid2.to_string());
    }

    #[test]
    fn test_chunk_level_deduplication() {
        // Create two files with identical chunks
        let chunk_data = vec![0u8; 1024];
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
}
