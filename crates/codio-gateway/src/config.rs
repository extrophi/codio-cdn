/// Gateway configuration
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    /// HTTP server port
    pub port: u16,
    /// Cache size in bytes
    pub cache_size: usize,
    /// Maximum upload size in bytes
    pub max_upload_size: u64,
    /// DHT listen address
    pub dht_addr: String,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            cache_size: 100 * 1024 * 1024,      // 100MB
            max_upload_size: 100 * 1024 * 1024, // 100MB
            dht_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
        }
    }
}

impl GatewayConfig {
    /// Create new configuration with custom values
    pub fn new(port: u16, cache_size: usize, max_upload_size: u64) -> Self {
        Self {
            port,
            cache_size,
            max_upload_size,
            dht_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
        }
    }
}
