## Agent: BETA - Kademlia DHT Implementation

**Duration:** 3 hours
**Branch:** `beta`
**Dependencies:** ALPHA (uses ContentId)

### Task
Implement Kademlia DHT for peer discovery using libp2p.

### Technical Reference
- `/docs/pm/TECHNICAL-PROPOSAL-PHASE-1.md` (lines 182-380)

### Deliverables
- `crates/dht/src/lib.rs` (300-400 lines)
- Bootstrap node connection
- Content announcement (provide)
- Provider discovery (find)
- Event handling

### Implementation
See TECHNICAL-PROPOSAL-PHASE-1.md for complete libp2p integration code.

### Success Criteria
- ✅ Can create DHT node
- ✅ Can connect to bootstrap peers
- ✅ Can announce content (start_providing)
- ✅ Can find providers (get_providers)
- ✅ All tests pass: `cargo test -p codio-dht`

### Commands
```bash
git checkout beta

# Create crate
cargo new --lib crates/dht

# Implement per TECHNICAL-PROPOSAL
# (See proposal for full code)

# Test
cargo test -p codio-dht

# Commit
git add crates/dht/
git commit -m "feat(dht): Implement Kademlia peer discovery"
git push origin beta
```

Update issue when complete.
