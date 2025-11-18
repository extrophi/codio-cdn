# Agent: CDN-EPSILON - Availability Tracking

**Duration:** 1.5 hours
**Branch:** `cdn-epsilon`
**Priority:** MEDIUM (optimization)
**Budget:** $20

## Mission

Implement peer availability tracking, reputation system, and load balancing to optimize content delivery performance.

## Deliverables

### 1. Create `crates/codio-tracker/`

**File:** `crates/codio-tracker/src/lib.rs` (~800 lines)

**Implementation Requirements:**

```rust
// Availability tracker
pub struct AvailabilityTracker {
    peer_metrics: HashMap<PeerId, PeerMetrics>,
    content_availability: HashMap<ContentId, Vec<PeerId>>,
    update_interval: Duration,
}

impl AvailabilityTracker {
    pub async fn new(config: TrackerConfig) -> Result<Self>;

    // Peer tracking
    pub fn record_peer_online(&mut self, peer: PeerId);
    pub fn record_peer_offline(&mut self, peer: PeerId);
    pub fn get_peer_metrics(&self, peer: &PeerId) -> Option<&PeerMetrics>;

    // Content availability
    pub fn record_content_available(&mut self, cid: ContentId, peer: PeerId);
    pub fn get_available_peers(&self, cid: &ContentId) -> Vec<PeerId>;

    // Performance tracking
    pub fn record_download_speed(&mut self, peer: PeerId, bytes_per_sec: u64);
    pub fn record_upload_success(&mut self, peer: PeerId);
    pub fn record_upload_failure(&mut self, peer: PeerId);

    // Load balancing
    pub fn select_best_peers(&self, cid: &ContentId, count: usize) -> Vec<PeerInfo>;
}

// Peer metrics
pub struct PeerMetrics {
    pub peer_id: PeerId,
    pub uptime: Duration,
    pub last_seen: SystemTime,
    pub download_speed: u64,  // bytes/sec (rolling average)
    pub chunks_served: u64,
    pub chunks_failed: u64,
    pub reputation_score: f64,
}

impl PeerMetrics {
    pub fn calculate_reputation(&self) -> f64;
    pub fn is_reliable(&self) -> bool;
}

// Content availability map
pub struct ContentAvailability {
    pub cid: ContentId,
    pub peers: Vec<PeerInfo>,
    pub replication_factor: usize,
    pub last_updated: SystemTime,
}
```

**Features to Implement:**

1. **Peer Metrics**
   - Uptime tracking
   - Bandwidth measurement (rolling average)
   - Success/failure rates
   - Last seen timestamp

2. **Reputation System**
   - Score = (successful_uploads / total_uploads) × uptime_factor
   - Reliable peers score > 0.8
   - Unreliable peers score < 0.3

3. **Load Balancing**
   - Select best N peers for download
   - Prefer high-reputation peers
   - Distribute load across peers
   - Avoid overloading single peer

4. **Gossip Protocol**
   - Share peer metrics with neighbors
   - Propagate content availability
   - Update interval: 30 seconds

### 2. Configuration

**File:** `crates/codio-tracker/src/config.rs`

```rust
pub struct TrackerConfig {
    pub update_interval: Duration,
    pub peer_timeout: Duration,
    pub min_reputation: f64,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            update_interval: Duration::from_secs(30),
            peer_timeout: Duration::from_secs(300),
            min_reputation: 0.3,
        }
    }
}
```

### 3. Dependencies

**File:** `crates/codio-tracker/Cargo.toml`

```toml
[package]
name = "codio-tracker"
version = "0.1.0"
edition = "2021"

[dependencies]
codio-content = { path = "../codio-content" }
codio-network = { path = "../codio-network" }
tokio = { version = "1.35", features = ["full"] }
anyhow = "1.0"
tracing = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

### 4. Testing

**File:** `crates/codio-tracker/tests/integration_test.rs`

```rust
#[test]
fn test_reputation_calculation() {
    let mut metrics = PeerMetrics::default();
    metrics.chunks_served = 100;
    metrics.chunks_failed = 10;
    metrics.uptime = Duration::from_secs(3600);

    let reputation = metrics.calculate_reputation();
    assert!(reputation > 0.8);
}

#[tokio::test]
async fn test_load_balancing() {
    let mut tracker = AvailabilityTracker::new(Default::default()).await.unwrap();

    // Record 5 peers with different speeds
    // Select best 3 peers
    // Verify selection is based on reputation
}

#[tokio::test]
async fn test_peer_timeout() {
    // Record peer online
    // Wait for timeout period
    // Verify peer marked as offline
}
```

## Success Criteria

- ✅ `cargo build` succeeds
- ✅ All tests pass (`cargo test`)
- ✅ Reputation system works correctly
- ✅ Load balancing selects best peers
- ✅ Peer timeouts handled properly
- ✅ Metrics updated in real-time

## Technical Constraints

- Use **rolling averages** for bandwidth (last 10 measurements)
- Reputation score range: 0.0 to 1.0
- Peer timeout: 5 minutes default
- No synchronous blocking operations
- No unsafe code

## Documentation

Include comprehensive docs:
- Reputation algorithm explanation
- Load balancing strategy
- Gossip protocol details
- Example usage in `README.md`

## Branch Workflow

1. Work on branch `cdn-epsilon`
2. Commit frequently with clear messages
3. Run `cargo fmt && cargo clippy` before final commit
4. Create PR to `main` when complete
5. Tag issue #13 in PR description

## Resources

- Gossip protocol: https://en.wikipedia.org/wiki/Gossip_protocol
- Nuclear Deployment Plan: `docs/pm/NUCLEAR-DEPLOYMENT-PLAN.md`

## Notes

Availability tracking makes the CDN **smart and efficient**. By tracking peer performance, we can always download from the fastest, most reliable peers. Focus on:
- Accurate metrics
- Fair reputation scoring
- Intelligent load balancing

**Optimize for speed and reliability.** 📊
