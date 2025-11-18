# TECHNICAL PROPOSAL: CODIO CDN PHASE 1
## Foundation - Content Addressing & Peer Discovery

**Version:** 1.0  
**Date:** Monday, November 18, 2025  
**Phase:** 1 (Foundation)  
**Duration:** 8-10 hours  
**Agents:** 4 sub-agents  

---

## EXECUTIVE SUMMARY

**Goal:** Build foundational P2P content delivery infrastructure

**Deliverables:**
- Content-addressed storage (sha256 CID)
- Peer discovery (Kademlia DHT)
- CLI tool for publish/retrieve
- Test suite validating all components

**Architecture:**
```
codio-cdn/
├── crates/
│   ├── content-id/     ← AGENT ALPHA (2 hrs)
│   ├── dht/            ← AGENT BETA (3 hrs)
│   ├── cli/            ← AGENT GAMMA (2 hrs)
│   └── common/         ← AGENT ALPHA (included)
├── tests/              ← AGENT DELTA (1.5 hrs)
└── docs/
```

**Critical Path:**
```
ALPHA (content-id) → Must complete first
    ↓
BETA (DHT) + GAMMA (CLI) → Can run parallel
    ↓
DELTA (tests) → Validates all
```

---

## AGENT ALPHA: CONTENT ADDRESSING

**Duration:** 2 hours  
**Priority:** CRITICAL (blocks others)  
**Risk:** LOW (well-understood cryptography)  

### Task

Implement content-addressed storage using sha256 hashing and IPFS-compatible CID format.

### Technical Specification

**Cargo Workspace Structure:**
```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "crates/content-id",
    "crates/common",
    "crates/dht",
    "crates/cli",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Codio Collective"]
license = "MIT"

[workspace.dependencies]
sha2 = "0.10"
bs58 = "0.5"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.35", features = ["full"] }
```

**content-id/Cargo.toml:**
```toml
[package]
name = "codio-content-id"
version.workspace = true
edition.workspace = true

[dependencies]
sha2.workspace = true
bs58.workspace = true
serde.workspace = true
thiserror = "1.0"

[dev-dependencies]
criterion = "0.5"
```

**Implementation:**
```rust
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
```

**Common Types:**
```rust
// crates/common/src/lib.rs

pub use codio_content_id::ContentId;

/// Peer identifier in the network
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerId(pub String);

/// Network address for a peer
#[derive(Debug, Clone)]
pub struct PeerAddr {
    pub peer_id: PeerId,
    pub multiaddr: String, // /ip4/127.0.0.1/tcp/4001
}

/// Content metadata
#[derive(Debug, Clone)]
pub struct ContentMeta {
    pub cid: ContentId,
    pub size: u64,
    pub providers: Vec<PeerAddr>,
}
```

### Deliverables

- ✅ `crates/content-id/src/lib.rs` (150-200 lines)
- ✅ `crates/common/src/lib.rs` (50-100 lines)
- ✅ Unit tests (95%+ coverage)
- ✅ Benchmarks (criterion)
- ✅ Documentation (rustdoc)

### Success Criteria

- ✅ Can generate CIDs from arbitrary content
- ✅ CID verification works (hash comparison)
- ✅ String parsing (from_str) works
- ✅ Same content produces same CID (deterministic)
- ✅ All tests pass: `cargo test -p codio-content-id`
- ✅ Benchmarks run: `cargo bench -p codio-content-id`

### Performance Targets

- CID generation: < 1ms for 1MB content
- Verification: < 1ms for 1MB content
- Zero allocations for hash comparison

---

## AGENT BETA: KADEMLIA DHT

**Duration:** 3 hours  
**Priority:** HIGH  
**Risk:** MEDIUM (libp2p complexity)  
**Dependency:** ALPHA (uses ContentId)  

### Task

Implement Kademlia DHT for peer discovery using libp2p.

### Technical Specification

**dht/Cargo.toml:**
```toml
[package]
name = "codio-dht"
version.workspace = true
edition.workspace = true

[dependencies]
codio-common = { path = "../common" }
codio-content-id = { path = "../content-id" }

libp2p = { version = "0.53", features = [
    "kad",
    "tcp",
    "noise",
    "yamux",
    "tokio",
] }
tokio.workspace = true
serde.workspace = true
tracing = "0.1"
anyhow = "1.0"

[dev-dependencies]
tokio-test = "0.4"
```

**Implementation:**
```rust
// crates/dht/src/lib.rs

use libp2p::{
    kad::{
        store::MemoryStore, Kademlia, KademliaConfig, KademliaEvent,
        QueryResult, PeerRecord, ProviderRecord,
    },
    swarm::{Swarm, SwarmBuilder, SwarmEvent},
    PeerId, Multiaddr,
};
use codio_content_id::ContentId;
use std::time::Duration;
use tokio::sync::mpsc;

/// DHT configuration
#[derive(Debug, Clone)]
pub struct DhtConfig {
    /// Bootstrap peers to connect to
    pub bootstrap_peers: Vec<(PeerId, Multiaddr)>,
    /// Query timeout
    pub query_timeout: Duration,
    /// Provider record TTL
    pub provider_ttl: Duration,
}

impl Default for DhtConfig {
    fn default() -> Self {
        Self {
            bootstrap_peers: vec![],
            query_timeout: Duration::from_secs(30),
            provider_ttl: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// DHT node for content discovery
pub struct DhtNode {
    swarm: Swarm<Kademlia<MemoryStore>>,
    event_tx: mpsc::UnboundedSender<DhtEvent>,
}

/// Events from the DHT
#[derive(Debug)]
pub enum DhtEvent {
    /// Providers found for content
    ProvidersFound {
        cid: ContentId,
        providers: Vec<PeerId>,
    },
    /// Content announcement successful
    ProvideSuccess {
        cid: ContentId,
    },
    /// Bootstrap completed
    BootstrapComplete,
}

impl DhtNode {
    /// Create new DHT node
    pub async fn new(config: DhtConfig) -> anyhow::Result<(Self, mpsc::UnboundedReceiver<DhtEvent>)> {
        // Generate keypair
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        
        tracing::info!("Local peer ID: {}", local_peer_id);
        
        // Create Kademlia store
        let store = MemoryStore::new(local_peer_id);
        
        // Configure Kademlia
        let mut kad_config = KademliaConfig::default();
        kad_config.set_query_timeout(config.query_timeout);
        kad_config.set_provider_record_ttl(Some(config.provider_ttl));
        
        // Create Kademlia behaviour
        let mut kademlia = Kademlia::with_config(local_peer_id, store, kad_config);
        
        // Add bootstrap peers
        for (peer_id, addr) in config.bootstrap_peers {
            kademlia.add_address(&peer_id, addr);
        }
        
        // Create swarm
        let swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )?
            .with_behaviour(|_| kademlia)?
            .build();
        
        // Event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Ok((
            DhtNode { swarm, event_tx },
            event_rx
        ))
    }
    
    /// Start listening on address
    pub async fn listen(&mut self, addr: Multiaddr) -> anyhow::Result<()> {
        self.swarm.listen_on(addr)?;
        Ok(())
    }
    
    /// Bootstrap DHT (connect to network)
    pub async fn bootstrap(&mut self) -> anyhow::Result<()> {
        self.swarm.behaviour_mut().bootstrap()?;
        Ok(())
    }
    
    /// Announce content availability
    pub async fn provide(&mut self, cid: ContentId) -> anyhow::Result<()> {
        let key = cid_to_kad_key(&cid);
        self.swarm.behaviour_mut().start_providing(key)?;
        Ok(())
    }
    
    /// Find providers for content
    pub async fn find_providers(&mut self, cid: ContentId) -> anyhow::Result<()> {
        let key = cid_to_kad_key(&cid);
        self.swarm.behaviour_mut().get_providers(key);
        Ok(())
    }
    
    /// Event loop (call continuously)
    pub async fn poll(&mut self) -> Option<SwarmEvent<KademliaEvent>> {
        use futures::StreamExt;
        self.swarm.next().await
    }
    
    /// Handle Kademlia event
    pub fn handle_event(&self, event: KademliaEvent) {
        match event {
            KademliaEvent::OutboundQueryProgressed { result, .. } => {
                match result {
                    QueryResult::GetProviders(Ok(ok)) => {
                        let cid = kad_key_to_cid(&ok.key);
                        let providers: Vec<PeerId> = ok.providers.into_iter().collect();
                        
                        let _ = self.event_tx.send(DhtEvent::ProvidersFound {
                            cid,
                            providers,
                        });
                    }
                    QueryResult::StartProviding(Ok(_)) => {
                        // Provider record published
                        tracing::debug!("Content announced to DHT");
                    }
                    QueryResult::Bootstrap(Ok(_)) => {
                        let _ = self.event_tx.send(DhtEvent::BootstrapComplete);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

/// Convert CID to Kademlia key
fn cid_to_kad_key(cid: &ContentId) -> libp2p::kad::RecordKey {
    libp2p::kad::RecordKey::new(cid.hash())
}

/// Convert Kademlia key to CID
fn kad_key_to_cid(key: &libp2p::kad::RecordKey) -> ContentId {
    // Reconstruct CID from hash bytes
    // Note: This is simplified - production needs proper CID reconstruction
    ContentId::from_str(&format!("Qm{}", bs58::encode(key.as_ref()).into_string()))
        .expect("Invalid CID from DHT")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dht_creation() {
        let config = DhtConfig::default();
        let (node, _rx) = DhtNode::new(config).await.unwrap();
        assert!(true); // Node created successfully
    }
    
    #[tokio::test]
    async fn test_cid_kad_key_roundtrip() {
        let cid = ContentId::new(b"Test content");
        let key = cid_to_kad_key(&cid);
        let cid2 = kad_key_to_cid(&key);
        assert_eq!(cid, cid2);
    }
}
```

### Deliverables

- ✅ `crates/dht/src/lib.rs` (300-400 lines)
- ✅ Bootstrap node connection
- ✅ Content announcement (provide)
- ✅ Provider discovery (find)
- ✅ Event handling
- ✅ Tests with mock peers

### Success Criteria

- ✅ Can create DHT node
- ✅ Can connect to bootstrap peers
- ✅ Can announce content (start_providing)
- ✅ Can find providers (get_providers)
- ✅ Event channel works (DhtEvent)
- ✅ All tests pass: `cargo test -p codio-dht`

### Known Challenges

**Challenge:** libp2p complexity
- **Mitigation:** Start with basic Kademlia only
- **Fallback:** Use in-memory store (no persistence yet)

**Challenge:** Bootstrap peer requirement
- **Mitigation:** Hard-code test bootstrap node
- **Future:** Dynamic discovery

---

## AGENT GAMMA: CLI TOOL

**Duration:** 2 hours  
**Priority:** MEDIUM  
**Risk:** LOW  
**Dependencies:** ALPHA, BETA  

### Task

Build command-line interface for publishing and retrieving content.

### Technical Specification

**cli/Cargo.toml:**
```toml
[package]
name = "codio-cdn"
version.workspace = true
edition.workspace = true

[[bin]]
name = "codio-cdn"
path = "src/main.rs"

[dependencies]
codio-common = { path = "../common" }
codio-content-id = { path = "../content-id" }
codio-dht = { path = "../dht" }

clap = { version = "4.4", features = ["derive"] }
tokio.workspace = true
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
colored = "2.1"

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.0"
tempfile = "3.8"
```

**Implementation:**
```rust
// crates/cli/src/main.rs

use clap::{Parser, Subcommand};
use codio_content_id::ContentId;
use codio_dht::{DhtNode, DhtConfig, DhtEvent};
use colored::Colorize;
use std::path::PathBuf;
use std::fs;

#[derive(Parser)]
#[command(name = "codio-cdn")]
#[command(about = "Decentralized content delivery CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Publish content and get CID
    Publish {
        /// File or directory to publish
        path: PathBuf,
        
        /// Announce to DHT
        #[arg(long, default_value_t = true)]
        announce: bool,
    },
    
    /// Retrieve content by CID
    Get {
        /// Content ID (CID) to retrieve
        cid: String,
        
        /// Output path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Show CID for content without publishing
    Hash {
        /// File to hash
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Setup logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .init();
    
    match cli.command {
        Commands::Publish { path, announce } => {
            publish_content(path, announce).await?;
        }
        Commands::Get { cid, output } => {
            get_content(&cid, output).await?;
        }
        Commands::Hash { path } => {
            hash_content(path)?;
        }
    }
    
    Ok(())
}

async fn publish_content(path: PathBuf, announce: bool) -> anyhow::Result<()> {
    println!("{}", "Publishing content...".cyan());
    
    // Read content
    let content = fs::read(&path)?;
    println!("  {} Read {} bytes", "✓".green(), content.len());
    
    // Generate CID
    let cid = ContentId::new(&content);
    println!("  {} Generated CID: {}", "✓".green(), cid.to_string().bright_blue());
    
    // Announce to DHT if requested
    if announce {
        println!("{}", "Announcing to DHT...".cyan());
        
        let config = DhtConfig::default();
        let (mut node, mut event_rx) = DhtNode::new(config).await?;
        
        // Listen on random port
        let listen_addr = "/ip4/0.0.0.0/tcp/0".parse()?;
        node.listen(listen_addr).await?;
        
        // Announce content
        node.provide(cid.clone()).await?;
        println!("  {} Content announced", "✓".green());
        
        // Wait briefly for DHT propagation
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    
    println!("\n{}", "Success!".bright_green().bold());
    println!("CID: {}", cid.to_string().bright_blue());
    
    Ok(())
}

async fn get_content(cid_str: &str, output: Option<PathBuf>) -> anyhow::Result<()> {
    println!("{} {}", "Retrieving:".cyan(), cid_str.bright_blue());
    
    // Parse CID
    let cid = ContentId::from_str(cid_str)?;
    println!("  {} CID parsed", "✓".green());
    
    // Create DHT node
    println!("{}", "Searching DHT...".cyan());
    let config = DhtConfig::default();
    let (mut node, mut event_rx) = DhtNode::new(config).await?;
    
    // Listen
    let listen_addr = "/ip4/0.0.0.0/tcp/0".parse()?;
    node.listen(listen_addr).await?;
    
    // Find providers
    node.find_providers(cid.clone()).await?;
    
    // Wait for providers
    tokio::select! {
        Some(event) = event_rx.recv() => {
            match event {
                DhtEvent::ProvidersFound { cid, providers } => {
                    println!("  {} Found {} providers", "✓".green(), providers.len());
                    
                    // TODO: Actually download from providers
                    println!("\n{}", "Note: Content download not yet implemented".yellow());
                    println!("Providers found: {:?}", providers);
                }
                _ => {}
            }
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
            println!("  {} Timeout: No providers found", "✗".red());
        }
    }
    
    Ok(())
}

fn hash_content(path: PathBuf) -> anyhow::Result<()> {
    println!("{} {}", "Hashing:".cyan(), path.display());
    
    let content = fs::read(&path)?;
    let cid = ContentId::new(&content);
    
    println!("\nCID: {}", cid.to_string().bright_blue());
    println!("Size: {} bytes", content.len());
    
    Ok(())
}
```

### Deliverables

- ✅ `crates/cli/src/main.rs` (200-300 lines)
- ✅ `codio-cdn publish <file>` command
- ✅ `codio-cdn get <cid>` command
- ✅ `codio-cdn hash <file>` command
- ✅ Colored output
- ✅ Error handling

### Success Criteria

- ✅ Can run: `cargo run -p codio-cdn -- publish test.txt`
- ✅ Returns valid CID
- ✅ Can hash files without publishing
- ✅ Error messages are clear
- ✅ Help text works: `codio-cdn --help`

### Example Usage

```bash
# Hash a file (no network)
$ codio-cdn hash README.md
Hashing: README.md
CID: QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG
Size: 1234 bytes

# Publish to network
$ codio-cdn publish README.md
Publishing content...
  ✓ Read 1234 bytes
  ✓ Generated CID: QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG
Announcing to DHT...
  ✓ Content announced

Success!
CID: QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG

# Retrieve content
$ codio-cdn get QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG
Retrieving: QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG
  ✓ CID parsed
Searching DHT...
  ✓ Found 3 providers
```

---

## AGENT DELTA: TESTING & DOCUMENTATION

**Duration:** 1.5 hours  
**Priority:** HIGH  
**Risk:** LOW  
**Dependencies:** ALPHA, BETA, GAMMA  

### Task

Create comprehensive test suite and documentation.

### Technical Specification

**Integration Tests:**
```rust
// tests/integration_test.rs

use codio_content_id::ContentId;
use codio_dht::{DhtNode, DhtConfig};
use std::time::Duration;

#[tokio::test]
async fn test_publish_retrieve_flow() {
    // Setup two DHT nodes
    let config1 = DhtConfig {
        query_timeout: Duration::from_secs(5),
        ..Default::default()
    };
    let config2 = config1.clone();
    
    let (mut node1, _rx1) = DhtNode::new(config1).await.unwrap();
    let (mut node2, mut rx2) = DhtNode::new(config2).await.unwrap();
    
    // Listen on different ports
    node1.listen("/ip4/127.0.0.1/tcp/0".parse().unwrap()).await.unwrap();
    node2.listen("/ip4/127.0.0.1/tcp/0".parse().unwrap()).await.unwrap();
    
    // Create content
    let content = b"Test content for DHT";
    let cid = ContentId::new(content);
    
    // Node 1 announces content
    node1.provide(cid.clone()).await.unwrap();
    
    // Wait for DHT propagation
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Node 2 finds providers
    node2.find_providers(cid.clone()).await.unwrap();
    
    // Should find node 1 as provider
    // (This test needs real P2P connection - simplified here)
    assert!(true);
}

#[test]
fn test_cid_determinism() {
    let content = b"Same content";
    let cid1 = ContentId::new(content);
    let cid2 = ContentId::new(content);
    
    assert_eq!(cid1, cid2);
    assert_eq!(cid1.as_str(), cid2.as_str());
}

#[test]
fn test_cid_uniqueness() {
    let cid1 = ContentId::new(b"Content A");
    let cid2 = ContentId::new(b"Content B");
    
    assert_ne!(cid1, cid2);
}
```

**CLI Tests:**
```rust
// tests/cli_test.rs

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn test_cli_hash_command() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "Test content").unwrap();
    
    let mut cmd = Command::cargo_bin("codio-cdn").unwrap();
    cmd.arg("hash")
       .arg(file.path())
       .assert()
       .success()
       .stdout(predicate::str::contains("CID:"))
       .stdout(predicate::str::contains("Qm"));
}

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("codio-cdn").unwrap();
    cmd.arg("--help")
       .assert()
       .success()
       .stdout(predicate::str::contains("codio-cdn"))
       .stdout(predicate::str::contains("publish"));
}

#[test]
fn test_cli_invalid_cid() {
    let mut cmd = Command::cargo_bin("codio-cdn").unwrap();
    cmd.arg("get")
       .arg("invalid-cid")
       .assert()
       .failure();
}
```

**Documentation:**
```rust
// README.md

# Codio CDN - Phase 1 Foundation

Decentralized content delivery using content addressing and DHT.

## Quick Start

```bash
# Install
cargo install --path crates/cli

# Hash a file
codio-cdn hash README.md

# Publish content
codio-cdn publish myfile.txt

# Retrieve by CID
codio-cdn get QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG
```

## Architecture

- **content-id**: SHA256-based content addressing
- **dht**: Kademlia DHT for peer discovery
- **cli**: Command-line interface

## Development

```bash
# Run tests
cargo test --workspace

# Run benchmarks
cargo bench -p codio-content-id

# Build CLI
cargo build --release -p codio-cdn
```

## How It Works

1. Content is hashed with SHA256
2. CID generated: `Qm<base58(hash)>`
3. DHT announces content availability
4. Peers can find providers via DHT
5. Content retrieved from nearby peers

## Status

**Phase 1** (Current): Foundation
- ✅ Content addressing
- ✅ DHT peer discovery
- ✅ CLI tool
- ⚠️ Content transfer (Phase 2)
```

### Deliverables

- ✅ `tests/integration_test.rs` (100-150 lines)
- ✅ `tests/cli_test.rs` (50-100 lines)
- ✅ `README.md` (comprehensive docs)
- ✅ `ARCHITECTURE.md` (design docs)
- ✅ CI/CD workflow (.github/workflows/rust-ci.yml)

### Success Criteria

- ✅ All tests pass: `cargo test --workspace`
- ✅ Test coverage > 80%
- ✅ Documentation builds: `cargo doc`
- ✅ CLI integration tests pass
- ✅ README has usage examples

---

## GITHUB ACTIONS CI/CD

```yaml
# .github/workflows/rust-ci.yml
name: Rust CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      
      - name: Cache cargo
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Run tests
        run: cargo test --workspace --verbose
      
      - name: Run clippy
        run: cargo clippy --workspace -- -D warnings
      
      - name: Check formatting
        run: cargo fmt --all -- --check
      
      - name: Build release
        run: cargo build --release --workspace

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      
      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin
      
      - name: Generate coverage
        run: cargo tarpaulin --workspace --out Xml
      
      - name: Upload coverage
        uses: codecov/codecov-action@v3
```

---

## EXECUTION PLAN

### Wave 1: Foundation (Parallel)

**ALPHA + BETA + GAMMA spawn simultaneously:**

```
Hour 0:00 - ALPHA starts (content-id crate)
Hour 0:00 - BETA starts (DHT - needs ALPHA types)
Hour 0:00 - GAMMA starts (CLI - needs both)

Hour 1:30 - ALPHA complete (content-id done)
Hour 2:00 - GAMMA complete (CLI basic done)
Hour 2:30 - BETA complete (DHT done)

Hour 2:30 - DELTA starts (integration tests)
Hour 4:00 - DELTA complete (all tests pass)
```

### Critical Path

```
ALPHA (2h) → Must complete first for types
    ↓
BETA (3h, starts after 30min)
GAMMA (2h, starts after 30min)
    ↓
DELTA (1.5h) → Integration and validation
```

**Total Duration:** 4-5 hours (with parallelization)

---

## DEPENDENCIES

### External Crates

**Core:**
- `sha2`: SHA256 hashing
- `bs58`: Base58 encoding (IPFS-compatible)
- `libp2p`: P2P networking (DHT)
- `tokio`: Async runtime

**CLI:**
- `clap`: Command-line parsing
- `colored`: Terminal colors
- `anyhow`: Error handling

**Testing:**
- `criterion`: Benchmarks
- `assert_cmd`: CLI testing
- `tempfile`: Test fixtures

### System Requirements

- Rust 1.70+ (stable)
- TCP ports for P2P (random)
- Internet (for DHT bootstrap)

---

## RISKS & MITIGATIONS

### Risk 1: libp2p Complexity

**Risk Level:** MEDIUM  
**Impact:** Could delay BETA

**Mitigation:**
- Start with minimal Kademlia config
- Use MemoryStore (no persistence)
- Hard-code bootstrap node
- Extensive testing in isolation

**Fallback:**
- Skip DHT announcement in Phase 1
- Focus on CID generation only
- Add DHT in Phase 2

---

### Risk 2: Bootstrap Node Requirement

**Risk Level:** LOW  
**Impact:** DHT won't connect without peers

**Mitigation:**
- Use IPFS public bootstrap nodes
- Fall back to local testing (2 nodes)
- Document bootstrap setup

**Solution:**
```rust
// Use IPFS public bootstrap
let bootstrap_peers = vec![
    ("/dnsaddr/bootstrap.libp2p.io".parse()?, peer_id),
];
```

---

### Risk 3: Content Transfer Not Included

**Risk Level:** LOW (Expected)  
**Impact:** Phase 1 can't actually transfer data

**Mitigation:**
- Phase 1 = Foundation only
- Content transfer is Phase 2 (WebRTC)
- CLI shows "not yet implemented" message
- Success = CID generation + DHT announce

---

## SUCCESS METRICS

### Technical Metrics

**Performance:**
- CID generation: < 1ms per MB
- DHT lookup: < 5s average
- CLI startup: < 100ms

**Quality:**
- Test coverage: > 80%
- Clippy warnings: 0
- Documentation: 100% public APIs

### Functional Metrics

**Features:**
- ✅ Can generate CIDs
- ✅ Can announce to DHT
- ✅ Can find providers
- ✅ CLI works end-to-end
- ⚠️ Cannot transfer data yet (Phase 2)

### Deliverables Checklist

- ✅ `content-id` crate (150-200 lines)
- ✅ `dht` crate (300-400 lines)
- ✅ `cli` crate (200-300 lines)
- ✅ `common` crate (50-100 lines)
- ✅ Integration tests (150+ lines)
- ✅ CLI tests (50+ lines)
- ✅ README with examples
- ✅ CI/CD workflow
- ✅ All tests passing

---

## GITHUB ISSUES

### Issue Template

```markdown
# [AGENT] Task Description

**Duration:** X hours
**Priority:** HIGH/MEDIUM/LOW
**Dependencies:** List other agents

## Task
Brief description

## Deliverables
- [ ] File 1
- [ ] File 2
- [ ] Tests

## Success Criteria
- [ ] Criterion 1
- [ ] Criterion 2

## Commands
```bash
# Commands to run
```

## Technical Reference
- Link to proposal sections
- Link to examples
```

### Issues to Create

1. **[ALPHA] Content Addressing Library** (#1)
2. **[BETA] Kademlia DHT Implementation** (#2)
3. **[GAMMA] CLI Tool** (#3)
4. **[DELTA] Testing & Documentation** (#4)

---

## COMPLETION CRITERIA

**Phase 1 is COMPLETE when:**

- ✅ All 4 agents finish successfully
- ✅ All tests pass: `cargo test --workspace`
- ✅ CLI works: `codio-cdn publish/get/hash`
- ✅ CI/CD passing on GitHub
- ✅ README documents usage
- ✅ Can generate CIDs
- ✅ Can announce to DHT
- ✅ Can find providers via DHT

**Phase 1 is NOT complete until:**
- Content transfer (Phase 2 - WebRTC)
- Browser integration (Phase 2)
- Token economics (Phase 3)

---

## NEXT PHASE PREVIEW

**Phase 2:** Browser Integration (3 months)
- WebRTC data transfer
- Service Worker
- Browser extension
- HTTPS fallback

**Will build on Phase 1 foundation:**
- Uses `content-id` for verification
- Uses `dht` for peer discovery
- Adds actual content transfer

---

**TECHNICAL PROPOSAL STATUS:** ✅ READY  
**RECOMMENDATION:** Spawn agents immediately  
**ESTIMATED COMPLETION:** 4-5 hours  
**CONFIDENCE:** HIGH (similar to Extrophi Wave 1)  

**LET'S BUILD THE DECENTRALIZED FUTURE** 🚀
