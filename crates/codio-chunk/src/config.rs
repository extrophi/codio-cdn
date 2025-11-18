use std::time::Duration;

/// Distribution strategy for chunk downloads
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistributionStrategy {
    /// Download rarest chunks first (BitTorrent strategy)
    /// Helps distribute rare chunks quickly across the network
    RarestFirst,

    /// Download chunks in sequential order
    Sequential,

    /// Download chunks in random order
    RandomOrder,
}

/// Configuration for chunk distribution
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Maximum number of concurrent downloads
    pub max_concurrent_downloads: usize,

    /// Maximum number of chunk requests per peer
    pub chunks_per_peer: usize,

    /// Timeout for chunk requests
    pub request_timeout: Duration,

    /// Distribution strategy to use
    pub strategy: DistributionStrategy,

    /// Chunk size in bytes (default: 256 KB)
    pub chunk_size: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_concurrent_downloads: 10,
            chunks_per_peer: 4,
            request_timeout: Duration::from_secs(30),
            strategy: DistributionStrategy::RarestFirst,
            chunk_size: 256 * 1024, // 256 KB
        }
    }
}

impl ChunkConfig {
    /// Create a new configuration with custom values
    pub fn new(
        max_concurrent_downloads: usize,
        chunks_per_peer: usize,
        request_timeout: Duration,
        strategy: DistributionStrategy,
    ) -> Self {
        Self {
            max_concurrent_downloads,
            chunks_per_peer,
            request_timeout,
            strategy,
            chunk_size: 256 * 1024,
        }
    }

    /// Set chunk size
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }
}
