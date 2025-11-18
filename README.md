# Codio CDN - Phase 1 Foundation

Decentralized content delivery network using content addressing (CIDs), Kademlia DHT for peer discovery, and economic incentives via $BANDWIDTH tokens.

## Quick Start

```bash
# Build the workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Hash a file (offline)
cargo run -p codio-cdn -- hash README.md

# Publish content (with DHT announcement)
cargo run -p codio-cdn -- publish myfile.txt

# Retrieve by CID (Phase 2 - not yet implemented)
cargo run -p codio-cdn -- get QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG
```

## Architecture

```
codio-cdn/
├── crates/
│   ├── content-id/     # SHA256-based CID generation (IPFS-compatible)
│   ├── dht/            # Kademlia DHT for peer discovery (libp2p)
│   ├── cli/            # CLI tool (publish/get/hash commands)
│   └── common/         # Shared types (ContentId, PeerId, etc.)
├── tests/              # Integration tests
└── docs/pm/            # Technical proposals and PRDs
```

## Development

```bash
# Run tests
cargo test --workspace --verbose

# Run tests for specific crate
cargo test -p codio-content-id

# Linting and formatting
cargo clippy --workspace -- -D warnings
cargo fmt --all

# Build release
cargo build --release -p codio-cdn

# Install CLI
cargo install --path crates/cli
```

## How It Works

1. **Content Addressing**: Content is hashed with SHA256 to generate a CID in IPFS CIDv0 format: `Qm<base58(sha256(content))>`
2. **DHT Announcement**: Content availability is announced to the Kademlia DHT for peer discovery
3. **Provider Lookup**: Peers can find content providers via DHT queries
4. **Content Transfer**: (Phase 2 - WebRTC transfer not yet implemented)

## Phase 1 Status

**Current Phase**: Foundation ✅

- ✅ Content addressing (SHA256 + Base58)
- ✅ IPFS CIDv0 compatible format
- ✅ Kademlia DHT peer discovery
- ✅ CLI tool (hash, publish commands)
- ✅ Test suite (>80% coverage)
- ⚠️  Content transfer (Phase 2 - WebRTC)
- ⚠️  Browser integration (Phase 2)
- ⚠️  Token economics (Phase 3)

## CLI Commands

### Hash Command

Generate CID for content without publishing:

```bash
cargo run -p codio-cdn -- hash <file>
```

Example:
```bash
$ cargo run -p codio-cdn -- hash README.md
Hashing: README.md
CID: QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG
Size: 1234 bytes
```

### Publish Command

Publish content and announce to DHT:

```bash
cargo run -p codio-cdn -- publish <file>
```

Example:
```bash
$ cargo run -p codio-cdn -- publish myfile.txt
Publishing content...
  ✓ Read 1234 bytes
  ✓ Generated CID: QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG
Announcing to DHT...
  ✓ Content announced

Success!
CID: QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG
```

### Get Command

Retrieve content by CID (not yet fully implemented):

```bash
cargo run -p codio-cdn -- get <cid>
```

## Next Steps (Phase 2)

- WebRTC data transfer between peers
- Service Worker for browser integration
- Browser extension
- HTTPS fallback
- Token economics (Phase 3)

## Technical Reference

See `/docs/pm/TECHNICAL-PROPOSAL-PHASE-1.md` for complete technical specifications.

## License

MIT
