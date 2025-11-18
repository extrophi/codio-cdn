//! # Codio Network Layer
//!
//! This crate provides the P2P networking foundation for the Codio CDN using libp2p.
//!
//! ## Features
//!
//! - **Peer Discovery**: mDNS for local networks, Kademlia DHT for global discovery
//! - **Connection Management**: Automatic reconnection, connection limits, NAT traversal
//! - **Transport Security**: Noise protocol encryption, Yamux multiplexing
//! - **Protocol Support**: Custom `/codio/1.0.0` protocol, ping, identify
//!
//! ## Example
//!
//! ```no_run
//! use codio_network::{NetworkManager, NetworkConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = NetworkConfig::default();
//!     let mut manager = NetworkManager::new(config).await?;
//!     manager.start().await?;
//!     Ok(())
//! }
//! ```

pub mod config;

pub use config::NetworkConfig;

use anyhow::{Context, Result};
use futures::StreamExt;
use libp2p::{
    identify,
    kad::{self, store::MemoryStore},
    mdns, noise, ping,
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, SwarmBuilder,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Custom protocol version for Codio CDN
const CODIO_PROTOCOL_VERSION: &str = "/codio/1.0.0";

/// Events emitted by the network manager
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// A new peer has been discovered
    PeerDiscovered {
        peer_id: PeerId,
        addresses: Vec<Multiaddr>,
    },
    /// A peer connection has been established
    PeerConnected { peer_id: PeerId, address: Multiaddr },
    /// A peer has disconnected
    PeerDisconnected { peer_id: PeerId },
    /// A peer has been identified
    PeerIdentified {
        peer_id: PeerId,
        info: Box<identify::Info>,
    },
    /// Kademlia bootstrap completed
    BootstrapCompleted,
    /// Network error occurred
    Error { message: String },
}

/// Information about a connected peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer identifier
    pub peer_id: PeerId,
    /// Known addresses for this peer
    pub addresses: Vec<Multiaddr>,
    /// Time when connection was established
    pub connection_time: Instant,
    /// Last time we received data from this peer
    pub last_seen: Instant,
    /// Peer's agent version (from identify protocol)
    pub agent_version: Option<String>,
    /// Peer's protocol version
    pub protocol_version: Option<String>,
    /// Supported protocols
    pub protocols: Vec<String>,
}

impl PeerInfo {
    /// Creates a new PeerInfo instance
    fn new(peer_id: PeerId, addresses: Vec<Multiaddr>) -> Self {
        let now = Instant::now();
        Self {
            peer_id,
            addresses,
            connection_time: now,
            last_seen: now,
            agent_version: None,
            protocol_version: None,
            protocols: Vec::new(),
        }
    }

    /// Updates the last seen timestamp
    fn update_last_seen(&mut self) {
        self.last_seen = Instant::now();
    }

    /// Updates peer information from identify protocol
    fn update_from_identify(&mut self, info: &identify::Info) {
        self.agent_version = Some(info.agent_version.clone());
        self.protocol_version = Some(info.protocol_version.clone());
        self.protocols = info.protocols.iter().map(|p| p.to_string()).collect();
        self.addresses = info.listen_addrs.clone();
        self.update_last_seen();
    }
}

/// Network behaviour combining multiple libp2p protocols
#[derive(NetworkBehaviour)]
pub struct CodioNetworkBehaviour {
    /// mDNS for local peer discovery (optional, may be disabled)
    mdns: libp2p::swarm::behaviour::toggle::Toggle<mdns::tokio::Behaviour>,
    /// Kademlia DHT for global peer discovery and content routing
    kademlia: kad::Behaviour<MemoryStore>,
    /// Ping protocol for keep-alive and latency measurement
    ping: ping::Behaviour,
    /// Identify protocol for peer metadata exchange
    identify: identify::Behaviour,
}

/// Main network manager for the P2P layer
pub struct NetworkManager {
    /// libp2p swarm managing the network
    swarm: Swarm<CodioNetworkBehaviour>,
    /// Local peer ID
    peer_id: PeerId,
    /// Listen addresses
    listen_addrs: Vec<Multiaddr>,
    /// Configuration
    config: NetworkConfig,
    /// Connected peers information
    connected_peers: HashMap<PeerId, PeerInfo>,
    /// Event channel for external listeners
    event_tx: mpsc::UnboundedSender<NetworkEvent>,
    /// Event receiver
    event_rx: mpsc::UnboundedReceiver<NetworkEvent>,
    /// Bootstrap status
    bootstrap_completed: bool,
    /// Pending connections
    pending_connections: HashMap<PeerId, Vec<Multiaddr>>,
}

impl NetworkManager {
    /// Creates a new network manager with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Network configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use codio_network::{NetworkManager, NetworkConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let config = NetworkConfig::with_port(8080);
    ///     let manager = NetworkManager::new(config).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(config: NetworkConfig) -> Result<Self> {
        info!("Initializing network manager with config: {:?}", config);

        // Generate local keypair
        let local_key = libp2p::identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        info!("Local peer ID: {}", local_peer_id);

        // Store config values for use in closure
        let enable_mdns = config.enable_mdns;
        let kademlia_replication_factor = config.kademlia_replication_factor;
        let bootstrap_peers = config.bootstrap_peers.clone();
        let idle_timeout = config.idle_connection_timeout;

        // Build the swarm using the new builder API
        let swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {
                let peer_id = key.public().to_peer_id();

                // Create Kademlia DHT
                let store = MemoryStore::new(peer_id);
                let mut kademlia_config = kad::Config::default();
                kademlia_config.set_replication_factor(
                    kademlia_replication_factor.try_into()
                        .expect("Invalid replication factor"),
                );
                let mut kademlia = kad::Behaviour::with_config(
                    peer_id,
                    store,
                    kademlia_config,
                );

                // Add bootstrap peers to Kademlia
                for addr in &bootstrap_peers {
                    if let Some(bootstrap_peer_id) = extract_peer_id(addr) {
                        kademlia.add_address(&bootstrap_peer_id, addr.clone());
                    }
                }

                // Create mDNS only if enabled
                let mdns = if enable_mdns {
                    match mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id) {
                        Ok(behaviour) => Some(behaviour).into(),
                        Err(e) => {
                            return Err(Box::new(std::io::Error::new(
                                std::io::ErrorKind::PermissionDenied,
                                format!("Failed to create mDNS behaviour: {}. Try disabling mDNS with .without_mdns()", e)
                            )) as Box<dyn std::error::Error + Send + Sync>);
                        }
                    }
                } else {
                    // mDNS disabled
                    None.into()
                };

                // Create identify protocol
                let identify = identify::Behaviour::new(
                    identify::Config::new(
                        CODIO_PROTOCOL_VERSION.to_string(),
                        key.public(),
                    )
                    .with_agent_version(format!("codio-network/{}", env!("CARGO_PKG_VERSION"))),
                );

                // Create ping protocol
                let ping = ping::Behaviour::new(
                    ping::Config::new()
                        .with_interval(Duration::from_secs(30))
                        .with_timeout(Duration::from_secs(10)),
                );

                Ok(CodioNetworkBehaviour {
                    mdns,
                    kademlia,
                    ping,
                    identify,
                })
            })?
            .with_swarm_config(|c| {
                c.with_idle_connection_timeout(idle_timeout)
            })
            .build();

        // Log bootstrap peers after swarm creation
        for addr in &bootstrap_peers {
            if let Some(peer_id) = extract_peer_id(addr) {
                info!("Added bootstrap peer: {} at {}", peer_id, addr);
            } else {
                warn!("Bootstrap address missing peer ID: {}", addr);
            }
        }

        // Create event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Ok(Self {
            swarm,
            peer_id: local_peer_id,
            listen_addrs: Vec::new(),
            config,
            connected_peers: HashMap::new(),
            event_tx,
            event_rx,
            bootstrap_completed: false,
            pending_connections: HashMap::new(),
        })
    }

    /// Returns the local peer ID
    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    /// Returns the listen addresses
    pub fn listen_addrs(&self) -> &[Multiaddr] {
        &self.listen_addrs
    }

    /// Returns information about connected peers
    pub fn connected_peers(&self) -> &HashMap<PeerId, PeerInfo> {
        &self.connected_peers
    }

    /// Returns the number of connected peers
    pub fn peer_count(&self) -> usize {
        self.connected_peers.len()
    }

    /// Starts the network manager and begins listening for connections.
    ///
    /// This method starts listening on the configured port and begins
    /// peer discovery via mDNS and Kademlia.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use codio_network::{NetworkManager, NetworkConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let config = NetworkConfig::default();
    ///     let mut manager = NetworkManager::new(config).await?;
    ///     manager.start().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn start(&mut self) -> Result<()> {
        // Start listening
        let listen_addr = format!("/ip4/0.0.0.0/tcp/{}", self.config.listen_port)
            .parse()
            .context("Failed to parse listen address")?;

        self.swarm
            .listen_on(listen_addr)
            .context("Failed to start listening")?;

        info!("Network manager started, waiting for listen address...");

        // Wait for the listen address to be confirmed
        while let Some(event) = self.swarm.next().await {
            if let SwarmEvent::NewListenAddr { address, .. } = event {
                info!("Listening on: {}", address);
                self.listen_addrs.push(address);
                break;
            }
        }

        // Bootstrap Kademlia if we have bootstrap peers
        if !self.config.bootstrap_peers.is_empty() {
            info!("Starting Kademlia bootstrap...");
            if let Err(e) = self.swarm.behaviour_mut().kademlia.bootstrap() {
                warn!("Failed to start Kademlia bootstrap: {:?}", e);
            }
        }

        Ok(())
    }

    /// Connects to a peer at the specified address.
    ///
    /// # Arguments
    ///
    /// * `addr` - Multiaddress of the peer to connect to
    ///
    /// # Returns
    ///
    /// The peer ID if the address contains one, or an error
    ///
    /// # Example
    ///
    /// ```no_run
    /// use codio_network::{NetworkManager, NetworkConfig};
    /// use libp2p::Multiaddr;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let mut manager = NetworkManager::new(NetworkConfig::default()).await?;
    ///     let addr: Multiaddr = "/ip4/127.0.0.1/tcp/8080".parse()?;
    ///     manager.connect_peer(addr).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn connect_peer(&mut self, addr: Multiaddr) -> Result<PeerId> {
        let peer_id = extract_peer_id(&addr).context("Address must contain peer ID")?;

        // Check if we're already connected
        if self.connected_peers.contains_key(&peer_id) {
            info!("Already connected to peer: {}", peer_id);
            return Ok(peer_id);
        }

        // Check connection limit
        if self.connected_peers.len() >= self.config.max_peers {
            anyhow::bail!("Maximum peer limit ({}) reached", self.config.max_peers);
        }

        info!("Connecting to peer {} at {}", peer_id, addr);

        // Add to Kademlia routing table
        self.swarm
            .behaviour_mut()
            .kademlia
            .add_address(&peer_id, addr.clone());

        // Dial the peer
        self.swarm
            .dial(addr.clone())
            .context("Failed to dial peer")?;

        // Track pending connection
        self.pending_connections
            .entry(peer_id)
            .or_default()
            .push(addr);

        Ok(peer_id)
    }

    /// Disconnects from a peer.
    ///
    /// # Arguments
    ///
    /// * `peer_id` - ID of the peer to disconnect from
    ///
    /// # Example
    ///
    /// ```no_run
    /// use codio_network::{NetworkManager, NetworkConfig};
    /// use libp2p::PeerId;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let mut manager = NetworkManager::new(NetworkConfig::default()).await?;
    ///     // ... connect to peer ...
    ///     # let peer_id = PeerId::random();
    ///     manager.disconnect_peer(peer_id).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn disconnect_peer(&mut self, peer_id: PeerId) -> Result<()> {
        if !self.connected_peers.contains_key(&peer_id) {
            warn!("Peer {} is not connected", peer_id);
            return Ok(());
        }

        info!("Disconnecting from peer: {}", peer_id);

        // Close all connections to this peer
        let _ = self.swarm.disconnect_peer_id(peer_id);

        // Remove from connected peers
        self.connected_peers.remove(&peer_id);

        // Emit disconnect event
        let _ = self
            .event_tx
            .send(NetworkEvent::PeerDisconnected { peer_id });

        Ok(())
    }

    /// Processes network events.
    ///
    /// This method should be called in a loop to process incoming network events.
    /// It returns `None` when the network should shut down.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use codio_network::{NetworkManager, NetworkConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let mut manager = NetworkManager::new(NetworkConfig::default()).await?;
    ///     manager.start().await?;
    ///
    ///     loop {
    ///         if let Some(event) = manager.next_event().await {
    ///             println!("Network event: {:?}", event);
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn next_event(&mut self) -> Option<NetworkEvent> {
        tokio::select! {
            // Process swarm events
            event = self.swarm.next() => {
                if let Some(event) = event {
                    self.handle_swarm_event(event).await;
                }
                None
            }
            // Receive events from the channel
            event = self.event_rx.recv() => {
                event
            }
        }
    }

    /// Runs the event loop indefinitely.
    ///
    /// This is a convenience method that processes events in a loop.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use codio_network::{NetworkManager, NetworkConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let mut manager = NetworkManager::new(NetworkConfig::default()).await?;
    ///     manager.start().await?;
    ///     manager.run().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting network event loop");

        loop {
            let event = self.swarm.select_next_some().await;
            self.handle_swarm_event(event).await;
        }
    }

    /// Handles swarm events from libp2p
    async fn handle_swarm_event(&mut self, event: SwarmEvent<CodioNetworkBehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on: {}", address);
                if !self.listen_addrs.contains(&address) {
                    self.listen_addrs.push(address);
                }
            }

            SwarmEvent::ConnectionEstablished {
                peer_id,
                endpoint,
                num_established,
                ..
            } => {
                let address = endpoint.get_remote_address().clone();
                info!(
                    "Connection established with {} at {} (total: {})",
                    peer_id, address, num_established
                );

                // Add or update peer info
                if let std::collections::hash_map::Entry::Vacant(e) =
                    self.connected_peers.entry(peer_id)
                {
                    let peer_info = PeerInfo::new(peer_id, vec![address.clone()]);
                    e.insert(peer_info);

                    // Emit connected event
                    let _ = self.event_tx.send(NetworkEvent::PeerConnected {
                        peer_id,
                        address,
                    });
                }

                // Remove from pending connections
                self.pending_connections.remove(&peer_id);
            }

            SwarmEvent::ConnectionClosed {
                peer_id,
                cause,
                num_established,
                ..
            } => {
                info!(
                    "Connection closed with {} (remaining: {}, cause: {:?})",
                    peer_id, num_established, cause
                );

                // If this was the last connection, remove peer info
                if num_established == 0 {
                    self.connected_peers.remove(&peer_id);

                    // Emit disconnect event
                    let _ = self
                        .event_tx
                        .send(NetworkEvent::PeerDisconnected { peer_id });
                }
            }

            SwarmEvent::IncomingConnection { .. } => {
                debug!("Incoming connection");
            }

            SwarmEvent::IncomingConnectionError { error, .. } => {
                warn!("Incoming connection error: {}", error);
            }

            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                warn!("Outgoing connection error to {:?}: {}", peer_id, error);
                if let Some(peer_id) = peer_id {
                    self.pending_connections.remove(&peer_id);
                }

                let _ = self.event_tx.send(NetworkEvent::Error {
                    message: format!("Connection error: {}", error),
                });
            }

            SwarmEvent::Behaviour(event) => {
                self.handle_behaviour_event(event).await;
            }

            _ => {}
        }
    }

    /// Handles behaviour-specific events
    async fn handle_behaviour_event(&mut self, event: CodioNetworkBehaviourEvent) {
        match event {
            // mDNS events
            CodioNetworkBehaviourEvent::Mdns(mdns_event) => {
                match mdns_event {
                    mdns::Event::Discovered(peers) => {
                        if self.config.enable_mdns {
                            for (peer_id, addr) in peers {
                                if peer_id == self.peer_id {
                                    continue; // Skip self
                                }

                                debug!("mDNS discovered peer: {} at {}", peer_id, addr);

                                // Add to Kademlia routing table
                                self.swarm
                                    .behaviour_mut()
                                    .kademlia
                                    .add_address(&peer_id, addr.clone());

                                // Emit discovery event
                                let _ = self.event_tx.send(NetworkEvent::PeerDiscovered {
                                    peer_id,
                                    addresses: vec![addr.clone()],
                                });

                                // Auto-connect if under peer limit
                                if self.connected_peers.len() < self.config.max_peers
                                    && !self.connected_peers.contains_key(&peer_id)
                                    && !self.pending_connections.contains_key(&peer_id)
                                {
                                    if let Err(e) = self.connect_peer(addr).await {
                                        warn!(
                                            "Failed to auto-connect to mDNS peer {}: {}",
                                            peer_id, e
                                        );
                                    }
                                }
                            }
                        }
                    }
                    mdns::Event::Expired(peers) => {
                        for (peer_id, _addr) in peers {
                            debug!("mDNS peer expired: {}", peer_id);
                        }
                    }
                }
            }

            // Kademlia events
            CodioNetworkBehaviourEvent::Kademlia(kad_event) => {
                match kad_event {
                    kad::Event::RoutingUpdated {
                        peer, addresses, ..
                    } => {
                        debug!("Kademlia routing updated for peer: {}", peer);

                        if let Some(peer_info) = self.connected_peers.get_mut(&peer) {
                            // Extract addresses from the addresses type
                            peer_info.addresses = addresses.into_vec();
                            peer_info.update_last_seen();
                        }
                    }

                    kad::Event::OutboundQueryProgressed { result, .. } => match result {
                        kad::QueryResult::Bootstrap(Ok(kad::BootstrapOk { peer, .. })) => {
                            info!("Kademlia bootstrap succeeded with peer: {}", peer);
                            if !self.bootstrap_completed {
                                self.bootstrap_completed = true;
                                let _ = self.event_tx.send(NetworkEvent::BootstrapCompleted);
                            }
                        }
                        kad::QueryResult::Bootstrap(Err(e)) => {
                            warn!("Kademlia bootstrap error: {:?}", e);
                        }
                        kad::QueryResult::GetProviders(Ok(_ok)) => {
                            debug!("Kademlia GetProviders query completed");
                        }
                        kad::QueryResult::GetProviders(Err(e)) => {
                            debug!("Kademlia GetProviders failed: {:?}", e);
                        }
                        kad::QueryResult::StartProviding(Ok(_)) => {
                            debug!("Kademlia StartProviding succeeded");
                        }
                        kad::QueryResult::StartProviding(Err(e)) => {
                            warn!("Kademlia StartProviding failed: {:?}", e);
                        }
                        _ => {}
                    },

                    _ => {}
                }
            }

            // Ping events
            CodioNetworkBehaviourEvent::Ping(ping_event) => {
                match ping_event.result {
                    Ok(duration) => {
                        debug!("Ping to {} succeeded: {:?}", ping_event.peer, duration);

                        // Update last seen time
                        if let Some(peer_info) = self.connected_peers.get_mut(&ping_event.peer) {
                            peer_info.update_last_seen();
                        }
                    }
                    Err(e) => {
                        warn!("Ping to {} failed: {:?}", ping_event.peer, e);
                    }
                }
            }

            // Identify events
            CodioNetworkBehaviourEvent::Identify(identify_event) => {
                match identify_event {
                    identify::Event::Received { peer_id, info } => {
                        info!(
                            "Identified peer: {} (agent: {}, protocol: {})",
                            peer_id, info.agent_version, info.protocol_version
                        );

                        // Update peer info with identify data
                        if let Some(peer_info) = self.connected_peers.get_mut(&peer_id) {
                            peer_info.update_from_identify(&info);
                        }

                        // Add addresses to Kademlia
                        for addr in &info.listen_addrs {
                            self.swarm
                                .behaviour_mut()
                                .kademlia
                                .add_address(&peer_id, addr.clone());
                        }

                        // Emit identified event
                        let _ = self.event_tx.send(NetworkEvent::PeerIdentified {
                            peer_id,
                            info: Box::new(info),
                        });
                    }
                    identify::Event::Sent { .. } => {
                        debug!("Identify info sent");
                    }
                    identify::Event::Pushed { .. } => {
                        debug!("Identify info pushed");
                    }
                    identify::Event::Error { peer_id, error } => {
                        warn!("Identify error with peer {}: {:?}", peer_id, error);
                    }
                }
            }
        }
    }

    /// Adds a peer address to the Kademlia routing table.
    ///
    /// This is useful for adding known peers that may be used for future connections.
    pub fn add_peer_address(&mut self, peer_id: PeerId, addr: Multiaddr) {
        self.swarm
            .behaviour_mut()
            .kademlia
            .add_address(&peer_id, addr);
    }

    /// Starts providing content on the DHT.
    ///
    /// This announces that this peer has content with the given key.
    pub fn start_providing(&mut self, key: kad::RecordKey) -> Result<kad::QueryId> {
        self.swarm
            .behaviour_mut()
            .kademlia
            .start_providing(key)
            .context("Failed to start providing")
    }

    /// Gets providers for a content key from the DHT.
    ///
    /// This queries the DHT to find peers that have content with the given key.
    pub fn get_providers(&mut self, key: kad::RecordKey) -> kad::QueryId {
        self.swarm.behaviour_mut().kademlia.get_providers(key)
    }

    /// Returns whether the bootstrap process has completed.
    pub fn is_bootstrap_completed(&self) -> bool {
        self.bootstrap_completed
    }

    /// Returns statistics about the network.
    pub fn stats(&self) -> NetworkStats {
        NetworkStats {
            peer_count: self.connected_peers.len(),
            listen_addrs: self.listen_addrs.clone(),
            bootstrap_completed: self.bootstrap_completed,
            pending_connections: self.pending_connections.len(),
        }
    }

    /// Processes a single swarm event.
    ///
    /// This is primarily useful for testing and low-level control.
    /// For normal usage, prefer `run()` or `next_event()`.
    pub async fn poll_once(&mut self) {
        if let Some(event) = self.swarm.next().await {
            self.handle_swarm_event(event).await;
        }
    }
}

/// Network statistics
#[derive(Debug, Clone)]
pub struct NetworkStats {
    /// Number of connected peers
    pub peer_count: usize,
    /// Listen addresses
    pub listen_addrs: Vec<Multiaddr>,
    /// Bootstrap completion status
    pub bootstrap_completed: bool,
    /// Number of pending connections
    pub pending_connections: usize,
}

/// Extracts the peer ID from a multiaddress.
///
/// Returns `None` if the address doesn't contain a peer ID component.
fn extract_peer_id(addr: &Multiaddr) -> Option<PeerId> {
    addr.iter().find_map(|component| {
        if let libp2p::multiaddr::Protocol::P2p(peer_id) = component {
            Some(peer_id)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_manager_creation() {
        // Disable mDNS for tests as it requires network permissions
        let config = NetworkConfig::default().without_mdns();
        let manager = NetworkManager::new(config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_network_config_builder() {
        let config = NetworkConfig::with_port(8080)
            .with_max_peers(100)
            .without_mdns();

        assert_eq!(config.listen_port, 8080);
        assert_eq!(config.max_peers, 100);
        assert!(!config.enable_mdns);
    }

    #[test]
    fn test_extract_peer_id() {
        let peer_id = PeerId::random();
        let addr: Multiaddr = format!("/ip4/127.0.0.1/tcp/8080/p2p/{}", peer_id)
            .parse()
            .unwrap();

        let extracted = extract_peer_id(&addr);
        assert_eq!(extracted, Some(peer_id));
    }

    #[test]
    fn test_extract_peer_id_none() {
        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/8080".parse().unwrap();
        let extracted = extract_peer_id(&addr);
        assert_eq!(extracted, None);
    }

    #[tokio::test]
    async fn test_peer_info_creation() {
        let peer_id = PeerId::random();
        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/8080".parse().unwrap();
        let info = PeerInfo::new(peer_id, vec![addr.clone()]);

        assert_eq!(info.peer_id, peer_id);
        assert_eq!(info.addresses.len(), 1);
        assert_eq!(info.addresses[0], addr);
    }
}
