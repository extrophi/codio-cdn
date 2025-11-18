# Agent: CDN-DELTA - Chunk Distribution

**Duration:** 2 hours
**Branch:** `cdn-delta`
**Priority:** HIGH (core feature)
**Budget:** $25

## Mission

Implement BitTorrent-style chunk distribution with parallel downloads, rarest-first strategy, and tit-for-tat uploading.

## Deliverables

### 1. Create `crates/codio-chunk/`

**File:** `crates/codio-chunk/src/lib.rs` (~1,000 lines)

**Implementation Requirements:**

```rust
// Chunk distribution manager
pub struct ChunkDistributor {
    active_downloads: HashMap<ContentId, Download>,
    active_uploads: HashMap<ContentId, Vec<Upload>>,
    strategy: DistributionStrategy,
}

impl ChunkDistributor {
    pub async fn new(config: ChunkConfig) -> Result<Self>;

    // Download operations
    pub async fn download_content(
        &mut self,
        cid: ContentId,
        providers: Vec<PeerInfo>,
    ) -> Result<Vec<u8>>;

    pub fn download_progress(&self, cid: &ContentId) -> Option<DownloadProgress>;

    // Upload operations
    pub async fn serve_chunk(
        &mut self,
        cid: ContentId,
        chunk_index: u32,
        peer: PeerId,
    ) -> Result<Chunk>;
}

// Download tracking
pub struct Download {
    pub cid: ContentId,
    pub total_chunks: u32,
    pub downloaded_chunks: HashSet<u32>,
    pub active_requests: HashMap<u32, PeerInfo>,
    pub strategy: DownloadStrategy,
}

pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub download_rate: f64,  // bytes/sec
    pub peers: usize,
}

// Upload tracking
pub struct Upload {
    pub cid: ContentId,
    pub peer: PeerInfo,
    pub uploaded_chunks: u32,
    pub last_upload: Instant,
}

// Distribution strategies
pub enum DistributionStrategy {
    RarestFirst,     // BitTorrent strategy
    Sequential,      // Linear download
    RandomOrder,     // Random chunk order
}
```

**Features to Implement:**

1. **Parallel Downloads**
   - Download from multiple peers simultaneously
   - 4 concurrent chunk requests per peer
   - Automatic peer failover

2. **Rarest-First Strategy**
   - Query peers for chunk availability
   - Download rarest chunks first
   - Helps distribute rare chunks quickly

3. **Tit-for-Tat**
   - Track upload/download ratios per peer
   - Prefer peers who upload to us
   - Optimistic unchoking (random peer every 30s)

4. **Integrity Verification**
   - Verify each chunk SHA-256 hash
   - Discard corrupted chunks
   - Re-request from different peer

### 2. Configuration

**File:** `crates/codio-chunk/src/config.rs`

```rust
pub struct ChunkConfig {
    pub max_concurrent_downloads: usize,
    pub chunks_per_peer: usize,
    pub request_timeout: Duration,
    pub strategy: DistributionStrategy,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_concurrent_downloads: 10,
            chunks_per_peer: 4,
            request_timeout: Duration::from_secs(30),
            strategy: DistributionStrategy::RarestFirst,
        }
    }
}
```

### 3. Dependencies

**File:** `crates/codio-chunk/Cargo.toml`

```toml
[package]
name = "codio-chunk"
version = "0.1.0"
edition = "2021"

[dependencies]
codio-content = { path = "../codio-content" }
codio-network = { path = "../codio-network" }
codio-dht = { path = "../codio-dht" }
tokio = { version = "1.35", features = ["full"] }
anyhow = "1.0"
tracing = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

### 4. Testing

**File:** `crates/codio-chunk/tests/integration_test.rs`

```rust
#[tokio::test]
async fn test_parallel_download() {
    // Setup 5 peers, each has all chunks
    // Download content from all 5 simultaneously
    // Verify faster than single peer
}

#[tokio::test]
async fn test_rarest_first() {
    // Setup 3 peers with different chunk availability
    // Peer 1: chunks [0, 1, 2]
    // Peer 2: chunks [0, 1, 3]
    // Peer 3: chunks [0, 1]
    // Verify chunks downloaded in order: 2, 3, 0, 1
}

#[tokio::test]
async fn test_chunk_verification() {
    // Download chunk from malicious peer (corrupted hash)
    // Verify chunk rejected
    // Verify re-request from different peer
}

#[tokio::test]
async fn test_tit_for_tat() {
    // Peer uploads chunks to us
    // Verify we prioritize uploading to them
}
```

## Success Criteria

- ✅ `cargo build` succeeds
- ✅ All tests pass (`cargo test`)
- ✅ Parallel downloads work correctly
- ✅ Rarest-first strategy implemented
- ✅ Corrupted chunks detected and rejected
- ✅ Download progress tracking works

## Technical Constraints

- Use **async/await** throughout
- Maximum 4 chunk requests per peer
- 30 second timeout per chunk
- Verify SHA-256 hash for every chunk
- No unsafe code

## Documentation

Include comprehensive docs:
- BitTorrent strategy explanation
- Rarest-first algorithm
- Tit-for-tat mechanism
- Example usage in `README.md`

## Branch Workflow

1. Work on branch `cdn-delta`
2. Commit frequently with clear messages
3. Run `cargo fmt && cargo clippy` before final commit
4. Create PR to `main` when complete
5. Tag issue #12 in PR description

## Resources

- BitTorrent spec: http://bittorrent.org/beps/bep_0003.html
- Nuclear Deployment Plan: `docs/pm/NUCLEAR-DEPLOYMENT-PLAN.md`

## Notes

Chunk distribution is what makes the CDN **fast and resilient**. The rarest-first strategy ensures content spreads quickly across the network. Focus on:
- Parallel performance
- Fault tolerance
- Fair sharing (tit-for-tat)

**Speed is everything.** ⚡
