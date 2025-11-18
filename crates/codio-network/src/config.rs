//! Network configuration for the Codio P2P network.
//!
//! This module provides configuration options for the network layer,
//! including peer limits, bootstrap nodes, and protocol settings.

use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};

/// Configuration for the P2P network layer.
///
/// # Examples
///
/// ```
/// use codio_network::NetworkConfig;
///
/// let config = NetworkConfig::default();
/// assert_eq!(config.max_peers, 50);
/// assert!(config.enable_mdns);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Port to listen on for incoming connections.
    /// If set to 0, the OS will assign a random available port.
    pub listen_port: u16,

    /// Maximum number of concurrent peer connections.
    pub max_peers: usize,

    /// List of bootstrap peer addresses to connect to on startup.
    /// Format: `/ip4/1.2.3.4/tcp/4001/p2p/QmBootstrapPeerId`
    pub bootstrap_peers: Vec<Multiaddr>,

    /// Enable mDNS for local network peer discovery.
    pub enable_mdns: bool,

    /// Enable relay protocol for NAT traversal.
    pub enable_relay: bool,

    /// Kademlia replication factor (number of closest peers to replicate to).
    pub kad_replication_factor: usize,

    /// Connection idle timeout in seconds.
    pub connection_idle_timeout_secs: u64,

    /// Enable custom Codio protocol.
    pub enable_codio_protocol: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_port: 0, // random port
            max_peers: 50,
            bootstrap_peers: vec![],
            enable_mdns: true,
            enable_relay: true,
            kad_replication_factor: 20,
            connection_idle_timeout_secs: 300, // 5 minutes
            enable_codio_protocol: true,
        }
    }
}

impl NetworkConfig {
    /// Create a new network configuration with custom settings.
    pub fn new(listen_port: u16, max_peers: usize) -> Self {
        Self {
            listen_port,
            max_peers,
            ..Default::default()
        }
    }

    /// Add a bootstrap peer address.
    pub fn with_bootstrap_peer(mut self, addr: Multiaddr) -> Self {
        self.bootstrap_peers.push(addr);
        self
    }

    /// Add multiple bootstrap peer addresses.
    pub fn with_bootstrap_peers(mut self, addrs: Vec<Multiaddr>) -> Self {
        self.bootstrap_peers.extend(addrs);
        self
    }

    /// Disable mDNS discovery.
    pub fn without_mdns(mut self) -> Self {
        self.enable_mdns = false;
        self
    }

    /// Disable relay protocol.
    pub fn without_relay(mut self) -> Self {
        self.enable_relay = false;
        self
    }

    /// Set the Kademlia replication factor.
    pub fn with_kad_replication_factor(mut self, factor: usize) -> Self {
        self.kad_replication_factor = factor;
        self
    }

    /// Validate the configuration.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.max_peers == 0 {
            anyhow::bail!("max_peers must be greater than 0");
        }
        if self.kad_replication_factor == 0 {
            anyhow::bail!("kad_replication_factor must be greater than 0");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NetworkConfig::default();
        assert_eq!(config.listen_port, 0);
        assert_eq!(config.max_peers, 50);
        assert!(config.enable_mdns);
        assert!(config.enable_relay);
        assert!(config.bootstrap_peers.is_empty());
    }

    #[test]
    fn test_builder_pattern() {
        let config = NetworkConfig::new(4001, 100)
            .without_mdns()
            .without_relay()
            .with_kad_replication_factor(10);

        assert_eq!(config.listen_port, 4001);
        assert_eq!(config.max_peers, 100);
        assert!(!config.enable_mdns);
        assert!(!config.enable_relay);
        assert_eq!(config.kad_replication_factor, 10);
    }

    #[test]
    fn test_validation() {
        let mut config = NetworkConfig::default();
        assert!(config.validate().is_ok());

        config.max_peers = 0;
        assert!(config.validate().is_err());
    }
}
