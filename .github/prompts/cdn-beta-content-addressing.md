# Agent: CDN-BETA - Content Addressing

**Duration:** 2 hours
**Branch:** `cdn-beta`
**Priority:** CRITICAL (core functionality)
**Budget:** $25

## Mission

Implement IPFS-style content addressing with SHA-256 hashing, CID generation, and content chunking for the decentralized CDN.

## Deliverables

### 1. Create `crates/codio-content/`

**File:** `crates/codio-content/src/lib.rs` (~1,200 lines)

**Implementation Requirements:**

```rust
// Content Identifier (IPFS CIDv0 compatible)
pub struct ContentId {
    hash: [u8; 32],
    multibase: String,  // Qm... format
}

impl ContentId {
    pub fn new(content: &[u8]) -> Self;
    pub fn from_string(cid: &str) -> Result<Self>;
    pub fn to_string(&self) -> String;
    pub fn verify(&self, content: &[u8]) -> bool;
    pub fn hash(&self) -> &[u8; 32];
}

// Content chunking (1MB chunks)
pub struct Chunker {
    chunk_size: usize,
}

impl Chunker {
    pub fn new(chunk_size: usize) -> Self;
    pub fn chunk(&self, data: &[u8]) -> Vec<Chunk>;
    pub fn reconstruct(&self, chunks: Vec<Chunk>) -> Result<Vec<u8>>;
}

pub struct Chunk {
    pub index: u32,
    pub data: Vec<u8>,
    pub cid: ContentId,
}

// Merkle DAG for chunk relationships
pub struct MerkleDAG {
    root: ContentId,
    children: Vec<ContentId>,
}

impl MerkleDAG {
    pub fn from_chunks(chunks: &[Chunk]) -> Self;
    pub fn verify(&self, chunks: &[Chunk]) -> bool;
    pub fn root_cid(&self) -> &ContentId;
}
```

**Features to Implement:**

1. **CID Generation**
   - SHA-256 hashing
   - Base58 encoding (IPFS CIDv0 format)
   - Qm prefix for compatibility

2. **Content Chunking**
   - Fixed-size chunks (1MB default)
   - SHA-256 hash per chunk
   - Chunk index tracking

3. **Merkle DAG**
   - Root CID from child CIDs
   - Integrity verification
   - Chunk ordering preservation

4. **Content Verification**
   - Hash verification per chunk
   - Full content verification
   - Corruption detection

### 2. Configuration

**File:** `crates/codio-content/src/config.rs`

```rust
pub struct ContentConfig {
    pub chunk_size: usize,
    pub enable_compression: bool,
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1024 * 1024, // 1MB
            enable_compression: false,
        }
    }
}
```

### 3. Dependencies

**File:** `crates/codio-content/Cargo.toml`

```toml
[package]
name = "codio-content"
version = "0.1.0"
edition = "2021"

[dependencies]
sha2 = "0.10"
bs58 = "0.5"
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
bytes = "1.5"
```

### 4. Testing

**File:** `crates/codio-content/tests/integration_test.rs`

```rust
#[test]
fn test_cid_generation() {
    let content = b"Hello, Codio CDN!";
    let cid = ContentId::new(content);
    assert!(cid.to_string().starts_with("Qm"));
    assert!(cid.verify(content));
}

#[test]
fn test_chunking() {
    let data = vec![0u8; 5 * 1024 * 1024]; // 5MB
    let chunker = Chunker::new(1024 * 1024);
    let chunks = chunker.chunk(&data);
    assert_eq!(chunks.len(), 5);

    let reconstructed = chunker.reconstruct(chunks).unwrap();
    assert_eq!(data, reconstructed);
}

#[test]
fn test_merkle_dag() {
    let data = vec![0u8; 3 * 1024 * 1024];
    let chunker = Chunker::new(1024 * 1024);
    let chunks = chunker.chunk(&data);

    let dag = MerkleDAG::from_chunks(&chunks);
    assert!(dag.verify(&chunks));
}

#[test]
fn test_corruption_detection() {
    let content = b"Original content";
    let cid = ContentId::new(content);

    let tampered = b"Tampered content";
    assert!(!cid.verify(tampered));
}
```

## Success Criteria

- ✅ `cargo build` succeeds
- ✅ All tests pass (`cargo test`)
- ✅ CID generation matches IPFS format
- ✅ Chunking handles files of any size
- ✅ Merkle DAG verification works
- ✅ Corruption detection is 100% reliable

## Technical Constraints

- Use **SHA-256** (not MD5 or SHA-1)
- IPFS CIDv0 compatibility required
- Chunk size must be configurable
- All CIDs must start with "Qm"
- No unsafe code

## Documentation

Include comprehensive docs:
- CID format explanation
- Chunking algorithm description
- Merkle DAG structure
- Example usage in `README.md`

## Branch Workflow

1. Work on branch `cdn-beta`
2. Commit frequently with clear messages
3. Run `cargo fmt && cargo clippy` before final commit
4. Create PR to `main` when complete
5. Tag issue #10 in PR description

## Resources

- SHA-256 docs: Rust std library
- Base58 encoding: bs58 crate
- IPFS CID spec: https://docs.ipfs.tech/concepts/content-addressing/
- Nuclear Deployment Plan: `docs/pm/NUCLEAR-DEPLOYMENT-PLAN.md`

## Notes

Content addressing is the **core innovation** that enables decentralization. Every piece of content must have a unique, verifiable identifier. Focus on:
- Cryptographic integrity
- IPFS compatibility
- Performance (fast hashing)

**This is the DNA of the CDN.** 🧬
