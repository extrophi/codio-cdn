# Codio Network

**P2P Network Layer for Codio CDN**

This crate provides the foundational peer-to-peer networking layer for the Codio CDN, built on top of [libp2p](https://libp2p.io/). It handles peer discovery, connection management, transport encryption, and network protocols.

## Features

- 🔍 **Peer Discovery**
  - Local network discovery via mDNS
  - Global peer discovery via Kademlia DHT
  - Bootstrap node support for initial network entry

- 🔗 **Connection Management**
  - Automatic reconnection on failure
  - Configurable peer limits
  - Connection health monitoring
  - Clean disconnect handling

- 🔒 **Transport Security**
  - Noise protocol for encryption
  - Yamux for connection multiplexing
  - TCP transport layer

- 🌐 **Protocol Support**
  - Kademlia DHT for content routing
  - Ping for keep-alive and latency measurement
  - Identify for peer metadata exchange
  - Custom Codio protocol (`/codio/1.0.0`)

## Architecture

```
┌─────────────────────────────────────────────┐
│         NetworkManager (High-level API)     │
├─────────────────────────────────────────────┤
│  libp2p Swarm (Connection Management)       │
├─────────────────────────────────────────────┤
│  NetworkBehaviour (Protocol Layer)          │
│  ├─ mDNS (Local Discovery)                  │
│  ├─ Kademlia (DHT & Content Routing)        │
│  ├─ Ping (Keep-alive)                       │
│  └─ Identify (Peer Metadata)                │
├─────────────────────────────────────────────┤
│  Transport Layer                            │
│  ├─ TCP (Base Transport)                    │
│  ├─ Noise (Encryption)                      │
│  └─ Yamux (Multiplexing)                    │
└─────────────────────────────────────────────┘
```

## Usage

### Basic Setup

```rust
use codio_network::{NetworkManager, NetworkConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create network with default configuration
    let config = NetworkConfig::default();
    let mut network = NetworkManager::new(config).await?;

    // Start listening for connections
    network.start().await?;

    // Run the event loop
    network.run().await?;

    Ok(())
}
```

### Custom Configuration

```rust
use codio_network::NetworkConfig;

let config = NetworkConfig::new(4001, 100)
    .with_bootstrap_peer("/ip4/1.2.3.4/tcp/4001/p2p/QmBootstrap".parse()?)
    .without_mdns()
    .with_kad_replication_factor(30);
```

### Connecting to Peers

```rust
// Connect to a specific peer
let peer_addr = "/ip4/127.0.0.1/tcp/4001/p2p/QmPeerId".parse()?;
let peer_id = network.connect_peer(peer_addr).await?;

// Disconnect from a peer
network.disconnect_peer(peer_id).await?;
```

### Monitoring Connections

```rust
// Get peer count
let count = network.peer_count();

// Get detailed peer information
let peers = network.connected_peers();
let peers_lock = peers.read().unwrap();

for (peer_id, peer_info) in peers_lock.iter() {
    println!("Peer: {}", peer_id);
    println!("  Connected for: {:?}", peer_info.connection_duration());
    println!("  Last seen: {:?} ago", peer_info.idle_duration());
    println!("  Addresses: {:?}", peer_info.addresses);
}
```

### Content Routing (DHT)

```rust
use libp2p::kad::RecordKey;

// Announce that we're providing content
let key = RecordKey::new(&b"my-content-key");
network.start_providing(key.clone())?;

// Find providers for content
let query_id = network.get_providers(key);
```

## Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `listen_port` | `0` (random) | Port to listen on |
| `max_peers` | `50` | Maximum concurrent connections |
| `enable_mdns` | `true` | Enable local network discovery |
| `enable_relay` | `true` | Enable NAT traversal relay |
| `kad_replication_factor` | `20` | Kademlia replication factor |
| `connection_idle_timeout_secs` | `300` | Connection timeout (5 min) |
| `bootstrap_peers` | `[]` | Initial peers to connect to |

## Testing

Run the test suite:

```bash
cargo test -p codio-network
```

Run with logging:

```bash
RUST_LOG=debug cargo test -p codio-network -- --nocapture
```

## Examples

### Three-Peer Network

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create three peers
    let mut peer1 = NetworkManager::new(NetworkConfig::default()).await?;
    let mut peer2 = NetworkManager::new(NetworkConfig::default()).await?;
    let mut peer3 = NetworkManager::new(NetworkConfig::default()).await?;

    // Start all peers
    peer1.start().await?;
    peer2.start().await?;
    peer3.start().await?;

    // They will discover each other via mDNS
    // Run event loops in separate tasks
    let handle1 = tokio::spawn(async move { peer1.run().await });
    let handle2 = tokio::spawn(async move { peer2.run().await });
    let handle3 = tokio::spawn(async move { peer3.run().await });

    // Wait for all to complete
    tokio::try_join!(handle1, handle2, handle3)?;

    Ok(())
}
```

## Dependencies

- **libp2p 0.53+**: Core P2P networking library
- **tokio 1.35+**: Async runtime
- **anyhow**: Error handling
- **tracing**: Structured logging

## Performance Considerations

- **Connection Limits**: Adjust `max_peers` based on available resources
- **mDNS**: Disable on production networks to reduce broadcast traffic
- **Kademlia**: Higher replication factors increase redundancy but use more bandwidth
- **Event Processing**: The `process_event()` method should be called frequently in a loop

## Debugging

Enable debug logging:

```bash
RUST_LOG=codio_network=debug cargo run
```

View all libp2p events:

```bash
RUST_LOG=libp2p=debug cargo run
```

## Known Limitations

- mDNS discovery may not work reliably in all network environments (especially Docker/VMs)
- NAT traversal requires relay nodes for optimal connectivity
- Bootstrap nodes are required for initial DHT connectivity
- Connection establishment may take several seconds

## Contributing

When contributing to this crate:

1. Ensure all tests pass: `cargo test -p codio-network`
2. Run clippy: `cargo clippy -p codio-network -- -D warnings`
3. Format code: `cargo fmt --all`
4. Add tests for new functionality
5. Update documentation

## License

MIT

## Resources

- [libp2p Documentation](https://docs.rs/libp2p/)
- [libp2p Specs](https://github.com/libp2p/specs)
- [Kademlia DHT](https://en.wikipedia.org/wiki/Kademlia)
- [Noise Protocol](https://noiseprotocol.org/)
