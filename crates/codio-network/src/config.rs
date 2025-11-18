//! Network configuration module.
//!
//! This module provides configuration options for the P2P network layer.

use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Network configuration for the P2P layer.
///
/// # Examples
///
/// ```
/// use codio_network::NetworkConfig;
///
/// let config = NetworkConfig::default();
/// assert_eq!(config.max_peers, 50);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Port to listen on (0 for random port)
    pub listen_port: u16,

    /// Maximum number of connected peers
    pub max_peers: usize,

    /// Bootstrap peer addresses for initial connection
    pub bootstrap_peers: Vec<Multiaddr>,

    /// Enable mDNS for local network discovery
    pub enable_mdns: bool,

    /// Enable relay for NAT traversal
    pub enable_relay: bool,

    /// Connection timeout duration
    pub connection_timeout: Duration,

    /// Idle connection timeout
    pub idle_connection_timeout: Duration,

    /// Kademlia replication factor
    pub kademlia_replication_factor: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_port: 0, // random port
            max_peers: 50,
            bootstrap_peers: vec![],
            enable_mdns: true,
            enable_relay: true,
            connection_timeout: Duration::from_secs(10),
            idle_connection_timeout: Duration::from_secs(60),
            kademlia_replication_factor: 20,
        }
    }
}

impl NetworkConfig {
    /// Creates a new network configuration with the specified port.
    ///
    /// # Examples
    ///
    /// ```
    /// use codio_network::NetworkConfig;
    ///
    /// let config = NetworkConfig::with_port(8080);
    /// assert_eq!(config.listen_port, 8080);
    /// ```
    pub fn with_port(port: u16) -> Self {
        Self {
            listen_port: port,
            ..Default::default()
        }
    }

    /// Adds a bootstrap peer to the configuration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use codio_network::NetworkConfig;
    /// use libp2p::Multiaddr;
    ///
    /// let mut config = NetworkConfig::default();
    /// let addr: Multiaddr = "/ip4/127.0.0.1/tcp/8080".parse().unwrap();
    /// config.add_bootstrap_peer(addr);
    /// ```
    pub fn add_bootstrap_peer(&mut self, addr: Multiaddr) {
        self.bootstrap_peers.push(addr);
    }

    /// Sets the maximum number of peers.
    pub fn with_max_peers(mut self, max_peers: usize) -> Self {
        self.max_peers = max_peers;
        self
    }

    /// Disables mDNS discovery.
    pub fn without_mdns(mut self) -> Self {
        self.enable_mdns = false;
        self
    }

    /// Disables relay for NAT traversal.
    pub fn without_relay(mut self) -> Self {
        self.enable_relay = false;
        self
    }
}
