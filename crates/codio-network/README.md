# Codio Network Layer

The P2P networking foundation for the Codio CDN, built on libp2p.

## Overview

This crate provides a robust peer-to-peer network layer with support for:

- **Peer Discovery**: Automatic discovery via mDNS (local) and Kademlia DHT (global)
- **Connection Management**: Automatic reconnection, connection limits, NAT traversal
- **Transport Security**: Noise protocol encryption and Yamux multiplexing
- **Protocol Support**: Custom `/codio/1.0.0` protocol, ping, and identify

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              NetworkManager (Public API)                │
├─────────────────────────────────────────────────────────┤
│  - Peer management                                      │
│  - Connection lifecycle                                 │
│  - Event distribution                                   │
└────────────────┬────────────────────────────────────────┘
                 │
┌────────────────▼────────────────────────────────────────┐
│           libp2p Swarm (Core Engine)                    │
├─────────────────────────────────────────────────────────┤
│  Transport: TCP + Noise + Yamux                         │
│  Protocols:                                             │
│    - mDNS: Local peer discovery                         │
│    - Kademlia: Global DHT routing                       │
│    - Ping: Keep-alive & latency                         │
│    - Identify: Peer metadata exchange                   │
└─────────────────────────────────────────────────────────┘
```

## Features

### Peer Discovery

**mDNS (Local Network)**
- Zero-configuration peer discovery on local networks
- Automatic connection to discovered peers (up to max limit)
- Multicast DNS for service advertisement

**Kademlia DHT (Global)**
- Distributed hash table for peer routing
- O(log N) lookup complexity
- Content provider advertisement and discovery
- Bootstrap node support

### Connection Management

- **Automatic Reconnection**: Handles transient network failures
- **Connection Limits**: Configurable maximum peer count
- **NAT Traversal**: Relay protocol support for restricted networks
- **Connection Pooling**: Efficient resource management
- **Timeout Handling**: Configurable connection and idle timeouts

### Security

- **Noise Protocol**: Encrypted transport layer (XX pattern)
- **Peer Authentication**: Ed25519 keypair-based identity
- **Replay Protection**: Built into Noise handshake
- **Forward Secrecy**: Ephemeral key exchange

### Observability

- **Event System**: Async event stream for network state changes
- **Peer Tracking**: Detailed peer information and statistics
- **Structured Logging**: `tracing` integration for debugging
- **Network Stats**: Connection counts, latency metrics

## Usage

### Basic Example

```rust
use codio_network::{NetworkManager, NetworkConfig, NetworkEvent};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create network manager
    let config = NetworkConfig::default();
    let mut manager = NetworkManager::new(config).await?;

    // Start listening
    manager.start().await?;

    println!("Peer ID: {}", manager.peer_id());
    println!("Listening on: {:?}", manager.listen_addrs());

    // Process events
    loop {
        if let Some(event) = manager.next_event().await {
            match event {
                NetworkEvent::PeerConnected { peer_id, address } => {
                    println!("Connected to {} at {}", peer_id, address);
                }
                NetworkEvent::PeerDisconnected { peer_id } => {
                    println!("Disconnected from {}", peer_id);
                }
                _ => {}
            }
        }
    }
}
```

### Custom Configuration

```rust
use codio_network::{NetworkConfig, NetworkManager};
use libp2p::Multiaddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut config = NetworkConfig::with_port(8080)
        .with_max_peers(100)
        .without_mdns(); // Disable local discovery

    // Add bootstrap peers
    let bootstrap: Multiaddr = "/ip4/1.2.3.4/tcp/8080/p2p/12D3KooW...".parse()?;
    config.add_bootstrap_peer(bootstrap);

    let mut manager = NetworkManager::new(config).await?;
    manager.start().await?;

    // Run event loop
    manager.run().await?;

    Ok(())
}
```

### Manual Peer Connection

```rust
use codio_network::{NetworkManager, NetworkConfig};
use libp2p::Multiaddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut manager = NetworkManager::new(NetworkConfig::default()).await?;
    manager.start().await?;

    // Connect to specific peer
    let peer_addr: Multiaddr = "/ip4/192.168.1.100/tcp/8080/p2p/12D3KooW...".parse()?;
    let peer_id = manager.connect_peer(peer_addr).await?;

    println!("Connected to peer: {}", peer_id);

    // Get peer info
    if let Some(info) = manager.connected_peers().get(&peer_id) {
        println!("Peer info: {:?}", info);
    }

    // Disconnect
    manager.disconnect_peer(peer_id).await?;

    Ok(())
}
```

### DHT Operations

```rust
use codio_network::{NetworkManager, NetworkConfig};
use libp2p::kad;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut manager = NetworkManager::new(NetworkConfig::default()).await?;
    manager.start().await?;

    // Announce content availability
    let content_key = kad::RecordKey::new(&b"my-content-hash");
    manager.start_providing(content_key.clone())?;

    // Find providers for content
    let query_id = manager.get_providers(content_key);

    // Process events to get results
    loop {
        if let Some(event) = manager.next_event().await {
            // Handle provider results
            println!("Event: {:?}", event);
        }
    }
}
```

## Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `listen_port` | `0` (random) | TCP port to listen on |
| `max_peers` | `50` | Maximum number of concurrent connections |
| `bootstrap_peers` | `[]` | List of bootstrap node addresses |
| `enable_mdns` | `true` | Enable local network discovery |
| `enable_relay` | `true` | Enable NAT traversal via relay |
| `connection_timeout` | `10s` | Timeout for new connections |
| `idle_connection_timeout` | `60s` | Timeout for idle connections |
| `kademlia_replication_factor` | `20` | DHT replication factor |

## Events

The network manager emits the following events:

- `PeerDiscovered`: New peer found via mDNS or DHT
- `PeerConnected`: Connection established
- `PeerDisconnected`: Peer disconnected
- `PeerIdentified`: Peer metadata received
- `BootstrapCompleted`: DHT bootstrap finished
- `Error`: Network error occurred

## Testing

Run the test suite:

```bash
# All tests
cargo test -p codio-network

# With verbose output
cargo test -p codio-network -- --nocapture

# Specific test
cargo test -p codio-network test_connection_management
```

### Integration Tests

The test suite includes:

- ✅ Peer discovery via mDNS
- ✅ Connection management (connect/disconnect)
- ✅ Bootstrap node connection
- ✅ Connection limit enforcement
- ✅ Peer information tracking
- ✅ Network statistics

**Note**: mDNS tests may not work in all CI/CD environments due to multicast restrictions.

## Performance Considerations

### Scalability

- **O(log N) peer lookup**: Via Kademlia DHT
- **Connection pooling**: Reuses TCP connections
- **Lazy evaluation**: Connections established on-demand
- **Resource limits**: Configurable peer count prevents memory exhaustion

### Latency

- **Multiplexing**: Yamux enables concurrent streams over single connection
- **Keep-alive**: Ping protocol maintains connections
- **Local caching**: DHT results cached for faster lookups

### Bandwidth

- **Efficient encoding**: Protobuf-based protocol messages
- **Compression**: Optional transport-level compression
- **Rate limiting**: Built-in backpressure mechanisms

## Security Considerations

### Threat Model

- **Man-in-the-middle**: Prevented by Noise encryption
- **Replay attacks**: Prevented by Noise handshake nonces
- **Sybil attacks**: Mitigated by Kademlia's key-based routing
- **Eclipse attacks**: Mitigated by diverse peer connections

### Best Practices

1. **Use bootstrap peers**: Don't rely solely on mDNS
2. **Set connection limits**: Prevent resource exhaustion
3. **Monitor peer quality**: Disconnect unresponsive peers
4. **Validate content**: Always verify content hashes
5. **Rate limit**: Implement application-level rate limiting

## Troubleshooting

### No peers discovered

- Check firewall settings (allow UDP 5353 for mDNS)
- Verify bootstrap peers are reachable
- Enable debug logging: `RUST_LOG=codio_network=debug`

### Connection failures

- Verify peer addresses are correct
- Check NAT/firewall configuration
- Enable relay for NAT traversal
- Increase connection timeout

### High memory usage

- Reduce `max_peers` limit
- Decrease `kademlia_replication_factor`
- Monitor peer count: `manager.peer_count()`

## Dependencies

- **libp2p** (0.53): Core P2P networking
- **tokio** (1.35): Async runtime
- **anyhow**: Error handling
- **tracing**: Structured logging
- **serde**: Configuration serialization

## Future Enhancements

- [ ] WebRTC transport for browser compatibility
- [ ] QUIC transport for improved performance
- [ ] Gossipsub for pub/sub messaging
- [ ] Circuit relay v2 for better NAT traversal
- [ ] Bandwidth metering and QoS
- [ ] Custom content routing strategies

## References

- [libp2p Documentation](https://docs.libp2p.io/)
- [Kademlia DHT Paper](https://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf)
- [Noise Protocol](https://noiseprotocol.org/)
- [IPFS Networking](https://docs.ipfs.io/concepts/how-ipfs-works/#network)

## License

See repository LICENSE file.

## Contributing

See repository CONTRIBUTING.md file.
