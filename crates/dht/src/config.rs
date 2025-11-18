//! DHT Configuration module
//!
//! Provides configuration options for the Kademlia DHT including
//! timeouts, replication factors, and bootstrap peers.

use libp2p::{Multiaddr, PeerId};
use std::time::Duration;

/// Configuration for the DHT
///
/// Controls various aspects of DHT behavior including query timeouts,
/// replication factor, and provider record TTL.
///
/// # Example
///
/// ```rust
/// use codio_dht::DHTConfig;
/// use std::time::Duration;
///
/// let config = DHTConfig {
///     replication_factor: 20,
///     query_timeout: Duration::from_secs(60),
///     provider_timeout: Some(Duration::from_secs(24 * 3600)),
///     republish_interval: Some(Duration::from_secs(12 * 3600)),
///     bootstrap_peers: vec![],
/// };
/// ```
#[derive(Debug, Clone)]
pub struct DHTConfig {
    /// Replication factor (K) - number of nodes to replicate data to
    ///
    /// This determines how many nodes will store provider records for each piece
    /// of content. Higher values provide better fault tolerance but increase
    /// network overhead. Kademlia typically uses K=20.
    ///
    /// Default: 20
    pub replication_factor: usize,

    /// Timeout for DHT queries
    ///
    /// Maximum time to wait for a DHT query (provide, find_providers, bootstrap)
    /// to complete before considering it failed.
    ///
    /// Default: 60 seconds
    pub query_timeout: Duration,

    /// Provider record time-to-live (TTL)
    ///
    /// How long provider records remain valid in the DHT before expiring.
    /// Nodes should re-announce content before this timeout expires.
    ///
    /// Default: 24 hours (86400 seconds)
    pub provider_timeout: Option<Duration>,

    /// Republish interval for provider records
    ///
    /// How often to automatically re-announce content we're providing.
    /// Should be less than `provider_timeout` to prevent records from expiring.
    ///
    /// Default: 12 hours (43200 seconds)
    pub republish_interval: Option<Duration>,

    /// Bootstrap peer addresses
    ///
    /// List of known peers to connect to when bootstrapping the DHT.
    /// These are typically long-running, stable nodes in the network.
    ///
    /// Format: Vec<(PeerId, Multiaddr)>
    ///
    /// Default: empty (local-only mode)
    pub bootstrap_peers: Vec<(PeerId, Multiaddr)>,
}

impl Default for DHTConfig {
    /// Create a default DHT configuration
    ///
    /// Uses standard Kademlia parameters:
    /// - Replication factor: 20 (K=20)
    /// - Query timeout: 60 seconds
    /// - Provider TTL: 24 hours
    /// - Republish interval: 12 hours
    /// - No bootstrap peers (local mode)
    fn default() -> Self {
        Self {
            replication_factor: 20,
            query_timeout: Duration::from_secs(60),
            provider_timeout: Some(Duration::from_secs(24 * 3600)), // 24 hours
            republish_interval: Some(Duration::from_secs(12 * 3600)), // 12 hours
            bootstrap_peers: vec![],
        }
    }
}

impl DHTConfig {
    /// Create a new DHT config with custom settings
    ///
    /// # Arguments
    ///
    /// * `replication_factor` - Number of nodes to replicate to (K value)
    /// * `query_timeout` - Timeout for DHT queries
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_dht::DHTConfig;
    /// use std::time::Duration;
    ///
    /// let config = DHTConfig::new(20, Duration::from_secs(30));
    /// ```
    pub fn new(replication_factor: usize, query_timeout: Duration) -> Self {
        Self {
            replication_factor,
            query_timeout,
            ..Default::default()
        }
    }

    /// Create a config for testing (shorter timeouts)
    ///
    /// Uses reduced timeouts suitable for unit tests:
    /// - Query timeout: 5 seconds
    /// - Provider TTL: 1 hour
    /// - Republish interval: 30 minutes
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_dht::DHTConfig;
    ///
    /// let config = DHTConfig::test();
    /// assert_eq!(config.replication_factor, 3);
    /// ```
    pub fn test() -> Self {
        Self {
            replication_factor: 3,
            query_timeout: Duration::from_secs(5),
            provider_timeout: Some(Duration::from_secs(3600)), // 1 hour
            republish_interval: Some(Duration::from_secs(1800)), // 30 minutes
            bootstrap_peers: vec![],
        }
    }

    /// Create a config for production use
    ///
    /// Uses standard Kademlia parameters with conservative timeouts
    /// for production environments.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_dht::DHTConfig;
    ///
    /// let config = DHTConfig::production();
    /// ```
    pub fn production() -> Self {
        Self::default()
    }

    /// Set bootstrap peers
    ///
    /// # Arguments
    ///
    /// * `peers` - List of (PeerId, Multiaddr) tuples for bootstrap nodes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use codio_dht::DHTConfig;
    /// use libp2p::{PeerId, Multiaddr};
    ///
    /// let mut config = DHTConfig::default();
    /// let peer_id = PeerId::random();
    /// let addr: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
    /// config.with_bootstrap_peers(vec![(peer_id, addr)]);
    /// ```
    pub fn with_bootstrap_peers(mut self, peers: Vec<(PeerId, Multiaddr)>) -> Self {
        self.bootstrap_peers = peers;
        self
    }

    /// Set replication factor
    ///
    /// # Arguments
    ///
    /// * `k` - Replication factor (typically 20)
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_dht::DHTConfig;
    ///
    /// let config = DHTConfig::default().with_replication_factor(10);
    /// assert_eq!(config.replication_factor, 10);
    /// ```
    pub fn with_replication_factor(mut self, k: usize) -> Self {
        self.replication_factor = k;
        self
    }

    /// Set query timeout
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time for queries
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_dht::DHTConfig;
    /// use std::time::Duration;
    ///
    /// let config = DHTConfig::default()
    ///     .with_query_timeout(Duration::from_secs(30));
    /// ```
    pub fn with_query_timeout(mut self, timeout: Duration) -> Self {
        self.query_timeout = timeout;
        self
    }

    /// Set provider record TTL
    ///
    /// # Arguments
    ///
    /// * `ttl` - Time-to-live for provider records
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_dht::DHTConfig;
    /// use std::time::Duration;
    ///
    /// let config = DHTConfig::default()
    ///     .with_provider_timeout(Duration::from_secs(12 * 3600));
    /// ```
    pub fn with_provider_timeout(mut self, ttl: Duration) -> Self {
        self.provider_timeout = Some(ttl);
        self
    }

    /// Set republish interval
    ///
    /// # Arguments
    ///
    /// * `interval` - How often to re-announce content
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_dht::DHTConfig;
    /// use std::time::Duration;
    ///
    /// let config = DHTConfig::default()
    ///     .with_republish_interval(Duration::from_secs(6 * 3600));
    /// ```
    pub fn with_republish_interval(mut self, interval: Duration) -> Self {
        self.republish_interval = Some(interval);
        self
    }

    /// Disable automatic republishing
    ///
    /// Provider records will not be automatically re-announced.
    /// Use this if you want manual control over re-announcing.
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_dht::DHTConfig;
    ///
    /// let config = DHTConfig::default().without_republish();
    /// assert!(config.republish_interval.is_none());
    /// ```
    pub fn without_republish(mut self) -> Self {
        self.republish_interval = None;
        self
    }

    /// Validate configuration
    ///
    /// Checks that configuration values are sensible and returns an error
    /// if any are invalid.
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, `Err(String)` with error message if invalid
    ///
    /// # Example
    ///
    /// ```rust
    /// use codio_dht::DHTConfig;
    ///
    /// let config = DHTConfig::default();
    /// assert!(config.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        // Replication factor must be > 0
        if self.replication_factor == 0 {
            return Err("Replication factor must be greater than 0".to_string());
        }

        // Replication factor should be reasonable (1-50)
        if self.replication_factor > 50 {
            return Err("Replication factor too high (max 50)".to_string());
        }

        // Query timeout must be reasonable
        if self.query_timeout < Duration::from_secs(1) {
            return Err("Query timeout too short (min 1 second)".to_string());
        }

        if self.query_timeout > Duration::from_secs(300) {
            return Err("Query timeout too long (max 5 minutes)".to_string());
        }

        // Republish interval should be less than provider timeout
        if let (Some(republish), Some(timeout)) = (self.republish_interval, self.provider_timeout) {
            if republish >= timeout {
                return Err("Republish interval must be less than provider timeout".to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DHTConfig::default();
        assert_eq!(config.replication_factor, 20);
        assert_eq!(config.query_timeout, Duration::from_secs(60));
        assert_eq!(
            config.provider_timeout,
            Some(Duration::from_secs(24 * 3600))
        );
        assert_eq!(
            config.republish_interval,
            Some(Duration::from_secs(12 * 3600))
        );
        assert!(config.bootstrap_peers.is_empty());
    }

    #[test]
    fn test_test_config() {
        let config = DHTConfig::test();
        assert_eq!(config.replication_factor, 3);
        assert_eq!(config.query_timeout, Duration::from_secs(5));
    }

    #[test]
    fn test_validation() {
        // Valid config
        let config = DHTConfig::default();
        assert!(config.validate().is_ok());

        // Zero replication factor
        let mut invalid = DHTConfig::default();
        invalid.replication_factor = 0;
        assert!(invalid.validate().is_err());

        // Too high replication factor
        invalid.replication_factor = 100;
        assert!(invalid.validate().is_err());

        // Query timeout too short
        invalid = DHTConfig::default();
        invalid.query_timeout = Duration::from_millis(500);
        assert!(invalid.validate().is_err());

        // Republish interval >= provider timeout
        invalid = DHTConfig::default();
        invalid.republish_interval = Some(Duration::from_secs(25 * 3600));
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_builder_pattern() {
        let config = DHTConfig::default()
            .with_replication_factor(10)
            .with_query_timeout(Duration::from_secs(30))
            .with_provider_timeout(Duration::from_secs(12 * 3600))
            .with_republish_interval(Duration::from_secs(6 * 3600));

        assert_eq!(config.replication_factor, 10);
        assert_eq!(config.query_timeout, Duration::from_secs(30));
        assert_eq!(
            config.provider_timeout,
            Some(Duration::from_secs(12 * 3600))
        );
        assert_eq!(
            config.republish_interval,
            Some(Duration::from_secs(6 * 3600))
        );
    }

    #[test]
    fn test_without_republish() {
        let config = DHTConfig::default().without_republish();
        assert!(config.republish_interval.is_none());
    }
}
