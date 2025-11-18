use std::time::Duration;

/// Configuration for the Kademlia DHT
#[derive(Debug, Clone)]
pub struct DHTConfig {
    /// Number of nodes to replicate content provider records to (default: 20)
    pub replication_factor: usize,

    /// How long provider records remain valid (default: 24 hours)
    pub provider_timeout: Duration,

    /// How often to republish provider records (default: 12 hours)
    pub republish_interval: Duration,

    /// Maximum time to wait for a query to complete (default: 60 seconds)
    pub query_timeout: Duration,

    /// Number of peers to keep in each k-bucket (default: 20)
    pub k_value: usize,

    /// Maximum number of parallel queries (default: 10)
    pub parallelism: usize,

    /// Record TTL for Kademlia records (default: 24 hours)
    pub record_ttl: Duration,

    /// Interval for periodic routing table maintenance (default: 5 minutes)
    pub maintenance_interval: Duration,

    /// Enable automatic provider republishing
    pub auto_republish: bool,

    /// Maximum number of provider records to track locally
    pub max_local_providers: usize,
}

impl Default for DHTConfig {
    fn default() -> Self {
        Self {
            replication_factor: 20,
            provider_timeout: Duration::from_secs(24 * 3600), // 24 hours
            republish_interval: Duration::from_secs(12 * 3600), // 12 hours
            query_timeout: Duration::from_secs(60),           // 60 seconds
            k_value: 20,
            parallelism: 10,
            record_ttl: Duration::from_secs(24 * 3600), // 24 hours
            maintenance_interval: Duration::from_secs(5 * 60), // 5 minutes
            auto_republish: true,
            max_local_providers: 10000,
        }
    }
}

impl DHTConfig {
    /// Create a new DHT configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the replication factor (number of nodes to replicate to)
    pub fn with_replication_factor(mut self, factor: usize) -> Self {
        self.replication_factor = factor;
        self
    }

    /// Set the provider record timeout
    pub fn with_provider_timeout(mut self, timeout: Duration) -> Self {
        self.provider_timeout = timeout;
        self
    }

    /// Set the republish interval
    pub fn with_republish_interval(mut self, interval: Duration) -> Self {
        self.republish_interval = interval;
        self
    }

    /// Set the query timeout
    pub fn with_query_timeout(mut self, timeout: Duration) -> Self {
        self.query_timeout = timeout;
        self
    }

    /// Set the k-value (bucket size)
    pub fn with_k_value(mut self, k: usize) -> Self {
        self.k_value = k;
        self
    }

    /// Set the parallelism level
    pub fn with_parallelism(mut self, parallelism: usize) -> Self {
        self.parallelism = parallelism;
        self
    }

    /// Disable automatic republishing
    pub fn without_auto_republish(mut self) -> Self {
        self.auto_republish = false;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.replication_factor == 0 {
            return Err(ConfigError::InvalidReplicationFactor);
        }

        if self.k_value == 0 {
            return Err(ConfigError::InvalidKValue);
        }

        if self.parallelism == 0 {
            return Err(ConfigError::InvalidParallelism);
        }

        if self.query_timeout.as_secs() == 0 {
            return Err(ConfigError::InvalidQueryTimeout);
        }

        if self.provider_timeout < self.republish_interval {
            return Err(ConfigError::InvalidRepublishInterval);
        }

        Ok(())
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Replication factor must be greater than 0")]
    InvalidReplicationFactor,

    #[error("K-value must be greater than 0")]
    InvalidKValue,

    #[error("Parallelism must be greater than 0")]
    InvalidParallelism,

    #[error("Query timeout must be greater than 0")]
    InvalidQueryTimeout,

    #[error("Republish interval must be less than provider timeout")]
    InvalidRepublishInterval,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DHTConfig::default();
        assert_eq!(config.replication_factor, 20);
        assert_eq!(config.k_value, 20);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_builder_pattern() {
        let config = DHTConfig::new()
            .with_replication_factor(30)
            .with_k_value(25)
            .with_parallelism(15)
            .without_auto_republish();

        assert_eq!(config.replication_factor, 30);
        assert_eq!(config.k_value, 25);
        assert_eq!(config.parallelism, 15);
        assert!(!config.auto_republish);
    }

    #[test]
    fn test_invalid_config() {
        let mut config = DHTConfig::default();
        config.replication_factor = 0;
        assert!(config.validate().is_err());

        let mut config = DHTConfig::default();
        config.republish_interval = config.provider_timeout + Duration::from_secs(1);
        assert!(config.validate().is_err());
    }
}
