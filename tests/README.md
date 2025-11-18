# Codio CDN Test Suite

## Quick Start

```bash
# Run all tests
cargo test --workspace --verbose

# Run Phase 1 tests only
cargo test --test integration_test
cargo test --test cli_test

# Run Phase 2 tests (will skip if components not implemented)
cargo test --test phase2_integration

# Run browser tests
cd browser && npm test
```

## Test Organization

### Phase 1 Tests (Current)

- `integration_test.rs` - CID and DHT integration tests
- `cli_test.rs` - CLI tool tests

**Status:** ✅ Complete | All tests passing

### Phase 2 Tests (Future)

- `phase2_integration.rs` - Main entry point
- `phase2/test_webrtc_transfer.rs` - WebRTC P2P transfer tests
- `phase2/test_gateway_fallback.rs` - HTTPS gateway fallback tests
- `phase2/test_full_integration.rs` - End-to-end integration tests
- `browser/tests/*.test.js` - Browser WASM and Service Worker tests

**Status:** 📝 Structure Complete | ⏸️ Awaiting Implementation

## Coverage

Current coverage: **>80%** (Phase 1)

```bash
# Generate coverage report
cargo install cargo-tarpaulin
cargo tarpaulin --workspace --out Html
open tarpaulin-report.html
```

## Documentation

See [INTEGRATION-TESTS.md](../docs/INTEGRATION-TESTS.md) for:
- Detailed test descriptions
- Running instructions
- Performance targets
- Contributing guidelines
- Troubleshooting

## CI/CD

All tests run automatically on GitHub Actions:
- Push to main/develop
- Pull requests
- Scheduled nightly runs

View: `.github/workflows/phase2-integration-tests.yml`

## Test Philosophy

1. **Test structure exists before implementation**
   - Tests document expected behavior
   - Placeholders allow CI to pass
   - Easy to update as code is built

2. **Integration over unit tests**
   - Test components working together
   - Verify real-world scenarios
   - End-to-end user journeys

3. **Performance is a feature**
   - All tests verify performance targets
   - Benchmarks run in CI
   - Regressions caught early

## Quick Reference

| Task | Command |
|------|---------|
| Run all tests | `cargo test --workspace` |
| Run Phase 1 tests | `cargo test --test integration_test` |
| Run Phase 2 tests | `cargo test --test phase2_integration` |
| Run browser tests | `cd browser && npm test` |
| Coverage report | `cargo tarpaulin --workspace --out Html` |
| Single test | `cargo test test_name -- --exact` |
| Verbose output | `cargo test -- --nocapture` |

---

For detailed documentation, see [INTEGRATION-TESTS.md](../docs/INTEGRATION-TESTS.md)
