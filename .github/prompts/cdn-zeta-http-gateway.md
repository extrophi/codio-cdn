# Agent: CDN-ZETA - HTTP Gateway

**Duration:** 1.5 hours
**Branch:** `cdn-zeta`
**Priority:** HIGH (user-facing)
**Budget:** $20

## Mission

Implement an HTTP gateway that serves CDN content via standard HTTP, making the decentralized CDN a drop-in replacement for CloudFlare.

## Deliverables

### 1. Create `crates/codio-gateway/`

**File:** `crates/codio-gateway/src/main.rs` (~700 lines)

**Implementation Requirements:**

```rust
use axum::{
    Router,
    routing::{get, post},
    extract::{Path, Multipart, State},
    response::{IntoResponse, Json},
};

// Gateway state
#[derive(Clone)]
struct GatewayState {
    chunk_distributor: Arc<ChunkDistributor>,
    dht_manager: Arc<DHTManager>,
    cache: Arc<RwLock<LruCache<ContentId, Vec<u8>>>>,
}

// API routes
async fn upload_content(
    State(state): State<GatewayState>,
    multipart: Multipart,
) -> impl IntoResponse {
    // 1. Read file from multipart
    // 2. Generate CID
    // 3. Store in local cache
    // 4. Announce to DHT
    // 5. Return JSON response with CID
}

async fn download_content(
    State(state): State<GatewayState>,
    Path(cid): Path<String>,
) -> impl IntoResponse {
    // 1. Check local cache
    // 2. If not cached, find providers via DHT
    // 3. Download chunks from peers
    // 4. Verify integrity
    // 5. Cache locally
    // 6. Return content as bytes
}

async fn metrics(
    State(state): State<GatewayState>,
) -> impl IntoResponse {
    // Return Prometheus-compatible metrics
}

async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "timestamp": SystemTime::now(),
    }))
}
```

**Features to Implement:**

1. **HTTP API**
   - `POST /upload` - Upload file, get CID
   - `GET /{cid}` - Download content by CID
   - `GET /metrics` - Prometheus metrics
   - `GET /health` - Health check

2. **Local Cache**
   - LRU cache (100MB default)
   - Cache hot content
   - Automatic eviction

3. **Content-Type Detection**
   - Detect MIME type from content
   - Set appropriate `Content-Type` header
   - Support images, videos, HTML, etc.

4. **Error Handling**
   - 404 if CID not found
   - 500 for internal errors
   - 503 if no peers available
   - Proper HTTP status codes

### 2. Response Types

**File:** `crates/codio-gateway/src/response.rs`

```rust
#[derive(Serialize)]
pub struct UploadResponse {
    pub cid: String,
    pub size: u64,
    pub chunks: u32,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

#[derive(Serialize)]
pub struct MetricsResponse {
    pub uploads: u64,
    pub downloads: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_bytes_served: u64,
}
```

### 3. Configuration

**File:** `crates/codio-gateway/src/config.rs`

```rust
pub struct GatewayConfig {
    pub port: u16,
    pub cache_size: usize,  // bytes
    pub max_upload_size: u64,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            cache_size: 100 * 1024 * 1024,  // 100MB
            max_upload_size: 100 * 1024 * 1024,  // 100MB
        }
    }
}
```

### 4. Dependencies

**File:** `crates/codio-gateway/Cargo.toml`

```toml
[package]
name = "codio-gateway"
version = "0.1.0"
edition = "2021"

[dependencies]
codio-content = { path = "../codio-content" }
codio-network = { path = "../codio-network" }
codio-dht = { path = "../codio-dht" }
codio-chunk = { path = "../codio-chunk" }
codio-tracker = { path = "../codio-tracker" }

axum = "0.7"
tokio = { version = "1.35", features = ["full"] }
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tower-http = { version = "0.5", features = ["cors", "fs"] }
mime_guess = "2.0"
lru = "0.12"
```

### 5. Testing

**File:** `crates/codio-gateway/tests/integration_test.rs`

```rust
#[tokio::test]
async fn test_upload_and_download() {
    // Start gateway
    // Upload file via POST /upload
    // Get CID from response
    // Download via GET /{cid}
    // Verify content matches
}

#[tokio::test]
async fn test_cache() {
    // Download same CID twice
    // Verify second request is faster (cached)
}

#[tokio::test]
async fn test_404() {
    // Request non-existent CID
    // Verify 404 status code
}
```

### 6. CLI Usage

```bash
# Start gateway
cargo run --bin codio-gateway --port 8080

# Upload file
curl -X POST http://localhost:8080/upload \
  -F "file=@image.png"
# Response: {"cid": "QmXyz...", "size": 12345, "chunks": 1}

# Download file
curl http://localhost:8080/QmXyz... > downloaded.png

# Metrics
curl http://localhost:8080/metrics

# Health check
curl http://localhost:8080/health
```

## Success Criteria

- ✅ `cargo build` succeeds
- ✅ All tests pass (`cargo test`)
- ✅ Upload works via POST
- ✅ Download works via GET
- ✅ Cache improves performance
- ✅ Proper HTTP status codes

## Technical Constraints

- Use **Axum** web framework
- Port 8080 default (configurable)
- LRU cache for hot content
- Content-Type detection required
- No unsafe code

## Documentation

Include comprehensive docs:
- API reference
- Example curl commands
- Configuration options
- `README.md` with quick start

## Branch Workflow

1. Work on branch `cdn-zeta`
2. Commit frequently with clear messages
3. Run `cargo fmt && cargo clippy` before final commit
4. Create PR to `main` when complete
5. Tag issue #14 in PR description

## Resources

- Axum docs: https://docs.rs/axum/
- Nuclear Deployment Plan: `docs/pm/NUCLEAR-DEPLOYMENT-PLAN.md`

## Notes

The HTTP gateway is the **user-facing interface** of the CDN. It must be fast, reliable, and simple to use. Focus on:
- Clean API design
- Performance (caching)
- Error handling

**This is what users see. Make it beautiful.** ✨
