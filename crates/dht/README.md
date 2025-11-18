# Codio DHT - Kademlia Distributed Hash Table

A Kademlia DHT implementation for the Codio decentralized CDN, providing peer discovery and content routing capabilities.

## Overview

This crate implements a Kademlia Distributed Hash Table (DHT) for discovering peers and routing content requests in a peer-to-peer network. It uses libp2p's Kademlia implementation with custom provider tracking and management.

## Features

- **Content Routing**: Announce and discover content providers
- **Peer Discovery**: Find and connect to peers in the network
- **XOR Distance Metric**: Efficient routing based on key similarity
- **Provider Records**: Track content availability with expiration
- **Auto-Republishing**: Automatically refresh provider announcements
- **Configurable**: Customizable timeouts, replication factors, and more
- **Statistics**: Monitor DHT health and performance

## Architecture

### Kademlia Protocol

The DHT uses the Kademlia protocol, which provides:

1. **XOR Distance Metric**: Measures "closeness" between node IDs and content keys
2. **K-Buckets**: Organized routing table with up to k peers per bucket
3. **Iterative Lookups**: O(log N) time complexity for searches
4. **Redundancy**: Content announcements replicated to k closest nodes

### Key Concepts

#### XOR Distance

Kademlia uses XOR distance to determine which nodes are "close" to a piece of content:

```rust
distance = node_id XOR content_key
```

Nodes store information about content whose keys are close to their own ID.

#### Provider Records

When a node has content, it announces itself as a provider to the k closest nodes (default: 20). Other nodes can then query the DHT to find providers.

Provider records expire after 24 hours by default and are automatically republished every 12 hours.

#### Routing Table

The routing table is organized into k-buckets, where each bucket contains peers at a specific XOR distance range. This enables efficient lookups with O(log N) complexity.

## Usage

### Basic Example

```rust
use codio_dht::{DHTManager, DHTConfig};
use codio_content_id::ContentId;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create DHT manager
    let config = DHTConfig::default();
    let mut dht = DHTManager::new(config).await?;

    // Start listening
    dht.listen("/ip4/0.0.0.0/tcp/4001".parse()?).await?;

    // Announce content
    let cid = ContentId::new(b"Hello, world!");
    dht.provide(cid.clone()).await?;

    // Find providers
    let providers = dht.find_providers(cid).await?;
    println!("Found {} providers", providers.len());

    Ok(())
}
```

### Configuration

Customize DHT behavior with `DHTConfig`:

```rust
use std::time::Duration;
use codio_dht::DHTConfig;

let config = DHTConfig::new()
    .with_replication_factor(30)           // Replicate to 30 nodes
    .with_provider_timeout(Duration::from_secs(48 * 3600))  // 48 hour TTL
    .with_republish_interval(Duration::from_secs(24 * 3600)) // Republish daily
    .with_query_timeout(Duration::from_secs(120))  // 2 minute timeout
    .with_k_value(25)                      // 25 peers per k-bucket
    .with_parallelism(15)                  // 15 parallel queries
    .without_auto_republish();             // Disable auto-republish

let mut dht = DHTManager::new(config).await?;
```

### Bootstrap

Connect to the network using bootstrap nodes:

```rust
let bootstrap_addrs = vec![
    "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooW...".parse()?,
    "/ip4/192.168.1.100/tcp/4001/p2p/12D3KooW...".parse()?,
];

dht.bootstrap(bootstrap_addrs).await?;
```

### Event Handling

Monitor DHT events:

```rust
let mut event_rx = dht.take_event_receiver().unwrap();

tokio::spawn(async move {
    while let Some(event) = event_rx.recv().await {
        match event {
            DHTEvent::ProvidersFound { cid, providers } => {
                println!("Found {} providers for {}", providers.len(), cid);
            }
            DHTEvent::ProvideSuccess { cid } => {
                println!("Successfully announced {}", cid);
            }
            DHTEvent::BootstrapComplete { num_peers } => {
                println!("Bootstrap complete with {} peers", num_peers);
            }
            _ => {}
        }
    }
});
```

### Running the Event Loop

The DHT requires continuous event processing:

```rust
tokio::spawn(async move {
    loop {
        dht.run_event_loop().await;
    }
});
```

### Statistics

Monitor DHT health and performance:

```rust
let stats = dht.stats();

println!("Peers: {}", stats.num_peers);
println!("Providers: {}", stats.num_providers);
println!("Local content: {}", stats.local_content_count);
println!("Success rate: {:.2}%", stats.success_rate() * 100.0);
println!("Bootstrapped: {}", stats.is_bootstrapped);
```

## API Reference

### DHTManager

Main DHT manager for content routing and peer discovery.

#### Methods

- `new(config: DHTConfig) -> Result<Self>` - Create new DHT manager
- `peer_id() -> &PeerId` - Get local peer ID
- `listen(addr: Multiaddr) -> Result<()>` - Start listening on address
- `bootstrap(peers: Vec<Multiaddr>) -> Result<()>` - Bootstrap DHT
- `provide(cid: ContentId) -> Result<()>` - Announce content
- `find_providers(cid: ContentId) -> Result<Vec<PeerInfo>>` - Find providers
- `stop_providing(cid: &ContentId) -> Result<()>` - Stop providing content
- `find_peer(peer_id: PeerId) -> Result<Vec<Multiaddr>>` - Find peer addresses
- `stats() -> DHTStats` - Get DHT statistics
- `run_event_loop() -> ()` - Process DHT events (call continuously)

### DHTConfig

Configuration for DHT behavior.

#### Default Values

- `replication_factor`: 20
- `provider_timeout`: 24 hours
- `republish_interval`: 12 hours
- `query_timeout`: 60 seconds
- `k_value`: 20
- `parallelism`: 10
- `auto_republish`: true
- `max_local_providers`: 10,000

### DHTStats

Statistics about DHT state and performance.

#### Fields

- `num_peers`: Number of peers in routing table
- `num_providers`: Provider records being tracked
- `routing_table_size`: Number of k-buckets
- `pending_queries`: Active queries
- `local_content_count`: Content items we're providing
- `total_queries`: Total queries executed
- `successful_queries`: Successful queries
- `failed_queries`: Failed queries
- `last_bootstrap`: Last bootstrap time
- `is_bootstrapped`: Bootstrap status

#### Methods

- `success_rate() -> f64` - Calculate query success rate (0.0 to 1.0)

### DHTEvent

Events emitted by the DHT.

#### Variants

- `ProvidersFound { cid, providers }` - Providers found for content
- `ProvideSuccess { cid }` - Content announcement successful
- `ProvideFailed { cid, error }` - Content announcement failed
- `BootstrapComplete { num_peers }` - Bootstrap completed
- `BootstrapFailed { error }` - Bootstrap failed
- `PeerDiscovered { peer }` - New peer discovered
- `PeerAddressesFound { peer_id, addresses }` - Peer addresses found
- `QueryCompleted { query_type, success }` - Query completed
- `ProviderExpired { cid, provider }` - Provider record expired
- `RoutingTableUpdated { num_peers }` - Routing table updated

## Performance Characteristics

- **Lookup Time**: O(log N) where N is network size
- **Storage**: O(k * log N) for routing table
- **Bandwidth**: Minimal with exponential backoff
- **Scalability**: Tested with networks of 10,000+ nodes

## Testing

Run all tests:

```bash
cargo test -p codio-dht
```

Run integration tests:

```bash
cargo test -p codio-dht --test integration_test
```

Run specific test:

```bash
cargo test -p codio-dht test_provide_content
```

## Implementation Details

### Provider Record Lifecycle

1. **Announcement**: Node calls `provide(cid)` to announce content
2. **Replication**: Record replicated to k closest nodes (default: 20)
3. **Expiration**: Records expire after `provider_timeout` (default: 24h)
4. **Republishing**: Auto-republished every `republish_interval` (default: 12h)

### Query Process

1. Node initiates query (e.g., `find_providers`)
2. DHT performs iterative lookup to k closest nodes
3. Results aggregated and returned to caller
4. Query tracked in `active_queries` until completion or timeout

### Routing Table Maintenance

- Automatically updated when peers connect/disconnect
- Periodic maintenance via `maintenance_interval`
- K-buckets organized by XOR distance ranges
- LRU eviction when buckets are full

## Contributing

See the main [Codio CDN repository](https://github.com/Iamcodio/codio-cdn) for contribution guidelines.

## Resources

- [Kademlia Paper](https://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf)
- [libp2p Kademlia](https://docs.libp2p.io/concepts/kad-dht/)
- [Codio CDN Documentation](../../docs/pm/TECHNICAL-PROPOSAL-PHASE-1.md)

## License

MIT
