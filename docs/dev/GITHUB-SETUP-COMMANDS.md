# CODIO CDN GITHUB SETUP - COMPLETE COMMANDS
## Single Paste Block for Repository Setup

**Copy this entire block and paste into terminal**

```bash
# Navigate to project
cd /Users/kjd/01-projects/IAC-034-codio-cdn

# ===== 1. CREATE GITHUB REPOSITORY =====
gh repo create extrophi/codio-cdn \
  --public \
  --description "Decentralized CDN - Phase 1 Foundation" \
  --clone=false

# ===== 2. INITIALIZE GIT =====
git init
git add .
git commit -m "docs: Initial commit with Phase 1 technical proposal"
git branch -M main
git remote add origin https://github.com/extrophi/codio-cdn.git
git push -u origin main

# ===== 3. CREATE BRANCHES =====
git branch alpha
git branch beta
git branch gamma
git branch delta

git push origin alpha beta gamma delta

# ===== 4. CREATE LABELS =====
gh label create "agent:alpha" --color "0075ca" --force
gh label create "agent:beta" --color "1d76db" --force
gh label create "agent:gamma" --color "0e8a16" --force
gh label create "agent:delta" --color "fbca04" --force
gh label create "phase:1" --color "d93f0b" --force
gh label create "status:pending" --color "ededed" --force
gh label create "status:in-progress" --color "fbca04" --force
gh label create "status:review" --color "0e8a16" --force

# ===== 5. CREATE PROMPT FILES =====
mkdir -p .github/prompts

# ALPHA prompt
cat > .github/prompts/alpha-content-id.md << 'EOF'
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
EOF

# BETA prompt
cat > .github/prompts/beta-dht.md << 'EOF'
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
EOF

# GAMMA prompt
cat > .github/prompts/gamma-cli.md << 'EOF'
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
EOF

# DELTA prompt
cat > .github/prompts/delta-tests.md << 'EOF'
## Agent: DELTA - Testing & Documentation

**Duration:** 1.5 hours  
**Branch:** `delta`  
**Dependencies:** ALPHA, BETA, GAMMA  

### Task
Create comprehensive test suite and documentation.

### Technical Reference
- `/docs/pm/TECHNICAL-PROPOSAL-PHASE-1.md` (lines 552-680)

### Deliverables
- `tests/integration_test.rs` (100-150 lines)
- `tests/cli_test.rs` (50-100 lines)
- `README.md` (comprehensive docs)
- CI/CD workflow

### Implementation
See TECHNICAL-PROPOSAL-PHASE-1.md for complete test code.

### Success Criteria
- ✅ All tests pass: `cargo test --workspace`
- ✅ Test coverage > 80%
- ✅ README has usage examples
- ✅ CLI integration tests pass

### Commands
```bash
git checkout delta

# Create test files
mkdir -p tests
# (Create test files per TECHNICAL-PROPOSAL)

# Create README
# (See proposal for content)

# Run all tests
cargo test --workspace

# Commit
git add tests/ README.md
git commit -m "test: Add integration and CLI tests"
git push origin delta
```

Update issue when complete.
EOF

# ===== 6. CREATE GITHUB ACTIONS =====
mkdir -p .github/workflows

cat > .github/workflows/rust-ci.yml << 'EOF'
name: Rust CI

on:
  push:
    branches: [main, alpha, beta, gamma, delta]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      
      - name: Cache cargo
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Check formatting
        run: cargo fmt --all -- --check
      
      - name: Run clippy
        run: cargo clippy --workspace -- -D warnings
      
      - name: Run tests
        run: cargo test --workspace --verbose
      
      - name: Build release
        run: cargo build --release --workspace
EOF

# ===== 7. CREATE GITHUB ISSUES =====

# ALPHA issue
gh issue create \
  --title "[ALPHA] Content Addressing Library - Phase 1 Foundation" \
  --body "$(cat .github/prompts/alpha-content-id.md)" \
  --label "agent:alpha,phase:1,status:pending"

# BETA issue
gh issue create \
  --title "[BETA] Kademlia DHT Implementation - Phase 1 Foundation" \
  --body "$(cat .github/prompts/beta-dht.md)" \
  --label "agent:beta,phase:1,status:pending"

# GAMMA issue
gh issue create \
  --title "[GAMMA] CLI Tool - Phase 1 Foundation" \
  --body "$(cat .github/prompts/gamma-cli.md)" \
  --label "agent:gamma,phase:1,status:pending"

# DELTA issue
gh issue create \
  --title "[DELTA] Testing & Documentation - Phase 1 Foundation" \
  --body "$(cat .github/prompts/delta-tests.md)" \
  --label "agent:delta,phase:1,status:pending"

# ===== 8. COMMIT GITHUB FILES =====
git add .github/
git commit -m "ci: Add GitHub Actions workflow and issue prompts"
git push origin main

# ===== 9. VERIFICATION =====
echo ""
echo "===== SETUP COMPLETE ====="
echo ""
echo "Repository: https://github.com/extrophi/codio-cdn"
echo "Issues: https://github.com/extrophi/codio-cdn/issues"
echo "Actions: https://github.com/extrophi/codio-cdn/actions"
echo ""
echo "Next: Spawn 4 CCW agents"
echo ""
gh issue list --label "phase:1"
```

---

## AFTER SETUP COMPLETE

Spawn CCW agents with these prompts (one tab per agent):

### CCW Agent ALPHA
```
I am AGENT ALPHA for Codio CDN Phase 1.

Repository: https://github.com/extrophi/codio-cdn
Branch: alpha
Issue: https://github.com/extrophi/codio-cdn/issues/1

Read: .github/prompts/alpha-content-id.md
Reference: /docs/pm/TECHNICAL-PROPOSAL-PHASE-1.md

Task: Implement content addressing library (sha256 CID)
Duration: 2 hours

Clone repo, checkout alpha branch, implement per proposal, test, commit, push.

BEGIN now.
```

### CCW Agent BETA
```
I am AGENT BETA for Codio CDN Phase 1.

Repository: https://github.com/extrophi/codio-cdn
Branch: beta
Issue: https://github.com/extrophi/codio-cdn/issues/2

Read: .github/prompts/beta-dht.md
Reference: /docs/pm/TECHNICAL-PROPOSAL-PHASE-1.md

Task: Implement Kademlia DHT using libp2p
Duration: 3 hours

Clone repo, checkout beta branch, implement per proposal, test, commit, push.

BEGIN now.
```

### CCW Agent GAMMA
```
I am AGENT GAMMA for Codio CDN Phase 1.

Repository: https://github.com/extrophi/codio-cdn
Branch: gamma
Issue: https://github.com/extrophi/codio-cdn/issues/3

Read: .github/prompts/gamma-cli.md
Reference: /docs/pm/TECHNICAL-PROPOSAL-PHASE-1.md

Task: Build CLI tool (codio-cdn command)
Duration: 2 hours

Clone repo, checkout gamma branch, implement per proposal, test, commit, push.

BEGIN now.
```

### CCW Agent DELTA
```
I am AGENT DELTA for Codio CDN Phase 1.

Repository: https://github.com/extrophi/codio-cdn
Branch: delta
Issue: https://github.com/extrophi/codio-cdn/issues/4

Read: .github/prompts/delta-tests.md
Reference: /docs/pm/TECHNICAL-PROPOSAL-PHASE-1.md

Task: Create integration tests and documentation
Duration: 1.5 hours

Clone repo, checkout delta branch, implement per proposal, test, commit, push.

BEGIN now.
```
