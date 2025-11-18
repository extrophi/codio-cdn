use serde::{Deserialize, Serialize};

/// Response for successful upload
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    /// Content identifier
    pub cid: String,
    /// Size in bytes
    pub size: u64,
    /// Number of chunks (always 1 for Phase 1)
    pub chunks: u32,
}

impl UploadResponse {
    pub fn new(cid: String, size: u64) -> Self {
        Self {
            cid,
            size,
            chunks: 1, // Phase 1: no chunking
        }
    }
}

/// Error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
    /// HTTP status code
    pub code: u16,
}

impl ErrorResponse {
    pub fn new(error: String, code: u16) -> Self {
        Self { error, code }
    }

    pub fn not_found(cid: &str) -> Self {
        Self {
            error: format!("Content not found: {}", cid),
            code: 404,
        }
    }

    pub fn internal_error(msg: &str) -> Self {
        Self {
            error: msg.to_string(),
            code: 500,
        }
    }

    pub fn service_unavailable(msg: &str) -> Self {
        Self {
            error: msg.to_string(),
            code: 503,
        }
    }
}

/// Metrics response
#[derive(Debug, Serialize, Deserialize, Clone)]
#[derive(Default)]
pub struct MetricsResponse {
    /// Total uploads
    pub uploads: u64,
    /// Total downloads
    pub downloads: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Total bytes served
    pub total_bytes_served: u64,
}


/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Status string
    pub status: String,
    /// Timestamp (seconds since epoch)
    pub timestamp: u64,
}
