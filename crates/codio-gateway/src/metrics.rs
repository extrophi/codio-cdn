use crate::response::MetricsResponse;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Metrics tracker for gateway statistics
#[derive(Clone)]
pub struct Metrics {
    data: Arc<RwLock<MetricsData>>,
}

#[derive(Debug, Clone)]
#[derive(Default)]
struct MetricsData {
    uploads: u64,
    downloads: u64,
    cache_hits: u64,
    cache_misses: u64,
    total_bytes_served: u64,
}


impl Metrics {
    /// Create new metrics tracker
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(MetricsData::default())),
        }
    }

    /// Record an upload
    pub async fn record_upload(&self) {
        let mut data = self.data.write().await;
        data.uploads += 1;
    }

    /// Record a download
    pub async fn record_download(&self, bytes: u64) {
        let mut data = self.data.write().await;
        data.downloads += 1;
        data.total_bytes_served += bytes;
    }

    /// Record a cache hit
    pub async fn record_cache_hit(&self) {
        let mut data = self.data.write().await;
        data.cache_hits += 1;
    }

    /// Record a cache miss
    pub async fn record_cache_miss(&self) {
        let mut data = self.data.write().await;
        data.cache_misses += 1;
    }

    /// Get current metrics as response
    pub async fn get(&self) -> MetricsResponse {
        let data = self.data.read().await;
        MetricsResponse {
            uploads: data.uploads,
            downloads: data.downloads,
            cache_hits: data.cache_hits,
            cache_misses: data.cache_misses,
            total_bytes_served: data.total_bytes_served,
        }
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        let mut data = self.data.write().await;
        *data = MetricsData::default();
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_upload() {
        let metrics = Metrics::new();
        metrics.record_upload().await;
        metrics.record_upload().await;

        let response = metrics.get().await;
        assert_eq!(response.uploads, 2);
    }

    #[tokio::test]
    async fn test_metrics_download() {
        let metrics = Metrics::new();
        metrics.record_download(1024).await;
        metrics.record_download(2048).await;

        let response = metrics.get().await;
        assert_eq!(response.downloads, 2);
        assert_eq!(response.total_bytes_served, 3072);
    }

    #[tokio::test]
    async fn test_metrics_cache() {
        let metrics = Metrics::new();
        metrics.record_cache_hit().await;
        metrics.record_cache_miss().await;
        metrics.record_cache_miss().await;

        let response = metrics.get().await;
        assert_eq!(response.cache_hits, 1);
        assert_eq!(response.cache_misses, 2);
    }

    #[tokio::test]
    async fn test_metrics_reset() {
        let metrics = Metrics::new();
        metrics.record_upload().await;
        metrics.reset().await;

        let response = metrics.get().await;
        assert_eq!(response.uploads, 0);
    }
}
