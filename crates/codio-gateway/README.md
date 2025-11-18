# Codio Gateway

HTTP gateway for the Codio decentralized CDN. Provides a standard HTTP interface for uploading and downloading content via content-addressed identifiers (CIDs).

## Features

- **HTTP API** - RESTful API for content upload/download
- **Content Addressing** - IPFS-compatible CIDv0 identifiers
- **LRU Cache** - Automatic caching of hot content (100MB default)
- **Content-Type Detection** - Automatic MIME type detection
- **Metrics** - Prometheus-compatible metrics endpoint
- **Health Checks** - Health endpoint for monitoring

## Quick Start

### Start the Gateway

```bash
cargo run --bin codio-gateway
```

The gateway will start on port 8080 by default.

### Upload a File

```bash
curl -X POST http://localhost:8080/upload \
  -F "file=@/path/to/file.png"
```

Response:
```json
{
  "cid": "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG",
  "size": 12345,
  "chunks": 1
}
```

### Download a File

```bash
curl http://localhost:8080/QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG \
  > downloaded.png
```

The gateway automatically detects the content type and sets the appropriate `Content-Type` header.

### Check Metrics

```bash
curl http://localhost:8080/metrics
```

Response:
```json
{
  "uploads": 42,
  "downloads": 128,
  "cache_hits": 95,
  "cache_misses": 33,
  "total_bytes_served": 1048576
}
```

### Health Check

```bash
curl http://localhost:8080/health
```

Response:
```json
{
  "status": "healthy",
  "timestamp": 1699564800
}
```

## API Reference

### POST /upload

Upload content and receive a CID.

**Request:**
- Method: `POST`
- Content-Type: `multipart/form-data`
- Body: Form field `file` with file data

**Response:**
```json
{
  "cid": "QmXyz...",
  "size": 12345,
  "chunks": 1
}
```

**Status Codes:**
- `200 OK` - Upload successful
- `400 Bad Request` - Invalid request or file too large
- `500 Internal Server Error` - Server error

### GET /{cid}

Download content by CID.

**Request:**
- Method: `GET`
- Path: `/{cid}` where `{cid}` is a valid content identifier

**Response:**
- Body: Raw content bytes
- Headers:
  - `Content-Type`: Detected MIME type
  - `Content-Length`: Content size in bytes
  - `X-Content-ID`: The requested CID

**Status Codes:**
- `200 OK` - Content found and returned
- `400 Bad Request` - Invalid CID format
- `404 Not Found` - Content not found
- `500 Internal Server Error` - Server error

### GET /metrics

Get gateway metrics.

**Response:**
```json
{
  "uploads": 0,
  "downloads": 0,
  "cache_hits": 0,
  "cache_misses": 0,
  "total_bytes_served": 0
}
```

### GET /health

Health check endpoint.

**Response:**
```json
{
  "status": "healthy",
  "timestamp": 1699564800
}
```

## Configuration

Configuration options (currently hardcoded, Phase 2 will add CLI args):

```rust
GatewayConfig {
    port: 8080,                       // HTTP server port
    cache_size: 100 * 1024 * 1024,    // 100MB cache
    max_upload_size: 100 * 1024 * 1024, // 100MB max upload
    dht_addr: "/ip4/0.0.0.0/tcp/0",   // DHT listen address
}
```

## Architecture

```
┌─────────────────┐
│  HTTP Client    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   Axum Server   │
│   (Port 8080)   │
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
┌───────┐ ┌───────────┐
│ Cache │ │  Storage  │
│ (LRU) │ │ (In-mem)  │
└───────┘ └───────────┘
```

## Testing

Run tests:

```bash
# Run all tests
cargo test -p codio-gateway

# Run integration tests only
cargo test -p codio-gateway --test integration_test
```

## Examples

### Upload and Download Workflow

```bash
# 1. Upload a file
RESPONSE=$(curl -s -X POST http://localhost:8080/upload \
  -F "file=@image.png")

# 2. Extract CID from response
CID=$(echo $RESPONSE | jq -r '.cid')

# 3. Download using CID
curl http://localhost:8080/$CID > downloaded.png

# 4. Verify files match
diff image.png downloaded.png
```

### Monitoring Cache Performance

```bash
# Check cache hit rate
curl http://localhost:8080/metrics | jq '{
  cache_hit_rate: (.cache_hits / (.cache_hits + .cache_misses) * 100)
}'
```

## Dependencies

- **axum** - Web framework
- **tokio** - Async runtime
- **tower-http** - CORS and tracing middleware
- **lru** - LRU cache implementation
- **mime_guess** - MIME type detection
- **infer** - Content-based file type detection
- **codio-content-id** - CID generation
- **codio-common** - Shared types
- **codio-dht** - DHT integration (Phase 2)

## Roadmap

**Phase 1 (Current):**
- ✅ HTTP API (upload/download)
- ✅ Content addressing (CIDs)
- ✅ LRU cache
- ✅ Metrics endpoint

**Phase 2:**
- [ ] DHT integration for peer discovery
- [ ] Chunked transfers for large files
- [ ] Configuration via CLI/env vars
- [ ] Rate limiting
- [ ] Authentication

**Phase 3:**
- [ ] Token economics integration
- [ ] Payment channels
- [ ] Provider incentives

## License

MIT
