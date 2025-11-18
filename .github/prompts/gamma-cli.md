## Agent: GAMMA - CLI Tool

**Duration:** 2 hours
**Branch:** `gamma`
**Dependencies:** ALPHA, BETA

### Task
Build command-line interface for publishing and retrieving content.

### Technical Reference
- `/docs/pm/TECHNICAL-PROPOSAL-PHASE-1.md` (lines 382-550)

### Deliverables
- `crates/cli/src/main.rs` (200-300 lines)
- `codio-cdn publish <file>` command
- `codio-cdn get <cid>` command
- `codio-cdn hash <file>` command
- Colored output

### Implementation
See TECHNICAL-PROPOSAL-PHASE-1.md for complete CLI code.

### Success Criteria
- ✅ Can run: `cargo run -p codio-cdn -- publish test.txt`
- ✅ Returns valid CID
- ✅ Can hash files without publishing
- ✅ Help text works: `codio-cdn --help`

### Commands
```bash
git checkout gamma

# Create binary crate
cargo new crates/cli

# Implement per TECHNICAL-PROPOSAL
# (See proposal for full code)

# Test
cargo run -p codio-cdn -- hash README.md
cargo run -p codio-cdn -- --help

# Commit
git add crates/cli/
git commit -m "feat(cli): Add codio-cdn command-line tool"
git push origin gamma
```

Update issue when complete.
