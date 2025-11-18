use std::time::Duration;

/// Distribution strategy for chunk downloads
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistributionStrategy {
    /// Download rarest chunks first (BitTorrent strategy)
    /// Helps distribute rare chunks quickly across the network
    RarestFirst,

    /// Download chunks sequentially from start to finish
    /// Useful for streaming media
    Sequential,

    /// Download chunks in random order
    /// Provides some load balancing without rarest-first complexity
    RandomOrder,
}

impl Default for DistributionStrategy {
    fn default() -> Self {
        Self::RarestFirst
    }
}

/// Configuration for chunk distribution
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Maximum number of concurrent content downloads
    pub max_concurrent_downloads: usize,

    /// Maximum number of concurrent chunk requests per peer
    pub chunks_per_peer: usize,

    /// Timeout for individual chunk requests
    pub request_timeout: Duration,

    /// Download strategy to use
    pub strategy: DistributionStrategy,

    /// Size of each chunk in bytes (256KB default, like BitTorrent)
    pub chunk_size: usize,

    /// Optimistic unchoking interval (give random peers a chance)
    pub optimistic_unchoke_interval: Duration,

    /// Minimum upload ratio before we start preferring peers
    pub min_upload_ratio: f64,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_concurrent_downloads: 10,
            chunks_per_peer: 4,
            request_timeout: Duration::from_secs(30),
            strategy: DistributionStrategy::RarestFirst,
            chunk_size: 256 * 1024, // 256 KB
            optimistic_unchoke_interval: Duration::from_secs(30),
            min_upload_ratio: 0.8, // Upload at least 80% of what we download
        }
    }
}

impl ChunkConfig {
    /// Create a new configuration builder
    pub fn builder() -> ChunkConfigBuilder {
        ChunkConfigBuilder::default()
    }
}

/// Builder for ChunkConfig
#[derive(Default)]
pub struct ChunkConfigBuilder {
    max_concurrent_downloads: Option<usize>,
    chunks_per_peer: Option<usize>,
    request_timeout: Option<Duration>,
    strategy: Option<DistributionStrategy>,
    chunk_size: Option<usize>,
    optimistic_unchoke_interval: Option<Duration>,
    min_upload_ratio: Option<f64>,
}

impl ChunkConfigBuilder {
    pub fn max_concurrent_downloads(mut self, val: usize) -> Self {
        self.max_concurrent_downloads = Some(val);
        self
    }

    pub fn chunks_per_peer(mut self, val: usize) -> Self {
        self.chunks_per_peer = Some(val);
        self
    }

    pub fn request_timeout(mut self, val: Duration) -> Self {
        self.request_timeout = Some(val);
        self
    }

    pub fn strategy(mut self, val: DistributionStrategy) -> Self {
        self.strategy = Some(val);
        self
    }

    pub fn chunk_size(mut self, val: usize) -> Self {
        self.chunk_size = Some(val);
        self
    }

    pub fn optimistic_unchoke_interval(mut self, val: Duration) -> Self {
        self.optimistic_unchoke_interval = Some(val);
        self
    }

    pub fn min_upload_ratio(mut self, val: f64) -> Self {
        self.min_upload_ratio = Some(val);
        self
    }

    pub fn build(self) -> ChunkConfig {
        let default = ChunkConfig::default();
        ChunkConfig {
            max_concurrent_downloads: self
                .max_concurrent_downloads
                .unwrap_or(default.max_concurrent_downloads),
            chunks_per_peer: self.chunks_per_peer.unwrap_or(default.chunks_per_peer),
            request_timeout: self.request_timeout.unwrap_or(default.request_timeout),
            strategy: self.strategy.unwrap_or(default.strategy),
            chunk_size: self.chunk_size.unwrap_or(default.chunk_size),
            optimistic_unchoke_interval: self
                .optimistic_unchoke_interval
                .unwrap_or(default.optimistic_unchoke_interval),
            min_upload_ratio: self.min_upload_ratio.unwrap_or(default.min_upload_ratio),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ChunkConfig::default();
        assert_eq!(config.max_concurrent_downloads, 10);
        assert_eq!(config.chunks_per_peer, 4);
        assert_eq!(config.chunk_size, 256 * 1024);
        assert_eq!(config.strategy, DistributionStrategy::RarestFirst);
    }

    #[test]
    fn test_config_builder() {
        let config = ChunkConfig::builder()
            .max_concurrent_downloads(5)
            .chunks_per_peer(8)
            .strategy(DistributionStrategy::Sequential)
            .build();

        assert_eq!(config.max_concurrent_downloads, 5);
        assert_eq!(config.chunks_per_peer, 8);
        assert_eq!(config.strategy, DistributionStrategy::Sequential);
    }
}
