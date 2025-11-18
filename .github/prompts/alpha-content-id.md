## Agent: ALPHA - Content Addressing Library

**Duration:** 2 hours
**Branch:** `alpha`
**Priority:** CRITICAL (blocks others)

### Task
Implement content-addressed storage using sha256 hashing and IPFS-compatible CID format.

### Technical Reference
- `/docs/pm/TECHNICAL-PROPOSAL-PHASE-1.md` (lines 46-180)

### Deliverables
- `crates/content-id/src/lib.rs` (150-200 lines)
- `crates/common/src/lib.rs` (50-100 lines)
- Unit tests (95%+ coverage)
- Benchmarks (criterion)

### Implementation
See TECHNICAL-PROPOSAL-PHASE-1.md for complete Rust code examples.

### Success Criteria
- ✅ Can generate CIDs from arbitrary content
- ✅ CID verification works (hash comparison)
- ✅ String parsing (from_str) works
- ✅ Same content produces same CID
- ✅ All tests pass: `cargo test -p codio-content-id`

### Commands
```bash
cd /Users/kjd/01-projects/IAC-034-codio-cdn
git checkout alpha

# Create crate structure
cargo new --lib crates/content-id
cargo new --lib crates/common

# Implement per TECHNICAL-PROPOSAL
# (See proposal for full code)

# Test
cargo test -p codio-content-id -p codio-common

# Commit
git add crates/
git commit -m "feat(content-id): Implement sha256 CID generation"
git push origin alpha
```

Update issue when complete.
