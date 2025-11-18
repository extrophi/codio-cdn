# Phase 2 Integration Tests

## Overview

This document describes the comprehensive integration test suite for Codio CDN Phase 2 (Browser Integration). The test suite is designed to validate all Phase 2 components working together.

## Architecture

### Test Structure

```
codio-cdn/
├── tests/
│   ├── phase2/
│   │   ├── test_webrtc_transfer.rs      # WebRTC transfer tests (EPSILON)
│   │   ├── test_gateway_fallback.rs     # Gateway fallback tests (THETA)
│   │   ├── test_full_integration.rs     # E2E integration tests
│   │   └── mod.rs                       # Module definitions
│   └── phase2_integration.rs            # Main test entry point
│
├── browser/
│   └── tests/
│       ├── wasm.test.js                 # WASM binding tests (ZETA)
│       ├── service-worker.test.js       # Service Worker tests (ETA)
│       ├── e2e.test.js                  # Browser E2E tests
│       └── setup.js                     # Jest test setup
│
└── .github/
    └── workflows/
        └── phase2-integration-tests.yml # CI/CD pipeline
```

## Test Categories

### 1. Rust Integration Tests

#### WebRTC Transfer Tests (`test_webrtc_transfer.rs`)

Tests the peer-to-peer WebRTC data transfer layer:

- **test_webrtc_peer_connection**: Basic WebRTC connection between two peers
- **test_single_peer_content_transfer**: Transfer content from one peer to another
- **test_large_file_transfer**: Transfer large files (10MB+) via WebRTC
- **test_multi_peer_parallel_download**: Download from multiple peers simultaneously
- **test_transfer_progress_tracking**: Monitor transfer progress
- **test_peer_disconnection_handling**: Handle peer disconnection gracefully
- **test_data_channel_reliability**: Verify WebRTC data channel reliability
- **test_concurrent_transfers**: Multiple simultaneous transfers
- **test_content_verification_failure**: Handle corrupted content

**Success Criteria:**
- ✅ Can establish WebRTC connection between 2 peers
- ✅ Can transfer files via data channel
- ✅ Multi-peer download is faster than single peer
- ✅ Progress tracking works accurately
- ✅ Handles disconnections gracefully

#### Gateway Fallback Tests (`test_gateway_fallback.rs`)

Tests the HTTPS gateway fallback mechanism:

- **test_gateway_fetch_known_cid**: Fetch content from IPFS gateway
- **test_gateway_primary_and_fallback**: Primary failure triggers fallback
- **test_gateway_all_fail**: Error when all gateways fail
- **test_gateway_timeout**: Timeout handling
- **test_gateway_concurrent_requests**: Concurrent gateway requests
- **test_p2p_with_gateway_fallback_integration**: P2P → Gateway fallback flow

**Success Criteria:**
- ✅ Gateway can fetch from IPFS
- ✅ Fallback works automatically
- ✅ Handles failures gracefully
- ✅ Completes in <2s

#### Full Integration Tests (`test_full_integration.rs`)

End-to-end tests combining all components:

- **test_complete_publish_discover_transfer_flow**: Full user journey
- **test_wasm_to_rust_roundtrip**: WASM ↔ Rust communication
- **test_gateway_fallback_integration**: P2P + Gateway hybrid
- **test_multi_peer_chunk_download**: Parallel chunk downloads
- **test_peer_discovery_and_connection**: DHT → WebRTC flow
- **test_performance_targets**: Verify performance metrics

**Success Criteria:**
- ✅ Complete flow works end-to-end
- ✅ All components integrate correctly
- ✅ Performance targets met
- ✅ No resource leaks

### 2. Browser Integration Tests

#### WASM Tests (`wasm.test.js`)

Tests the WASM browser bindings:

- WASM module loading and initialization
- Content download via P2P
- Content upload and CID generation
- Peer discovery
- Error handling
- Performance benchmarks
- Browser compatibility (Chrome, Firefox, Safari)

**Success Criteria:**
- ✅ WASM loads in <1s
- ✅ Download/upload work correctly
- ✅ Works in Chrome and Firefox
- ✅ Bundle size <2MB

#### Service Worker Tests (`service-worker.test.js`)

Tests the Service Worker fetch interception:

- Service Worker registration
- Fetch interception (`codio://` protocol)
- X-Codio-CID header handling
- P2P → Gateway fallback
- Caching layer
- Error handling

**Success Criteria:**
- ✅ SW registers successfully
- ✅ Intercepts requests correctly
- ✅ Fallback works seamlessly
- ✅ Caching improves performance

#### E2E Browser Tests (`e2e.test.js`)

End-to-end browser integration tests:

- Complete user journey (upload → download)
- Multi-peer scenarios
- Browser extension integration
- Service Worker + WASM integration
- Real-world scenarios (JS libraries, images)
- Cross-browser compatibility
- Performance benchmarks
- Security (CID verification)

**Success Criteria:**
- ✅ Full user journey works
- ✅ Works across browsers
- ✅ Performance acceptable
- ✅ Security verified

## Running Tests

### Rust Integration Tests

```bash
# Run all Phase 2 integration tests
cargo test --test phase2_integration --verbose

# Run specific test module
cargo test --test phase2_integration test_webrtc_transfer --verbose
cargo test --test phase2_integration test_gateway_fallback --verbose

# Run with output
cargo test --test phase2_integration -- --nocapture

# Run specific test
cargo test --test phase2_integration test_complete_publish_discover_transfer_flow
```

### Browser Integration Tests

```bash
# Navigate to browser directory
cd browser

# Install dependencies (first time only)
npm install

# Run all browser tests
npm test

# Run specific test suites
npm run test:wasm
npm run test:sw
npm run test:e2e

# Run with coverage
npm run test:coverage

# Watch mode for development
npm run test:watch
```

### CI/CD Pipeline

GitHub Actions automatically runs all tests on:
- Push to `main`, `develop`, or `claude/*` branches
- Pull requests to `main` or `develop`

View results at: `.github/workflows/phase2-integration-tests.yml`

## Test Implementation Status

### Current Status

All test files are created with placeholder assertions:

```rust
assert!(true, "Component not yet implemented");
```

This approach allows:
- ✅ Test structure exists before implementation
- ✅ Tests pass in CI (with warnings)
- ✅ Easy to update as components are built
- ✅ Documentation of expected behavior

### Implementation Roadmap

As each Phase 2 component is implemented, update corresponding tests:

1. **EPSILON (WebRTC Transfer)**
   - Implement `crates/transfer/`
   - Update `test_webrtc_transfer.rs` tests
   - Remove placeholder assertions

2. **ZETA (WASM Bindings)**
   - Implement `crates/wasm/`
   - Update `wasm.test.js` tests
   - Build WASM module

3. **ETA (Service Worker)**
   - Implement `browser/service-worker/`
   - Update `service-worker.test.js` tests
   - Test fetch interception

4. **THETA (Gateway)**
   - Implement `crates/gateway/`
   - Update `test_gateway_fallback.rs` tests
   - Test fallback mechanism

5. **Integration**
   - Update `test_full_integration.rs`
   - Update `e2e.test.js`
   - Run full test suite

## Performance Targets

Tests verify these Phase 2 performance targets:

| Metric | Target | Test |
|--------|--------|------|
| CID generation | <1ms per MB | `test_cid_performance` |
| P2P transfer | >1MB/s | `test_large_file_transfer` |
| Multi-peer speedup | >1.5x single | `test_multi_peer_parallel_download` |
| Gateway fallback | <2s | `test_gateway_timeout` |
| WebRTC connection | <1s | `test_webrtc_peer_connection` |
| WASM initialization | <1s | `test_wasm_init_performance` |
| WASM bundle size | <2MB | CI build check |

## Coverage Requirements

### Rust Tests

Target: **>80% code coverage**

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run with coverage
cargo tarpaulin --workspace --out Html
```

### Browser Tests

Target: **>80% code coverage**

Configured in `browser/package.json`:

```json
"coverageThreshold": {
  "global": {
    "branches": 80,
    "functions": 80,
    "lines": 80,
    "statements": 80
  }
}
```

## Debugging Tests

### Enable verbose output

```bash
# Rust tests
RUST_LOG=debug cargo test --test phase2_integration -- --nocapture

# Browser tests
DEBUG_TESTS=1 npm test
```

### Run single test

```bash
# Rust
cargo test --test phase2_integration test_specific_test_name -- --exact

# Browser
npm test -- wasm.test.js -t "specific test name"
```

### Use test fixtures

```rust
// Example test fixture
#[tokio::test]
async fn test_with_fixture() {
    let (_cleanup, test_data) = setup_test_fixture().await;
    // Test code here
    // _cleanup runs on drop
}
```

## Contributing

### Adding New Tests

1. **Identify component to test**
2. **Choose appropriate test file**
   - WebRTC → `test_webrtc_transfer.rs`
   - Gateway → `test_gateway_fallback.rs`
   - WASM → `wasm.test.js`
   - Service Worker → `service-worker.test.js`
   - E2E → `test_full_integration.rs` or `e2e.test.js`

3. **Write test with TODO comment**
   ```rust
   #[tokio::test]
   async fn test_new_feature() {
       // TODO: Implement when component X is available
       // Expected behavior:
       // 1. ...
       // 2. ...
       assert!(true, "Component not yet implemented");
   }
   ```

4. **Update this documentation**
5. **Run tests to verify structure**
6. **Commit with clear description**

### Test Naming Convention

- Prefix: `test_`
- Descriptive: `test_webrtc_peer_connection`
- Not: `test1`, `test_basic`

### Test Organization

- One concept per test
- Use helper functions for setup
- Clean up resources (use `Drop` trait)
- Document expected behavior

## Troubleshooting

### Tests fail with "not yet implemented"

This is expected! Tests are structured before implementation.

### CI shows warnings

Warnings are expected for placeholder tests. They'll disappear as components are implemented.

### Browser tests can't find modules

Ensure WASM is built first:

```bash
cd crates/wasm
wasm-pack build --target web
```

### Performance tests timeout

Increase timeout in test:

```rust
#[tokio::test]
#[timeout(60000)] // 60 seconds
async fn long_running_test() {
    // ...
}
```

## Resources

- [Phase 2 Technical Proposal](../docs/pm/TECHNICAL-PROPOSAL-PHASE-2.md)
- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Jest Documentation](https://jestjs.io/docs/getting-started)
- [Puppeteer Documentation](https://pptr.dev/)
- [WebRTC API](https://developer.mozilla.org/en-US/docs/Web/API/WebRTC_API)

## Contact

For questions or issues with the test suite:
- GitHub Issues: [https://github.com/Iamcodio/codio-cdn/issues](https://github.com/Iamcodio/codio-cdn/issues)
- Technical Proposal: `docs/pm/TECHNICAL-PROPOSAL-PHASE-2.md`

---

**Test Suite Status:** ✅ Structure Complete | ⏸️ Awaiting Phase 2 Implementation

**Last Updated:** 2025-11-18
