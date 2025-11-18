# codio-chunk

BitTorrent-style chunk distribution for the Codio decentralized CDN.

## Overview

This crate implements efficient peer-to-peer content distribution using proven BitTorrent strategies:

- **Parallel Downloads**: Download from multiple peers simultaneously
- **Rarest-First Strategy**: Download rare chunks first to improve network-wide availability
- **Tit-for-Tat**: Reward peers who upload to us by prioritizing uploads to them
- **Chunk Verification**: SHA-256 verification of every chunk
- **Automatic Failover**: Retry failed chunks from different peers

## Architecture

```
┌─────────────────────────────────────────────┐
│         ChunkDistributor                    │
│  ┌──────────────────────────────────────┐   │
│  │  Download Manager                    │   │
│  │  - Active downloads                  │   │
│  │  - Chunk availability tracking       │   │
│  │  - Parallel request coordination     │   │
│  └──────────────────────────────────────┘   │
│  ┌──────────────────────────────────────┐   │
│  │  Upload Manager                      │   │
│  │  - Active uploads                    │   │
│  │  - Tit-for-tat tracking             │   │
│  │  - Peer statistics                   │   │
│  └──────────────────────────────────────┘   │
│  ┌──────────────────────────────────────┐   │
│  │  Strategy Engine                     │   │
│  │  - Rarest-first                      │   │
│  │  - Sequential                        │   │
│  │  - Random order                      │   │
│  └──────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

## BitTorrent Strategies

### Rarest-First

The rarest-first strategy downloads chunks that are least common across the network first. This has several benefits:

1. **Network Resilience**: Ensures rare chunks spread quickly before seeders leave
2. **Load Balancing**: Distributes load across peers naturally
3. **Fast Distribution**: New peers get rare chunks quickly, becoming useful seeders sooner

**How it works:**
- Query all peers for chunk availability
- Build a rarity map (chunk_index -> count of peers)
- Always download the rarest chunk next
- As chunks spread, rarity map updates dynamically

### Tit-for-Tat

Tit-for-tat is a reciprocal strategy that rewards peers who upload to us:

1. **Track Ratios**: Monitor upload/download ratio for each peer
2. **Prefer Generous Peers**: Prioritize downloading from peers with good ratios
3. **Optimistic Unchoking**: Randomly give new peers a chance every 30 seconds
4. **Fair Sharing**: Incentivizes uploading, prevents free-riding

**Benefits:**
- Encourages all peers to upload
- Creates a healthy sharing ecosystem
- Prevents bandwidth exploitation
- Self-regulating network behavior

### Chunk Verification

Every chunk is verified using SHA-256:

```rust
let chunk = download_chunk(index, peer).await?;

if !chunk.verify() {
    // Discard corrupted chunk
    // Re-request from different peer
    retry_chunk(index).await?;
}
```

This prevents:
- Malicious peer attacks
- Network corruption
- Incomplete transfers

## Usage

### Basic Download

```rust
use codio_chunk::{ChunkDistributor, ChunkConfig};
use codio_content_id::ContentId;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create distributor with default config
    let mut distributor = ChunkDistributor::new(ChunkConfig::default()).await?;

    // Get content from network
    let cid = ContentId::from_str("Qm...")?;
    let providers = vec![/* peer list from DHT */];

    let content = distributor.download_content(cid, providers).await?;

    println!("Downloaded {} bytes", content.len());
    Ok(())
}
```

### Custom Configuration

```rust
use codio_chunk::{ChunkConfig, DistributionStrategy};
use std::time::Duration;

let config = ChunkConfig::builder()
    .max_concurrent_downloads(20)
    .chunks_per_peer(8)
    .request_timeout(Duration::from_secs(60))
    .strategy(DistributionStrategy::RarestFirst)
    .chunk_size(512 * 1024) // 512KB chunks
    .build();

let mut distributor = ChunkDistributor::new(config).await?;
```

### Monitor Progress

```rust
// Start download in background
let cid_clone = cid.clone();
tokio::spawn(async move {
    distributor.download_content(cid_clone, providers).await
});

// Monitor progress
loop {
    if let Some(progress) = distributor.download_progress(&cid).await {
        println!(
            "Progress: {:.1}% ({} / {} bytes) - {:.2} KB/s - {} peers",
            progress.progress * 100.0,
            progress.downloaded_bytes,
            progress.total_bytes,
            progress.download_rate / 1024.0,
            progress.peers
        );

        if let Some(eta) = progress.eta {
            println!("ETA: {:?}", eta);
        }
    }

    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

### Serve Content

```rust
// Store content locally
let content = std::fs::read("video.mp4")?;
let cid = ContentId::new(&content);
distributor.store_content(cid.clone(), content).await?;

// Serve chunks to requesting peers
let peer = get_requesting_peer();
let chunk = distributor.serve_chunk(cid, chunk_index, peer).await?;
```

## Configuration Options

| Option                      | Default    | Description                                |
| --------------------------- | ---------- | ------------------------------------------ |
| `max_concurrent_downloads`  | 10         | Max concurrent content downloads           |
| `chunks_per_peer`           | 4          | Max concurrent chunks per peer             |
| `request_timeout`           | 30s        | Timeout for individual chunk requests      |
| `strategy`                  | RarestFirst| Download strategy                          |
| `chunk_size`                | 256KB      | Size of each chunk                         |
| `optimistic_unchoke_interval` | 30s      | How often to try random peers              |
| `min_upload_ratio`          | 0.8        | Minimum upload ratio before preferring peer|

## Distribution Strategies

### RarestFirst (Recommended)

Best for most use cases. Prioritizes network health and fast distribution.

```rust
.strategy(DistributionStrategy::RarestFirst)
```

### Sequential

Downloads chunks in order. Useful for streaming media where you need data sequentially.

```rust
.strategy(DistributionStrategy::Sequential)
```

### RandomOrder

Random chunk selection. Provides some load balancing without rarest-first complexity.

```rust
.strategy(DistributionStrategy::RandomOrder)
```

## Performance Tuning

### High-Speed Networks

```rust
ChunkConfig::builder()
    .max_concurrent_downloads(50)
    .chunks_per_peer(16)
    .chunk_size(1024 * 1024) // 1MB chunks
    .build()
```

### Slow/Unreliable Networks

```rust
ChunkConfig::builder()
    .max_concurrent_downloads(5)
    .chunks_per_peer(2)
    .request_timeout(Duration::from_secs(120))
    .chunk_size(128 * 1024) // 128KB chunks
    .build()
```

### Streaming Media

```rust
ChunkConfig::builder()
    .strategy(DistributionStrategy::Sequential)
    .chunks_per_peer(8)
    .chunk_size(256 * 1024)
    .build()
```

## Testing

```bash
# Run all tests
cargo test -p codio-chunk

# Run integration tests only
cargo test -p codio-chunk --test integration_test

# Run with output
cargo test -p codio-chunk -- --nocapture

# Run specific test
cargo test -p codio-chunk test_parallel_download
```

## Examples

See the `tests/` directory for comprehensive examples:

- `test_parallel_download` - Download from multiple peers
- `test_rarest_first` - Rarest-first strategy in action
- `test_chunk_verification` - SHA-256 verification
- `test_tit_for_tat` - Peer reciprocity
- `test_large_content` - Handling large files

## Integration

### With DHT

```rust
use codio_dht::DhtNode;

// Find providers using DHT
let (mut dht, mut events) = DhtNode::new(config).await?;
dht.find_providers(cid.clone()).await?;

// Wait for providers
let providers = match events.recv().await {
    Some(DhtEvent::ProvidersFound { providers, .. }) => providers,
    _ => vec![],
};

// Download using chunk distributor
let content = distributor.download_content(cid, providers).await?;
```

### With CLI

```rust
// In your CLI tool
let mut distributor = ChunkDistributor::new(config).await?;

match args.command {
    Command::Get { cid } => {
        let providers = query_dht(&cid).await?;
        let content = distributor.download_content(cid, providers).await?;
        std::fs::write("output.dat", content)?;
    }
}
```

## Benchmarks

Run benchmarks to test performance:

```bash
cargo bench -p codio-chunk
```

Typical results on modern hardware:
- Single peer: ~50 MB/s
- 5 peers: ~200 MB/s
- 10 peers: ~350 MB/s

## Technical Constraints

- **Maximum 4 chunk requests per peer**: Prevents overwhelming any single peer
- **30 second timeout per chunk**: Ensures failed requests don't hang
- **SHA-256 verification**: Every chunk is verified before acceptance
- **No unsafe code**: 100% safe Rust
- **Async throughout**: Fully async/await based for maximum performance

## References

- [BitTorrent Specification (BEP 3)](http://bittorrent.org/beps/bep_0003.html)
- [BitTorrent Economics Paper](https://www.bittorrent.org/bittorrentecon.pdf)
- [Codio Technical Proposal](../../docs/pm/TECHNICAL-PROPOSAL-PHASE-1.md)

## License

MIT OR Apache-2.0
