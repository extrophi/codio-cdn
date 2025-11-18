use std::time::Duration;

/// Configuration for the availability tracker
#[derive(Debug, Clone)]
pub struct TrackerConfig {
    /// Interval between gossip updates
    pub update_interval: Duration,
    /// Time after which a peer is considered offline
    pub peer_timeout: Duration,
    /// Minimum reputation score required for a peer to be considered reliable
    pub min_reputation: f64,
    /// Maximum number of bandwidth measurements to keep for rolling average
    pub max_bandwidth_samples: usize,
    /// Maximum number of peers to track
    pub max_peers: usize,
    /// Weight factor for uptime in reputation calculation
    pub uptime_weight: f64,
    /// Weight factor for success rate in reputation calculation
    pub success_weight: f64,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            update_interval: Duration::from_secs(30),
            peer_timeout: Duration::from_secs(300), // 5 minutes
            min_reputation: 0.3,
            max_bandwidth_samples: 10,
            max_peers: 1000,
            uptime_weight: 0.3,
            success_weight: 0.7,
        }
    }
}

impl TrackerConfig {
    /// Create a new configuration with custom values
    pub fn new(update_interval: Duration, peer_timeout: Duration, min_reputation: f64) -> Self {
        Self {
            update_interval,
            peer_timeout,
            min_reputation,
            ..Default::default()
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.min_reputation < 0.0 || self.min_reputation > 1.0 {
            anyhow::bail!("min_reputation must be between 0.0 and 1.0");
        }

        if self.uptime_weight + self.success_weight != 1.0 {
            anyhow::bail!("uptime_weight + success_weight must equal 1.0");
        }

        if self.max_bandwidth_samples == 0 {
            anyhow::bail!("max_bandwidth_samples must be greater than 0");
        }

        if self.max_peers == 0 {
            anyhow::bail!("max_peers must be greater than 0");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TrackerConfig::default();
        assert_eq!(config.update_interval, Duration::from_secs(30));
        assert_eq!(config.peer_timeout, Duration::from_secs(300));
        assert_eq!(config.min_reputation, 0.3);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_custom_config() {
        let config = TrackerConfig::new(Duration::from_secs(60), Duration::from_secs(600), 0.5);
        assert_eq!(config.update_interval, Duration::from_secs(60));
        assert_eq!(config.peer_timeout, Duration::from_secs(600));
        assert_eq!(config.min_reputation, 0.5);
    }

    #[test]
    fn test_invalid_reputation() {
        let mut config = TrackerConfig::default();
        config.min_reputation = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_weights() {
        let mut config = TrackerConfig::default();
        config.uptime_weight = 0.5;
        config.success_weight = 0.6;
        assert!(config.validate().is_err());
    }
}
