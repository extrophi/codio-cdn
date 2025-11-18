//! # Codio Content Addressing Library
//!
//! This library provides IPFS-compatible content addressing functionality for the Codio
//! decentralized CDN. It implements CID (Content Identifier) generation, content chunking,
//! and Merkle DAG structures for verifiable, immutable content distribution.
//!
//! ## Core Concepts
//!
//! ### Content Identifiers (CIDs)
//!
//! CIDs are cryptographic hashes that uniquely identify content. This implementation uses:
//! - **SHA-256** hashing for security
//! - **Base58** encoding for human-readable strings
//! - **IPFS CIDv0** format with "Qm" prefix for compatibility
//!
//! ### Content Chunking
//!
//! Large files are split into fixed-size chunks (default 1MB) for efficient transfer:
//! - Each chunk gets its own CID
//! - Chunks can be verified independently
//! - Enables parallel downloads and deduplication
//!
//! ### Merkle DAG
//!
//! A Directed Acyclic Graph structure that:
//! - Links chunks together
//! - Provides a root CID representing the entire content
//! - Enables efficient verification of content integrity
//!
//! ## Example Usage
//!
//! ```rust
//! use codio_content::{ContentId, Chunker, MerkleDAG};
//!
//! // Generate a CID for some content
//! let content = b"Hello, Codio CDN!";
//! let cid = ContentId::new(content);
//! println!("CID: {}", cid.to_string());
//!
//! // Verify content matches CID
//! assert!(cid.verify(content));
//!
//! // Chunk large content
//! let large_content = vec![0u8; 5 * 1024 * 1024]; // 5MB
//! let chunker = Chunker::new(1024 * 1024); // 1MB chunks
//! let chunks = chunker.chunk(&large_content);
//!
//! // Create Merkle DAG from chunks
//! let dag = MerkleDAG::from_chunks(&chunks);
//! println!("Root CID: {}", dag.root_cid().to_string());
//! ```

pub mod config;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// IPFS CIDv0 prefix bytes
/// Represents: multibase (base58btc) + multicodec (dag-pb) + multihash (sha2-256)
const IPFS_CID_PREFIX: &[u8] = &[0x12, 0x20]; // 0x12 = sha2-256, 0x20 = 32 bytes

/// Default chunk size (1 MB)
pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// Maximum chunk size (10 MB)
pub const MAX_CHUNK_SIZE: usize = 10 * 1024 * 1024;

/// Minimum chunk size (1 KB)
pub const MIN_CHUNK_SIZE: usize = 1024;

// ============================================================================
// ContentId - IPFS-Compatible Content Identifier
// ============================================================================

/// A content identifier (CID) that uniquely represents a piece of data.
///
/// This implementation follows the IPFS CIDv0 specification:
/// - Uses SHA-256 hashing
/// - Base58-encoded with "Qm" prefix
/// - Immutable and deterministic
///
/// # Example
///
/// ```rust
/// use codio_content::ContentId;
///
/// let content = b"Hello, world!";
/// let cid = ContentId::new(content);
///
/// // CID is a Qm... string
/// assert!(cid.to_string().starts_with("Qm"));
///
/// // Verify content matches CID
/// assert!(cid.verify(content));
/// ```
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct ContentId {
    /// Raw SHA-256 hash of the content
    hash: [u8; 32],

    /// Base58-encoded CID string (cached for performance)
    #[serde(skip)]
    multibase: Option<String>,
}

impl PartialEq for ContentId {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for ContentId {}

impl ContentId {
    /// Creates a new ContentId from raw content bytes.
    ///
    /// This computes the SHA-256 hash and generates an IPFS CIDv0-compatible identifier.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::ContentId;
    ///
    /// let cid = ContentId::new(b"Hello, Codio!");
    /// assert!(cid.to_string().starts_with("Qm"));
    /// ```
    pub fn new(content: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash: [u8; 32] = hasher.finalize().into();

        ContentId {
            hash,
            multibase: None,
        }
    }

    /// Creates a ContentId from a CID string (e.g., "QmXXX...").
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The CID doesn't start with "Qm"
    /// - The Base58 decoding fails
    /// - The decoded hash is not 34 bytes (2-byte prefix + 32-byte hash)
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::ContentId;
    ///
    /// let original = ContentId::new(b"test");
    /// let cid_string = original.to_string();
    /// let restored = ContentId::from_string(&cid_string).unwrap();
    /// assert_eq!(original, restored);
    /// ```
    pub fn from_string(cid: &str) -> Result<Self> {
        if !cid.starts_with("Qm") {
            return Err(anyhow!("Invalid CID: must start with 'Qm'"));
        }

        let decoded = bs58::decode(cid)
            .into_vec()
            .context("Failed to decode base58 CID")?;

        if decoded.len() != 34 {
            return Err(anyhow!(
                "Invalid CID length: expected 34 bytes, got {}",
                decoded.len()
            ));
        }

        // Verify prefix
        if &decoded[0..2] != IPFS_CID_PREFIX {
            return Err(anyhow!("Invalid CID prefix"));
        }

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&decoded[2..34]);

        Ok(ContentId {
            hash,
            multibase: Some(cid.to_string()),
        })
    }

    /// Converts the ContentId to a CID string (e.g., "QmXXX...").
    ///
    /// This is cached after the first call for performance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::ContentId;
    ///
    /// let cid = ContentId::new(b"test");
    /// println!("CID: {}", cid);
    /// ```
    fn encode_cid(&self) -> String {
        if let Some(ref cached) = self.multibase {
            return cached.clone();
        }

        // Build the CID bytes: prefix + hash
        let mut cid_bytes = Vec::with_capacity(34);
        cid_bytes.extend_from_slice(IPFS_CID_PREFIX);
        cid_bytes.extend_from_slice(&self.hash);

        // Encode to base58
        bs58::encode(cid_bytes).into_string()
    }

    /// Verifies that the given content matches this CID.
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
        let computed = ContentId::new(content);
        computed.hash == self.hash
    }

    /// Returns the raw SHA-256 hash bytes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::ContentId;
    ///
    /// let cid = ContentId::new(b"test");
    /// let hash = cid.hash();
    /// assert_eq!(hash.len(), 32);
    /// ```
    pub fn hash(&self) -> &[u8; 32] {
        &self.hash
    }

    /// Returns the hash as a hexadecimal string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::ContentId;
    ///
    /// let cid = ContentId::new(b"test");
    /// let hex = cid.hash_hex();
    /// assert_eq!(hex.len(), 64); // 32 bytes = 64 hex chars
    /// ```
    pub fn hash_hex(&self) -> String {
        hex::encode(self.hash)
    }

    /// Creates a ContentId from a raw SHA-256 hash.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::ContentId;
    ///
    /// let hash = [0u8; 32];
    /// let cid = ContentId::from_hash(hash);
    /// ```
    pub fn from_hash(hash: [u8; 32]) -> Self {
        ContentId {
            hash,
            multibase: None,
        }
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.encode_cid())
    }
}

// ============================================================================
// Chunk - A piece of content with its CID
// ============================================================================

/// A chunk represents a fixed-size piece of content with its own CID.
///
/// Chunks enable:
/// - Efficient parallel downloads
/// - Deduplication of identical content
/// - Independent verification
/// - Resume capability for interrupted transfers
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
/// assert!(chunk.verify());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Zero-based index of this chunk in the original content
    pub index: u32,

    /// The actual data in this chunk
    pub data: Vec<u8>,

    /// Content identifier for this chunk
    pub cid: ContentId,
}

impl Chunk {
    /// Creates a new chunk with the given index and data.
    ///
    /// The CID is automatically computed from the data.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::Chunk;
    ///
    /// let chunk = Chunk::new(0, vec![1, 2, 3]);
    /// assert_eq!(chunk.index, 0);
    /// assert!(chunk.verify());
    /// ```
    pub fn new(index: u32, data: Vec<u8>) -> Self {
        let cid = ContentId::new(&data);
        Chunk { index, data, cid }
    }

    /// Verifies that the chunk's data matches its CID.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::Chunk;
    ///
    /// let mut chunk = Chunk::new(0, vec![1, 2, 3]);
    /// assert!(chunk.verify());
    ///
    /// // Tamper with data
    /// chunk.data[0] = 99;
    /// assert!(!chunk.verify());
    /// ```
    pub fn verify(&self) -> bool {
        self.cid.verify(&self.data)
    }

    /// Returns the size of this chunk in bytes.
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Creates a chunk from existing data and CID (for deserialization).
    ///
    /// # Safety
    ///
    /// This does not verify that the CID matches the data. Use `verify()` to check.
    pub fn from_parts(index: u32, data: Vec<u8>, cid: ContentId) -> Self {
        Chunk { index, data, cid }
    }
}

// ============================================================================
// Chunker - Splits content into chunks
// ============================================================================

/// A chunker splits content into fixed-size chunks.
///
/// # Example
///
/// ```rust
/// use codio_content::{Chunker, DEFAULT_CHUNK_SIZE};
///
/// let data = vec![0u8; 5 * 1024 * 1024]; // 5MB
/// let chunker = Chunker::new(DEFAULT_CHUNK_SIZE);
/// let chunks = chunker.chunk(&data);
///
/// assert_eq!(chunks.len(), 5);
/// for chunk in &chunks {
///     assert!(chunk.verify());
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Chunker {
    /// Size of each chunk in bytes
    chunk_size: usize,
}

impl Chunker {
    /// Creates a new chunker with the specified chunk size.
    ///
    /// # Panics
    ///
    /// Panics if chunk_size is less than MIN_CHUNK_SIZE or greater than MAX_CHUNK_SIZE.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::Chunker;
    ///
    /// let chunker = Chunker::new(1024 * 1024); // 1MB chunks
    /// ```
    pub fn new(chunk_size: usize) -> Self {
        assert!(
            chunk_size >= MIN_CHUNK_SIZE,
            "Chunk size must be at least {} bytes",
            MIN_CHUNK_SIZE
        );
        assert!(
            chunk_size <= MAX_CHUNK_SIZE,
            "Chunk size must be at most {} bytes",
            MAX_CHUNK_SIZE
        );

        Chunker { chunk_size }
    }


    /// Splits data into chunks.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::Chunker;
    ///
    /// let data = vec![0u8; 2500]; // 2.5KB
    /// let chunker = Chunker::new(1024); // 1KB chunks
    /// let chunks = chunker.chunk(&data);
    ///
    /// assert_eq!(chunks.len(), 3); // 1KB + 1KB + 0.5KB
    /// ```
    pub fn chunk(&self, data: &[u8]) -> Vec<Chunk> {
        if data.is_empty() {
            return vec![];
        }

        let mut chunks = Vec::new();
        let mut offset = 0;
        let mut index = 0;

        while offset < data.len() {
            let end = std::cmp::min(offset + self.chunk_size, data.len());
            let chunk_data = data[offset..end].to_vec();
            chunks.push(Chunk::new(index, chunk_data));

            offset = end;
            index += 1;
        }

        chunks
    }

    /// Reconstructs the original data from chunks.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Chunks are not in sequential order
    /// - Any chunk fails verification
    /// - Chunk indices are not contiguous
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::Chunker;
    ///
    /// let original = vec![0u8; 5000];
    /// let chunker = Chunker::new(1024);
    /// let chunks = chunker.chunk(&original);
    ///
    /// let reconstructed = chunker.reconstruct(chunks).unwrap();
    /// assert_eq!(original, reconstructed);
    /// ```
    pub fn reconstruct(&self, chunks: Vec<Chunk>) -> Result<Vec<u8>> {
        if chunks.is_empty() {
            return Ok(vec![]);
        }

        // Verify all chunks first
        for chunk in &chunks {
            if !chunk.verify() {
                return Err(anyhow!("Chunk {} failed verification", chunk.index));
            }
        }

        // Sort chunks by index
        let mut sorted_chunks = chunks;
        sorted_chunks.sort_by_key(|c| c.index);

        // Verify sequential indices
        for (i, chunk) in sorted_chunks.iter().enumerate() {
            if chunk.index != i as u32 {
                return Err(anyhow!(
                    "Non-contiguous chunk indices: expected {}, got {}",
                    i,
                    chunk.index
                ));
            }
        }

        // Reconstruct data
        let total_size: usize = sorted_chunks.iter().map(|c| c.data.len()).sum();
        let mut data = Vec::with_capacity(total_size);

        for chunk in sorted_chunks {
            data.extend_from_slice(&chunk.data);
        }

        Ok(data)
    }

    /// Returns the chunk size in bytes.
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    /// Calculates the number of chunks needed for data of the given size.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::Chunker;
    ///
    /// let chunker = Chunker::new(1024);
    /// assert_eq!(chunker.num_chunks(2500), 3);
    /// assert_eq!(chunker.num_chunks(1024), 1);
    /// assert_eq!(chunker.num_chunks(0), 0);
    /// ```
    pub fn num_chunks(&self, data_size: usize) -> usize {
        if data_size == 0 {
            return 0;
        }
        data_size.div_ceil(self.chunk_size)
    }
}

impl Default for Chunker {
    fn default() -> Self {
        Chunker::new(DEFAULT_CHUNK_SIZE)
    }
}

// ============================================================================
// MerkleDAG - Merkle Directed Acyclic Graph
// ============================================================================

/// A Merkle DAG represents the relationship between content chunks.
///
/// The DAG structure:
/// - Root node contains the CID of all child CIDs concatenated
/// - Child nodes are individual chunk CIDs
/// - Provides cryptographic proof of content integrity
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
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleDAG {
    /// Root CID representing the entire content
    root: ContentId,

    /// Child CIDs (one per chunk)
    children: Vec<ContentId>,
}

impl MerkleDAG {
    /// Creates a Merkle DAG from a set of chunks.
    ///
    /// The root CID is computed by hashing the concatenated child CIDs.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::{Chunker, MerkleDAG};
    ///
    /// let data = vec![0u8; 2048];
    /// let chunker = Chunker::new(1024);
    /// let chunks = chunker.chunk(&data);
    ///
    /// let dag = MerkleDAG::from_chunks(&chunks);
    /// assert_eq!(dag.num_children(), 2);
    /// ```
    pub fn from_chunks(chunks: &[Chunk]) -> Self {
        let children: Vec<ContentId> = chunks.iter().map(|c| c.cid.clone()).collect();

        // Compute root by hashing all child CIDs
        let root = Self::compute_root(&children);

        MerkleDAG { root, children }
    }

    /// Computes the root CID from child CIDs.
    fn compute_root(children: &[ContentId]) -> ContentId {
        if children.is_empty() {
            return ContentId::new(&[]);
        }

        // Concatenate all child hashes
        let mut combined = Vec::new();
        for child in children {
            combined.extend_from_slice(child.hash());
        }

        ContentId::new(&combined)
    }

    /// Verifies that the given chunks match this DAG structure.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::{Chunker, MerkleDAG};
    ///
    /// let data = vec![0u8; 2048];
    /// let chunker = Chunker::new(1024);
    /// let chunks = chunker.chunk(&data);
    ///
    /// let dag = MerkleDAG::from_chunks(&chunks);
    /// assert!(dag.verify(&chunks));
    /// ```
    pub fn verify(&self, chunks: &[Chunk]) -> bool {
        // Verify number of chunks matches
        if chunks.len() != self.children.len() {
            return false;
        }

        // Verify each chunk CID matches
        for (i, chunk) in chunks.iter().enumerate() {
            if !chunk.verify() {
                return false;
            }
            if chunk.cid != self.children[i] {
                return false;
            }
        }

        // Verify root CID
        let computed_root = Self::compute_root(&self.children);
        computed_root == self.root
    }

    /// Returns the root CID.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::{Chunker, MerkleDAG};
    ///
    /// let data = vec![0u8; 1024];
    /// let chunker = Chunker::new(1024);
    /// let chunks = chunker.chunk(&data);
    ///
    /// let dag = MerkleDAG::from_chunks(&chunks);
    /// println!("Root CID: {}", dag.root_cid());
    /// ```
    pub fn root_cid(&self) -> &ContentId {
        &self.root
    }

    /// Returns the child CIDs.
    pub fn children(&self) -> &[ContentId] {
        &self.children
    }

    /// Returns the number of child nodes.
    pub fn num_children(&self) -> usize {
        self.children.len()
    }

    /// Verifies the DAG structure integrity (root matches children).
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::{Chunker, MerkleDAG};
    ///
    /// let data = vec![0u8; 1024];
    /// let chunker = Chunker::new(1024);
    /// let chunks = chunker.chunk(&data);
    ///
    /// let dag = MerkleDAG::from_chunks(&chunks);
    /// assert!(dag.verify_structure());
    /// ```
    pub fn verify_structure(&self) -> bool {
        let computed_root = Self::compute_root(&self.children);
        computed_root == self.root
    }

    /// Creates a DAG from a root CID and child CIDs (for deserialization).
    ///
    /// # Safety
    ///
    /// This does not verify the structure. Use `verify_structure()` to check.
    pub fn from_parts(root: ContentId, children: Vec<ContentId>) -> Self {
        MerkleDAG { root, children }
    }
}

// ============================================================================
// Content - High-level content management
// ============================================================================

/// Represents a complete piece of content with its chunks and DAG.
///
/// This is the primary interface for working with content in the Codio CDN.
///
/// # Example
///
/// ```rust
/// use codio_content::Content;
///
/// let data = vec![0u8; 5 * 1024 * 1024]; // 5MB
/// let content = Content::new(data, 1024 * 1024);
///
/// println!("Root CID: {}", content.root_cid());
/// println!("Chunks: {}", content.num_chunks());
///
/// // Verify integrity
/// assert!(content.verify());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// Merkle DAG structure
    dag: MerkleDAG,

    /// Content chunks
    chunks: Vec<Chunk>,

    /// Total content size in bytes
    total_size: usize,
}

impl Content {
    /// Creates a new Content from raw data.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_content::{Content, DEFAULT_CHUNK_SIZE};
    ///
    /// let data = vec![0u8; 2 * 1024 * 1024]; // 2MB
    /// let content = Content::new(data, DEFAULT_CHUNK_SIZE);
    /// ```
    pub fn new(data: Vec<u8>, chunk_size: usize) -> Self {
        let total_size = data.len();
        let chunker = Chunker::new(chunk_size);
        let chunks = chunker.chunk(&data);
        let dag = MerkleDAG::from_chunks(&chunks);

        Content {
            dag,
            chunks,
            total_size,
        }
    }

    /// Creates Content from existing chunks and DAG.
    pub fn from_parts(dag: MerkleDAG, chunks: Vec<Chunk>) -> Result<Self> {
        let total_size = chunks.iter().map(|c| c.size()).sum();

        // Verify structure
        if !dag.verify(&chunks) {
            return Err(anyhow!("DAG verification failed"));
        }

        Ok(Content {
            dag,
            chunks,
            total_size,
        })
    }

    /// Returns the root CID.
    pub fn root_cid(&self) -> &ContentId {
        self.dag.root_cid()
    }

    /// Returns the chunks.
    pub fn chunks(&self) -> &[Chunk] {
        &self.chunks
    }

    /// Returns the Merkle DAG.
    pub fn dag(&self) -> &MerkleDAG {
        &self.dag
    }

    /// Returns the total content size in bytes.
    pub fn size(&self) -> usize {
        self.total_size
    }

    /// Returns the number of chunks.
    pub fn num_chunks(&self) -> usize {
        self.chunks.len()
    }

    /// Verifies the integrity of all chunks and the DAG structure.
    pub fn verify(&self) -> bool {
        self.dag.verify(&self.chunks)
    }

    /// Reconstructs the original data from chunks.
    ///
    /// # Errors
    ///
    /// Returns an error if reconstruction fails or verification fails.
    pub fn reconstruct(&self) -> Result<Vec<u8>> {
        if !self.verify() {
            return Err(anyhow!("Content verification failed"));
        }

        // Determine chunk size from first chunk (or use default)
        let chunk_size = if !self.chunks.is_empty() {
            std::cmp::max(self.chunks[0].size(), DEFAULT_CHUNK_SIZE)
        } else {
            DEFAULT_CHUNK_SIZE
        };

        let chunker = Chunker::new(chunk_size);
        chunker.reconstruct(self.chunks.clone())
    }

    /// Returns a chunk by index.
    ///
    /// # Errors
    ///
    /// Returns an error if the index is out of bounds.
    pub fn get_chunk(&self, index: u32) -> Result<&Chunk> {
        self.chunks
            .iter()
            .find(|c| c.index == index)
            .ok_or_else(|| anyhow!("Chunk {} not found", index))
    }

    /// Returns all chunk CIDs.
    pub fn chunk_cids(&self) -> Vec<&ContentId> {
        self.chunks.iter().map(|c| &c.cid).collect()
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Hashes data using SHA-256 and returns the raw hash bytes.
///
/// # Example
///
/// ```rust
/// use codio_content::hash_sha256;
///
/// let hash = hash_sha256(b"test data");
/// assert_eq!(hash.len(), 32);
/// ```
pub fn hash_sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Hashes data and returns a hexadecimal string.
///
/// # Example
///
/// ```rust
/// use codio_content::hash_sha256_hex;
///
/// let hex = hash_sha256_hex(b"test data");
/// assert_eq!(hex.len(), 64); // 32 bytes = 64 hex chars
/// ```
pub fn hash_sha256_hex(data: &[u8]) -> String {
    hex::encode(hash_sha256(data))
}

/// Verifies that data matches a given SHA-256 hash.
///
/// # Example
///
/// ```rust
/// use codio_content::{hash_sha256, verify_hash};
///
/// let data = b"test";
/// let hash = hash_sha256(data);
/// assert!(verify_hash(data, &hash));
/// assert!(!verify_hash(b"wrong", &hash));
/// ```
pub fn verify_hash(data: &[u8], expected_hash: &[u8; 32]) -> bool {
    let computed = hash_sha256(data);
    &computed == expected_hash
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_id_new() {
        let cid = ContentId::new(b"Hello, Codio!");
        assert_eq!(cid.hash().len(), 32);
    }

    #[test]
    fn test_content_id_to_string() {
        let cid = ContentId::new(b"test");
        let cid_str = cid.to_string();
        assert!(cid_str.starts_with("Qm"));
        assert!(cid_str.len() > 10);
    }

    #[test]
    fn test_content_id_from_string() {
        let original = ContentId::new(b"test data");
        let cid_str = original.to_string();
        let restored = ContentId::from_string(&cid_str).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_content_id_verify() {
        let content = b"original content";
        let cid = ContentId::new(content);
        assert!(cid.verify(content));
        assert!(!cid.verify(b"tampered"));
    }

    #[test]
    fn test_chunk_new() {
        let data = vec![1, 2, 3, 4, 5];
        let chunk = Chunk::new(0, data.clone());
        assert_eq!(chunk.index, 0);
        assert_eq!(chunk.data, data);
        assert!(chunk.verify());
    }

    #[test]
    fn test_chunk_verify_tampered() {
        let mut chunk = Chunk::new(0, vec![1, 2, 3]);
        assert!(chunk.verify());

        chunk.data[0] = 99;
        assert!(!chunk.verify());
    }

    #[test]
    fn test_chunker_single_chunk() {
        let data = vec![0u8; 512];
        let chunker = Chunker::new(1024);
        let chunks = chunker.chunk(&data);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].data.len(), 512);
    }

    #[test]
    fn test_chunker_multiple_chunks() {
        let data = vec![0u8; 2500];
        let chunker = Chunker::new(1024);
        let chunks = chunker.chunk(&data);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].data.len(), 1024);
        assert_eq!(chunks[1].data.len(), 1024);
        assert_eq!(chunks[2].data.len(), 452);
    }

    #[test]
    fn test_chunker_reconstruct() {
        let original = vec![0u8; 5000];
        let chunker = Chunker::new(1024);
        let chunks = chunker.chunk(&original);
        let reconstructed = chunker.reconstruct(chunks).unwrap();
        assert_eq!(original, reconstructed);
    }

    #[test]
    fn test_chunker_empty_data() {
        let chunker = Chunker::new(1024);
        let chunks = chunker.chunk(&[]);
        assert_eq!(chunks.len(), 0);
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
    fn test_merkle_dag_multiple_chunks() {
        let data = vec![0u8; 3 * 1024];
        let chunker = Chunker::new(1024);
        let chunks = chunker.chunk(&data);
        let dag = MerkleDAG::from_chunks(&chunks);
        assert_eq!(dag.num_children(), 3);
        assert!(dag.verify(&chunks));
    }

    #[test]
    fn test_merkle_dag_verify_structure() {
        let data = vec![0u8; 2048];
        let chunker = Chunker::new(1024);
        let chunks = chunker.chunk(&data);
        let dag = MerkleDAG::from_chunks(&chunks);
        assert!(dag.verify_structure());
    }

    #[test]
    fn test_content_new() {
        let data = vec![0u8; 5 * 1024 * 1024];
        let content = Content::new(data.clone(), DEFAULT_CHUNK_SIZE);
        assert_eq!(content.size(), data.len());
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
    fn test_hash_sha256() {
        let hash1 = hash_sha256(b"test");
        let hash2 = hash_sha256(b"test");
        assert_eq!(hash1, hash2);

        let hash3 = hash_sha256(b"different");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_verify_hash() {
        let data = b"test data";
        let hash = hash_sha256(data);
        assert!(verify_hash(data, &hash));
        assert!(!verify_hash(b"wrong data", &hash));
    }
}
