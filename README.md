# Codio CDN - Phase 1 Foundation

> Decentralized content delivery using content addressing and DHT

Codio CDN is a peer-to-peer content delivery network built on content addressing (CID), Kademlia DHT for peer discovery, and designed for future economic incentives via $BANDWIDTH tokens.

## Quick Start

```bash
# Build the project
cargo build --workspace

# Hash a file (no network activity)
cargo run -p codio-cdn -- hash README.md

# Publish content (generates CID and announces to DHT)
cargo run -p codio-cdn -- publish myfile.txt

# Retrieve by CID (finds providers via DHT)
cargo run -p codio-cdn -- get QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG
```

## Features

### Phase 1 (Current) - Foundation

- ✅ **Content Addressing**: SHA256-based CID generation (IPFS CIDv0 compatible)
- ✅ **Peer Discovery**: Kademlia DHT for finding content providers
- ✅ **CLI Tool**: Simple command-line interface for publish/get/hash operations
- ✅ **Test Suite**: Comprehensive tests with >80% coverage
- ⚠️ **Content Transfer**: Not yet implemented (Phase 2)

### Future Phases

- **Phase 2** (Browser Integration): WebRTC transfer, Service Worker, browser extension
- **Phase 3** (Token Economics): $BANDWIDTH token, staking, bandwidth markets

## Architecture

```
codio-cdn/
├── crates/
│   ├── content-id/     # SHA256-based CID generation
│   ├── common/         # Shared types (ContentId, PeerId, etc.)
│   ├── dht/            # Kademlia DHT for peer discovery
│   └── cli/            # CLI tool (publish/get/hash commands)
├── tests/              # Integration and CLI tests
└── docs/               # Technical documentation
```

### Component Overview

#### Content-ID Crate

Provides content-addressed storage using SHA256 hashing:

```rust
use codio_content_id::ContentId;

// Generate CID from content
let content = b"Hello, decentralized world!";
let cid = ContentId::new(content);
println!("CID: {}", cid); // CID: Qm...

// Verify content matches CID
assert!(cid.verify(content));

// Parse CID from string
let cid2 = ContentId::from_str("QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG")?;
```

**Key Properties:**
- Format: `Qm<base58(sha256(content))>`
- IPFS CIDv0 compatible
- Deterministic: same content = same CID
- Immutable: content changes = different CID

#### DHT Crate

Implements Kademlia DHT for peer discovery using libp2p:

```rust
use codio_dht::{DhtNode, DhtConfig};

// Create DHT node
let config = DhtConfig::default();
let (mut node, mut events) = DhtNode::new(config).await?;

// Listen for connections
node.listen("/ip4/0.0.0.0/tcp/4001".parse()?).await?;

// Announce content availability
node.provide(cid).await?;

// Find providers for content
node.find_providers(cid).await?;
```

**Features:**
- O(log N) peer discovery
- Content announcement (start_providing)
- Provider lookup (get_providers)
- Configurable timeouts and TTLs

#### CLI Tool

Command-line interface for content operations:

```bash
# Hash a file (local only)
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

Note: Content download not yet implemented
```

## Development

### Prerequisites

- Rust 1.70+ (stable)
- Cargo (comes with Rust)

### Building

```bash
# Build all crates
cargo build --workspace

# Build release version
cargo build --release --workspace

# Build specific crate
cargo build -p codio-content-id
cargo build -p codio-dht
cargo build -p codio-cdn
```

### Testing

```bash
# Run all tests
cargo test --workspace --verbose

# Run tests for specific crate
cargo test -p codio-content-id
cargo test -p codio-dht
cargo test -p codio-cdn

# Run integration tests only
cargo test --test integration_test
cargo test --test cli_test

# Run with coverage
cargo install cargo-tarpaulin
cargo tarpaulin --workspace --out Html
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy --workspace -- -D warnings

# Generate documentation
cargo doc --workspace --open
```

### Benchmarks

```bash
# Run benchmarks
cargo bench -p codio-content-id
```

## How It Works

### Content Addressing Flow

1. **Hash Content**: Content is hashed with SHA256
2. **Generate CID**: Hash is encoded as `Qm<base58(hash)>`
3. **Verify**: Same content always produces same CID
4. **Deduplication**: Identical content shares same CID

### Peer Discovery Flow

1. **Bootstrap**: Node connects to DHT network
2. **Announce**: Provider announces content availability via `start_providing`
3. **Discover**: Consumer searches for providers via `get_providers`
4. **Connect**: Consumer connects to provider (Phase 2)
5. **Transfer**: Content is transferred (Phase 2)

### Why Content Addressing?

**Traditional CDN:**
```
https://cdn.example.com/file.js?v=1.2.3
```
- Location-based (server location matters)
- Trust required (server can change content)
- Single point of failure

**Content-Addressed CDN:**
```
codio://QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG
```
- Content-based (retrieve from any peer)
- Trustless (CID verifies content integrity)
- Decentralized (no single point of failure)

## Performance Targets

### Phase 1 Metrics

- **CID Generation**: < 1ms per MB
- **CID Verification**: < 1ms per MB
- **DHT Lookup**: < 5s average
- **CLI Startup**: < 100ms
- **Test Coverage**: > 80%

## API Reference

### ContentId

```rust
impl ContentId {
    pub fn new(content: &[u8]) -> Self;
    pub fn verify(&self, content: &[u8]) -> bool;
    pub fn as_str(&self) -> &str;
    pub fn hash(&self) -> &[u8; 32];
    pub fn from_str(s: &str) -> Result<Self, CidError>;
}
```

### DhtNode

```rust
impl DhtNode {
    pub async fn new(config: DhtConfig) -> Result<(Self, Receiver<DhtEvent>)>;
    pub async fn listen(&mut self, addr: Multiaddr) -> Result<()>;
    pub async fn bootstrap(&mut self) -> Result<()>;
    pub async fn provide(&mut self, cid: ContentId) -> Result<()>;
    pub async fn find_providers(&mut self, cid: ContentId) -> Result<()>;
}
```

## Known Limitations

### Phase 1

- ❌ **No Content Transfer**: DHT only finds providers, doesn't transfer data
- ❌ **No Persistence**: Content not stored permanently
- ❌ **No Browser Support**: CLI only
- ❌ **No Economics**: No token incentives yet

### Workarounds

Use IPFS for actual content transfer:
```bash
# Hash with Codio
codio-cdn hash file.txt

# Publish to IPFS
ipfs add file.txt

# Compare CIDs (should match if same hash algorithm)
```

## Troubleshooting

### Tests Failing

```bash
# Clean and rebuild
cargo clean
cargo build --workspace
cargo test --workspace
```

### DHT Connection Issues

The DHT requires bootstrap peers to connect. Phase 1 uses local-only testing.

### Build Errors

```bash
# Update Rust
rustup update stable

# Check Rust version
rustc --version  # Should be 1.70+

# Update dependencies
cargo update
```

## Contributing

### Development Workflow

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make changes and add tests
4. Run tests: `cargo test --workspace`
5. Format code: `cargo fmt --all`
6. Run clippy: `cargo clippy --workspace -- -D warnings`
7. Commit: `git commit -m "feat: add my feature"`
8. Push: `git push origin feature/my-feature`
9. Open a pull request

### Code Standards

- All public APIs must have documentation
- All new features must have tests
- Code coverage must be > 80%
- No clippy warnings allowed
- Use `cargo fmt` for formatting

## License

MIT

## Roadmap

### Phase 1 ✅ (Current - Complete)
- [x] Content addressing (SHA256 + base58)
- [x] Kademlia DHT implementation
- [x] CLI tool (publish/get/hash)
- [x] Comprehensive test suite
- [x] CI/CD pipeline

### Phase 2 🚧 (3 months)
- [ ] WebRTC peer-to-peer transfer
- [ ] Service Worker for browser caching
- [ ] Browser extension
- [ ] HTTPS fallback for non-P2P browsers

### Phase 3 🔮 (6 months)
- [ ] $BANDWIDTH token economics
- [ ] Staking for providers
- [ ] Bandwidth marketplace
- [ ] Payment channels

## Resources

- [Technical Proposal](docs/pm/TECHNICAL-PROPOSAL-PHASE-1.md)
- [IPFS Specs](https://github.com/ipfs/specs)
- [Kademlia Paper](https://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf)
- [libp2p Documentation](https://docs.libp2p.io/)

## Support

For issues and questions:
- GitHub Issues: [https://github.com/Iamcodio/codio-cdn/issues](https://github.com/Iamcodio/codio-cdn/issues)
- Technical Proposal: `docs/pm/TECHNICAL-PROPOSAL-PHASE-1.md`

---

**Built with ❤️ by the Codio Collective**

**Let's build the decentralized future** 🚀
