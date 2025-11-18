# Agent: CDN-ALPHA - P2P Network Layer

**Duration:** 2 hours
**Branch:** `cdn-alpha`
**Priority:** CRITICAL (foundation layer)
**Budget:** $25

## Mission

Implement the P2P network layer using libp2p for peer discovery, connection management, and transport encryption.

## Deliverables

### 1. Create `crates/codio-network/`

**File:** `crates/codio-network/src/lib.rs` (~1,500 lines)

**Implementation Requirements:**

```rust
// Core network manager
pub struct NetworkManager {
    swarm: Swarm<NetworkBehaviour>,
    peer_id: PeerId,
    listen_addr: Multiaddr,
    connected_peers: HashMap<PeerId, PeerInfo>,
}

impl NetworkManager {
    pub async fn new(config: NetworkConfig) -> Result<Self>;
    pub async fn start(&mut self) -> Result<()>;
    pub async fn connect_peer(&mut self, addr: Multiaddr) -> Result<PeerId>;
    pub async fn disconnect_peer(&mut self, peer_id: PeerId) -> Result<()>;
    pub fn connected_peers(&self) -> &HashMap<PeerId, PeerInfo>;
}

// Peer information tracking
pub struct PeerInfo {
    pub peer_id: PeerId,
    pub addresses: Vec<Multiaddr>,
    pub connection_time: Instant,
    pub last_seen: Instant,
}

// Network behavior (libp2p protocols)
#[derive(NetworkBehaviour)]
pub struct NetworkBehaviour {
    mdns: mdns::tokio::Behaviour,
    kademlia: Kademlia<MemoryStore>,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}
```

**Features to Implement:**

1. **Peer Discovery**
   - mDNS for local network discovery
   - Kademlia for global peer discovery
   - Bootstrap nodes support

2. **Connection Management**
   - Automatic reconnection on failure
   - Connection limits (max peers)
   - NAT traversal via relay

3. **Transport Layer**
   - TCP transport
   - Noise protocol for encryption
   - Yamux for multiplexing

4. **Protocol Support**
   - `/codio/1.0.0` custom protocol
   - Ping for keep-alive
   - Identify for peer metadata

### 2. Configuration

**File:** `crates/codio-network/src/config.rs`

```rust
pub struct NetworkConfig {
    pub listen_port: u16,
    pub max_peers: usize,
    pub bootstrap_peers: Vec<Multiaddr>,
    pub enable_mdns: bool,
    pub enable_relay: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_port: 0, // random port
            max_peers: 50,
            bootstrap_peers: vec![],
            enable_mdns: true,
            enable_relay: true,
        }
    }
}
```

### 3. Dependencies

**File:** `crates/codio-network/Cargo.toml`

```toml
[package]
name = "codio-network"
version = "0.1.0"
edition = "2021"

[dependencies]
libp2p = { version = "0.53", features = [
    "tcp",
    "noise",
    "yamux",
    "mdns",
    "kad",
    "ping",
    "identify",
    "relay",
] }
tokio = { version = "1.35", features = ["full"] }
anyhow = "1.0"
tracing = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

### 4. Testing

**File:** `crates/codio-network/tests/integration_test.rs`

```rust
#[tokio::test]
async fn test_peer_discovery() {
    // Start 3 peers
    // Verify they discover each other via mDNS
    // Verify peer count = 2 for each peer
}

#[tokio::test]
async fn test_connection_management() {
    // Connect two peers
    // Disconnect one peer
    // Verify connection state updates
}

#[tokio::test]
async fn test_bootstrap_nodes() {
    // Start bootstrap peer
    // Start regular peer with bootstrap address
    // Verify connection established
}
```

## Success Criteria

- ✅ `cargo build` succeeds
- ✅ All tests pass (`cargo test`)
- ✅ 3 peers can discover each other locally (mDNS)
- ✅ Peers can connect/disconnect cleanly
- ✅ Network manager handles reconnection
- ✅ Code is well-documented

## Technical Constraints

- Use **libp2p 0.53+** (latest stable)
- Follow Rust naming conventions
- Use `tracing` for logging (not `println!`)
- Handle all errors with `anyhow::Result`
- No unsafe code

## Documentation

Include comprehensive docs:
- Module-level documentation
- Function documentation with examples
- Architecture overview in `README.md`

## Branch Workflow

1. Work on branch `cdn-alpha`
2. Commit frequently with clear messages
3. Run `cargo fmt && cargo clippy` before final commit
4. Create PR to `main` when complete
5. Tag issue #9 in PR description

## Resources

- libp2p docs: `docs/dev/libp2p/`
- Tokio async runtime: `docs/dev/tokio/`
- Nuclear Deployment Plan: `docs/pm/NUCLEAR-DEPLOYMENT-PLAN.md`

## Notes

This is the **foundation layer** for the entire CDN. All other agents depend on this working correctly. Focus on:
- Robust connection handling
- Clean API design
- Comprehensive testing

**No shortcuts. Build it right.** 🔥
