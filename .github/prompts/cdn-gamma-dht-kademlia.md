# Agent: CDN-GAMMA - DHT (Kademlia)

**Duration:** 2 hours
**Branch:** `cdn-gamma`
**Priority:** CRITICAL (content routing)
**Budget:** $25

## Mission

Implement a Kademlia DHT for distributed content routing, peer discovery, and content provider tracking using libp2p.

## Deliverables

### 1. Create `crates/codio-dht/`

**File:** `crates/codio-dht/src/lib.rs` (~1,800 lines)

**Implementation Requirements:**

```rust
// DHT Manager
pub struct DHTManager {
    kademlia: Kademlia<MemoryStore>,
    peer_id: PeerId,
    replication_factor: usize,
}

impl DHTManager {
    pub async fn new(peer_id: PeerId, config: DHTConfig) -> Result<Self>;
    pub async fn bootstrap(&mut self, bootstrap_peers: Vec<Multiaddr>) -> Result<()>;

    // Content provider operations
    pub async fn provide(&mut self, cid: ContentId) -> Result<()>;
    pub async fn find_providers(&mut self, cid: ContentId) -> Result<Vec<PeerInfo>>;
    pub async fn stop_providing(&mut self, cid: ContentId) -> Result<()>;

    // Peer operations
    pub async fn find_peer(&mut self, peer_id: PeerId) -> Result<Vec<Multiaddr>>;
    pub fn get_closest_peers(&self, key: &[u8]) -> Vec<PeerId>;
}

// Provider record
pub struct ProviderRecord {
    pub cid: ContentId,
    pub provider: PeerInfo,
    pub timestamp: SystemTime,
    pub distance: Distance,  // XOR distance
}

// DHT statistics
pub struct DHTStats {
    pub num_peers: usize,
    pub num_providers: usize,
    pub routing_table_size: usize,
    pub pending_queries: usize,
}

impl DHTManager {
    pub fn stats(&self) -> DHTStats;
}
```

**Features to Implement:**

1. **Kademlia DHT**
   - XOR distance metric
   - K-buckets for routing table
   - Replication factor = 20

2. **Content Routing**
   - `provide(cid)` - Announce you have content
   - `find_providers(cid)` - Find who has content
   - Provider record expiration (24 hours)

3. **Peer Discovery**
   - `find_peer(peer_id)` - Locate peer by ID
   - `get_closest_peers(key)` - Find K closest peers
   - Bootstrap node support

4. **Fault Tolerance**
   - Automatic re-providing (every 12 hours)
   - Provider record redundancy
   - Routing table maintenance

### 2. Configuration

**File:** `crates/codio-dht/src/config.rs`

```rust
pub struct DHTConfig {
    pub replication_factor: usize,
    pub provider_timeout: Duration,
    pub republish_interval: Duration,
    pub query_timeout: Duration,
}

impl Default for DHTConfig {
    fn default() -> Self {
        Self {
            replication_factor: 20,
            provider_timeout: Duration::from_secs(24 * 3600),
            republish_interval: Duration::from_secs(12 * 3600),
            query_timeout: Duration::from_secs(60),
        }
    }
}
```

### 3. Dependencies

**File:** `crates/codio-dht/Cargo.toml`

```toml
[package]
name = "codio-dht"
version = "0.1.0"
edition = "2021"

[dependencies]
codio-content = { path = "../codio-content" }
libp2p = { version = "0.53", features = ["kad"] }
tokio = { version = "1.35", features = ["full"] }
anyhow = "1.0"
tracing = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

### 4. Testing

**File:** `crates/codio-dht/tests/integration_test.rs`

```rust
#[tokio::test]
async fn test_provide_and_find() {
    // Setup 3 DHT nodes
    // Node 1 provides CID
    // Node 2 finds providers for CID
    // Verify Node 1 is in results
}

#[tokio::test]
async fn test_replication() {
    // Setup 25 nodes
    // Provide CID from Node 1
    // Verify 20+ nodes have the provider record
}

#[tokio::test]
async fn test_bootstrap() {
    // Start bootstrap node
    // Start 5 nodes with bootstrap address
    // Verify all nodes discover each other
}

#[tokio::test]
async fn test_peer_discovery() {
    // Start 10 nodes
    // Each node finds closest peers to random key
    // Verify XOR distance ordering
}
```

## Success Criteria

- ✅ `cargo build` succeeds
- ✅ All tests pass (`cargo test`)
- ✅ `provide()` replicates to 20+ nodes
- ✅ `find_providers()` returns correct peers
- ✅ Bootstrap nodes work correctly
- ✅ Routing table maintains itself

## Technical Constraints

- Use **libp2p Kademlia** implementation
- Replication factor = 20 minimum
- Query timeout = 60 seconds max
- No synchronous blocking operations
- No unsafe code

## Documentation

Include comprehensive docs:
- Kademlia DHT explanation
- XOR distance metric
- Provider record lifecycle
- Example usage in `README.md`

## Branch Workflow

1. Work on branch `cdn-gamma`
2. Commit frequently with clear messages
3. Run `cargo fmt && cargo clippy` before final commit
4. Create PR to `main` when complete
5. Tag issue #11 in PR description

## Resources

- Kademlia paper: https://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf
- libp2p Kademlia: `docs/dev/libp2p/`
- Nuclear Deployment Plan: `docs/pm/NUCLEAR-DEPLOYMENT-PLAN.md`

## Notes

The DHT is how peers **find each other and content**. Without this, the CDN cannot function. Focus on:
- Fast content lookups (< 5 seconds)
- High availability (20x replication)
- Network scalability

**This is the CDN's nervous system.** 🧠
