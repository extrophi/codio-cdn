# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview
**Codio Decentralized CDN** - Phase 1 Foundation

Peer-to-peer content delivery network using content addressing (CIDs), Kademlia DHT for peer discovery, and economic incentives via $BANDWIDTH tokens.

**Repository**: https://github.com/Iamcodio/codio-cdn
**Phase**: 1 (Foundation) - 4-5 hours
**Tech Stack**: Rust, libp2p, WASM

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

## Common Commands

```bash
# Build workspace
cargo build --workspace

# Run tests (all)
cargo test --workspace --verbose

# Run tests (specific crate)
cargo test -p codio-content-id
cargo test -p codio-dht
cargo test -p codio-cdn

# Run CLI
cargo run -p codio-cdn -- hash <file>
cargo run -p codio-cdn -- publish <file>
cargo run -p codio-cdn -- get <cid>

# Linting and formatting
cargo clippy --workspace -- -D warnings
cargo fmt --all

# Build release
cargo build --release -p codio-cdn

# Benchmarks
cargo bench -p codio-content-id
```

## Development Environment
- **Rust**: 1.70+ (stable) - Use `rustup` for version management
- **Node.js** (future phases): Use `nvm` for version management (not Homebrew)
- **Python** (if needed): Use `uv` for version management (not Homebrew)

## Key Concepts

**Content Addressing (CID)**:
- Format: `Qm<base58(sha256(content))>`
- IPFS CIDv0 compatible
- Immutable, verifiable, deduplicating

**Kademlia DHT**:
- Peer discovery protocol
- Content announcement (start_providing)
- Provider lookup (get_providers)
- O(log N) scalability

**Phase 1 Scope**:
- ✅ Content addressing library
- ✅ DHT peer discovery
- ✅ CLI tool (publish/get/hash)
- ✅ Test suite (>80% coverage)
- ❌ WebRTC transfer (Phase 2)
- ❌ Browser integration (Phase 2)
- ❌ Token economics (Phase 3)

## Agent Assignments

**ALPHA** (Issue #1): Content addressing crate (2h)
**BETA** (Issue #2): Kademlia DHT crate (3h)
**GAMMA** (Issue #3): CLI tool (2h)
**DELTA** (Issue #4): Testing & docs (1.5h)

See `/docs/pm/TECHNICAL-PROPOSAL-PHASE-1.md` for detailed specifications.
