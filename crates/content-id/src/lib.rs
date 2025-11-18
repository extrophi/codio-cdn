// crates/content-id/src/lib.rs

use sha2::{Digest, Sha256};
use std::fmt;

/// Content Identifier (CID) using sha256 hash
/// Format: Qm<base58(sha256(content))>
/// Compatible with IPFS CIDv0
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentId {
    /// Raw sha256 hash bytes
    hash: [u8; 32],
    /// Base58-encoded string (Qm prefix)
    multibase: String,
}

impl ContentId {
    /// Create CID from content bytes
    pub fn new(content: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash: [u8; 32] = hasher.finalize().into();

        // IPFS CIDv0 format: Qm + base58(hash)
        let multibase = format!("Qm{}", bs58::encode(&hash).into_string());

        ContentId { hash, multibase }
    }

    /// Verify content matches this CID
    pub fn verify(&self, content: &[u8]) -> bool {
        let computed = Self::new(content);
        self.hash == computed.hash
    }

    /// Get multibase string representation
    pub fn as_str(&self) -> &str {
        &self.multibase
    }

    /// Get raw hash bytes
    pub fn hash(&self) -> &[u8; 32] {
        &self.hash
    }

    /// Parse CID from string
    pub fn from_str(s: &str) -> Result<Self, CidError> {
        if !s.starts_with("Qm") {
            return Err(CidError::InvalidFormat);
        }

        let hash_str = &s[2..];
        let hash_bytes = bs58::decode(hash_str)
            .into_vec()
            .map_err(|_| CidError::InvalidBase58)?;

        if hash_bytes.len() != 32 {
            return Err(CidError::InvalidHashLength);
        }

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hash_bytes);

        Ok(ContentId {
            hash,
            multibase: s.to_string(),
        })
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.multibase)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CidError {
    #[error("Invalid CID format (must start with Qm)")]
    InvalidFormat,
    #[error("Invalid base58 encoding")]
    InvalidBase58,
    #[error("Invalid hash length (expected 32 bytes)")]
    InvalidHashLength,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cid_creation() {
        let content = b"Hello, decentralized world!";
        let cid = ContentId::new(content);

        assert!(cid.as_str().starts_with("Qm"));
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
        let cid2 = ContentId::from_str(cid1.as_str()).unwrap();

        assert_eq!(cid1, cid2);
    }

    #[test]
    fn test_same_content_same_cid() {
        let cid1 = ContentId::new(b"Same content");
        let cid2 = ContentId::new(b"Same content");

        assert_eq!(cid1, cid2);
    }
}
