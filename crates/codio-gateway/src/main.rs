mod cache;
mod config;
mod metrics;
mod response;
mod storage;

use axum::{
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use cache::ContentCache;
use codio_content_id::ContentId;
use config::GatewayConfig;
use metrics::Metrics;
use response::{ErrorResponse, HealthResponse, MetricsResponse, UploadResponse};
use std::time::{SystemTime, UNIX_EPOCH};
use storage::ContentStorage;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Gateway state shared across handlers
#[derive(Clone)]
struct GatewayState {
    /// Content storage
    storage: ContentStorage,
    /// LRU cache for hot content
    cache: ContentCache,
    /// Metrics tracker
    metrics: Metrics,
    /// Configuration
    config: GatewayConfig,
}

impl GatewayState {
    /// Create new gateway state
    fn new(config: GatewayConfig) -> Self {
        Self {
            storage: ContentStorage::new(),
            cache: ContentCache::new(config.cache_size),
            metrics: Metrics::new(),
            config,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Load configuration
    let config = GatewayConfig::default();
    let port = config.port;

    tracing::info!("Starting Codio Gateway on port {}", port);
    tracing::info!("Cache size: {} MB", config.cache_size / (1024 * 1024));
    tracing::info!(
        "Max upload size: {} MB",
        config.max_upload_size / (1024 * 1024)
    );

    // Create gateway state
    let state = GatewayState::new(config);

    // Build router
    let app = Router::new()
        .route("/upload", post(upload_handler))
        .route("/:cid", get(download_handler))
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Gateway listening on {}", addr);
    tracing::info!("API endpoints:");
    tracing::info!("  POST /upload     - Upload file, get CID");
    tracing::info!("  GET  /<cid>      - Download content by CID");
    tracing::info!("  GET  /metrics    - Prometheus metrics");
    tracing::info!("  GET  /health     - Health check");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Upload content and get CID
///
/// Endpoint: POST /upload
/// Content-Type: multipart/form-data
/// Field: file
///
/// Returns: JSON with CID, size, and chunks
async fn upload_handler(
    State(state): State<GatewayState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    tracing::debug!("Upload request received");

    // Read file from multipart
    let mut content: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or("");
        if name == "file" {
            let data = field.bytes().await?;
            content = Some(data.to_vec());
            break;
        }
    }

    let content = content
        .ok_or_else(|| AppError::BadRequest("Missing 'file' field in multipart".to_string()))?;

    let size = content.len() as u64;

    // Check size limit
    if size > state.config.max_upload_size {
        return Err(AppError::BadRequest(format!(
            "File too large: {} bytes (max: {} bytes)",
            size, state.config.max_upload_size
        )));
    }

    // Store content and generate CID
    let cid = state
        .storage
        .store(content.clone())
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Cache the content
    state.cache.put(&cid, content).await;

    // Record metrics
    state.metrics.record_upload().await;

    tracing::info!("Uploaded content: {} ({} bytes)", cid, size);

    Ok(Json(UploadResponse::new(cid.to_string(), size)))
}

/// Download content by CID
///
/// Endpoint: GET /{cid}
///
/// Returns: Content bytes with appropriate Content-Type header
async fn download_handler(
    State(state): State<GatewayState>,
    Path(cid_str): Path<String>,
) -> Result<Response, AppError> {
    tracing::debug!("Download request for CID: {}", cid_str);

    // Parse CID
    let cid = ContentId::from_str(&cid_str)
        .map_err(|e| AppError::BadRequest(format!("Invalid CID: {}", e)))?;

    // Try cache first
    if let Some(content) = state.cache.get(&cid).await {
        tracing::debug!("Cache HIT for {}", cid);
        state.metrics.record_cache_hit().await;
        state.metrics.record_download(content.len() as u64).await;

        return Ok(build_response(&cid_str, content));
    }

    // Cache miss - try storage
    state.metrics.record_cache_miss().await;

    if let Some(content) = state.storage.retrieve(&cid).await {
        tracing::debug!("Storage HIT for {}", cid);

        // Add to cache for future requests
        state.cache.put(&cid, content.clone()).await;
        state.metrics.record_download(content.len() as u64).await;

        return Ok(build_response(&cid_str, content));
    }

    // Content not found
    tracing::warn!("Content not found: {}", cid);
    Err(AppError::NotFound(cid_str))
}

/// Build HTTP response with content
fn build_response(cid: &str, content: Vec<u8>) -> Response {
    // Detect MIME type
    let mime_type = detect_mime_type(cid, &content);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type)
        .header(header::CONTENT_LENGTH, content.len())
        .header("X-Content-ID", cid)
        .body(axum::body::Body::from(content))
        .unwrap()
}

/// Detect MIME type from content and filename
fn detect_mime_type(cid: &str, content: &[u8]) -> String {
    // Try to guess from content first
    let kind = infer::get(content);
    if let Some(k) = kind {
        return k.mime_type().to_string();
    }

    // Fallback to extension-based detection
    mime_guess::from_path(cid)
        .first()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string())
}

/// Get metrics
///
/// Endpoint: GET /metrics
///
/// Returns: JSON with metrics data
async fn metrics_handler(State(state): State<GatewayState>) -> Json<MetricsResponse> {
    let metrics = state.metrics.get().await;
    Json(metrics)
}

/// Health check
///
/// Endpoint: GET /health
///
/// Returns: JSON with status and timestamp
async fn health_handler() -> Json<HealthResponse> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp,
    })
}

/// Application error types
#[derive(Debug)]
enum AppError {
    BadRequest(String),
    NotFound(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_response) = match self {
            AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorResponse::new(msg, StatusCode::BAD_REQUEST.as_u16()),
            ),
            AppError::NotFound(cid) => (StatusCode::NOT_FOUND, ErrorResponse::not_found(&cid)),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse::internal_error(&msg),
            ),
        };

        (status, Json(error_response)).into_response()
    }
}

impl From<axum::extract::multipart::MultipartError> for AppError {
    fn from(err: axum::extract::multipart::MultipartError) -> Self {
        AppError::BadRequest(format!("Multipart error: {}", err))
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Internal(format!("IO error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_type_detection() {
        // PNG image
        let png_header = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_mime_type("test.png", &png_header), "image/png");

        // JPEG image
        let jpeg_header = vec![0xFF, 0xD8, 0xFF];
        assert_eq!(detect_mime_type("test.jpg", &jpeg_header), "image/jpeg");

        // Unknown content
        let unknown = vec![0x00, 0x01, 0x02];
        let mime = detect_mime_type("test.bin", &unknown);
        assert!(mime == "application/octet-stream" || mime.starts_with("application/"));
    }

    #[tokio::test]
    async fn test_gateway_state_creation() {
        let config = GatewayConfig::default();
        let state = GatewayState::new(config);

        assert_eq!(state.config.port, 8080);
        assert!(state.storage.is_empty().await);
        assert!(state.cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_upload_flow() {
        let config = GatewayConfig::default();
        let state = GatewayState::new(config);

        let content = b"Test content for upload".to_vec();
        let cid = state.storage.store(content.clone()).await.unwrap();

        assert!(state.storage.contains(&cid).await);
    }

    #[tokio::test]
    async fn test_cache_flow() {
        let config = GatewayConfig::default();
        let state = GatewayState::new(config);

        let content = b"Test content for cache".to_vec();
        let cid = ContentId::new(&content);

        state.cache.put(&cid, content.clone()).await;
        let retrieved = state.cache.get(&cid).await;

        assert_eq!(retrieved, Some(content));
    }
}
