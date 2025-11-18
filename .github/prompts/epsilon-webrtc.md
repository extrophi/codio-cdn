## Agent: EPSILON - WebRTC Transfer Layer

**Duration:** 3 hours
**Branch:** `epsilon`
**Priority:** CRITICAL (core P2P transfer)

### Task
Implement WebRTC peer-to-peer data transfer for actual content delivery.

### Technical Reference
- `/docs/pm/TECHNICAL-PROPOSAL-PHASE-2.md` (Agent EPSILON section)

### Deliverables
- `crates/transfer/src/lib.rs` (300-400 lines)
- WebRTC peer connection management
- Data channel creation
- Chunked content transfer
- Multi-peer support (parallel downloads)
- Progress tracking

### Success Criteria
- ✅ Can establish WebRTC connection between 2 peers
- ✅ Can transfer file via data channel
- ✅ Can download from multiple peers (chunked)
- ✅ Progress tracking works
- ✅ All tests pass: `cargo test -p codio-transfer`

Update issue when complete.
