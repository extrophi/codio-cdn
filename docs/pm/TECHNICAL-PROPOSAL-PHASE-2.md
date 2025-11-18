# TECHNICAL PROPOSAL: Codio CDN Phase 2
## Browser Integration - WebRTC Data Transfer

**Version:** 1.0
**Date:** Monday, November 18, 2025
**Phase:** 2 (Browser Integration)
**Duration:** 8-12 hours
**Dependencies:** Phase 1 Complete ✅

---

## EXECUTIVE SUMMARY

**Goal:** Enable actual P2P content transfer in browsers using WebRTC

**Current State (Phase 1):**
- ✅ CID generation works
- ✅ DHT announces content
- ✅ CLI finds providers
- ❌ No actual content transfer

**Phase 2 Adds:**
- ✅ WebRTC peer-to-peer data transfer
- ✅ Browser WASM integration
- ✅ Content chunking and streaming
- ✅ Multi-peer downloads
- ✅ HTTPS gateway fallback

**Outcome:** Working P2P CDN in browser (download content from peers)

---

## ARCHITECTURE OVERVIEW

### Phase 1 → Phase 2 Evolution

```
PHASE 1 (Current):
User → CLI → DHT → Find Providers → [NO TRANSFER]

PHASE 2 (Target):
User → Browser → DHT → Find Providers → WebRTC Transfer → Content
                                      ↓
                                   HTTPS Fallback
```

### New Components

```
codio-cdn/
├── crates/
│   ├── content-id/      (Phase 1 ✅)
│   ├── common/          (Phase 1 ✅)
│   ├── dht/             (Phase 1 ✅)
│   ├── cli/             (Phase 1 ✅)
│   │
│   ├── transfer/        (Phase 2 NEW)  ← WebRTC data transfer
│   ├── wasm/            (Phase 2 NEW)  ← Browser WASM bindings
│   └── gateway/         (Phase 2 NEW)  ← HTTPS fallback
│
├── browser/             (Phase 2 NEW)
│   ├── service-worker/  ← Fetch interception
│   ├── extension/       ← Chrome/Firefox extension
│   └── lib/             ← Browser JS library
│
└── examples/
    └── browser-demo/    ← Working demo page
```

---

## AGENT ASSIGNMENTS (4 Parallel Agents)

### AGENT EPSILON: WebRTC Transfer Layer
**Duration:** 3 hours
**Priority:** CRITICAL (core functionality)
**Branch:** `epsilon`

**Task:** Implement WebRTC peer-to-peer data transfer

**Deliverables:**
- `crates/transfer/src/lib.rs` (300-400 lines)
  - WebRTC peer connection management
  - Data channel creation
  - Chunked content transfer
  - Multi-peer support (parallel downloads)
  - Progress tracking

**Key Functions:**
```rust
pub struct TransferManager {
    connections: HashMap<PeerId, RTCPeerConnection>,
    active_transfers: HashMap<ContentId, Transfer>,
}

impl TransferManager {
    pub async fn download_from_peer(
        &mut self,
        peer: PeerId,
        cid: ContentId
    ) -> Result<Vec<u8>>;

    pub async fn download_from_multiple_peers(
        &mut self,
        peers: Vec<PeerId>,
        cid: ContentId
    ) -> Result<Vec<u8>>;

    pub fn serve_content(
        &mut self,
        peer: PeerId,
        cid: ContentId,
        content: &[u8]
    ) -> Result<()>;
}
```

**Success Criteria:**
- Can establish WebRTC connection between 2 peers
- Can transfer file via data channel
- Can download same file from multiple peers (chunked)
- Progress tracking works
- Handles peer disconnection gracefully

---

### AGENT ZETA: WASM Browser Bindings
**Duration:** 2.5 hours
**Priority:** HIGH
**Dependencies:** EPSILON (uses transfer layer)
**Branch:** `zeta`

**Task:** Create WASM bindings for browser integration

**Deliverables:**
- `crates/wasm/src/lib.rs` (200-300 lines)
  - WASM-pack configuration
  - JavaScript bindings
  - Browser API wrappers
  - Content download functions

**Implementation:**
```rust
use wasm_bindgen::prelude::*;
use web_sys::{RtcPeerConnection, RtcDataChannel};

#[wasm_bindgen]
pub struct CodioCDN {
    transfer_manager: TransferManager,
    dht: DhtNode,
}

#[wasm_bindgen]
impl CodioCDN {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<CodioCDN, JsValue>;

    #[wasm_bindgen]
    pub async fn download(
        &mut self,
        cid: String
    ) -> Result<Vec<u8>, JsValue>;

    #[wasm_bindgen]
    pub async fn upload(
        &mut self,
        content: Vec<u8>
    ) -> Result<String, JsValue>;

    #[wasm_bindgen]
    pub fn get_peers(&self) -> Result<JsValue, JsValue>;
}
```

**Build Configuration:**
```toml
[package]
name = "codio-wasm"
version = "0.1.0"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = [
    "RtcPeerConnection",
    "RtcDataChannel",
    "RtcConfiguration"
]}
js-sys = "0.3"
```

**Success Criteria:**
- WASM compiles successfully
- JavaScript can call `CodioCDN.download(cid)`
- Returns content as Uint8Array
- Works in Chrome, Firefox, Safari

---

### AGENT ETA: Service Worker & Extension
**Duration:** 3 hours
**Priority:** MEDIUM
**Dependencies:** ZETA (uses WASM)
**Branch:** `eta`

**Task:** Build Service Worker for fetch interception and browser extension

**Deliverables:**

**1. Service Worker** (`browser/service-worker/sw.js`):
```javascript
// Intercept fetch requests
self.addEventListener('fetch', async (event) => {
  const url = new URL(event.request.url);

  // Handle codio:// protocol
  if (url.protocol === 'codio:') {
    event.respondWith(handleCodioFetch(url));
    return;
  }

  // Handle regular HTTPS with X-Codio-CID header
  const cid = event.request.headers.get('X-Codio-CID');
  if (cid) {
    event.respondWith(handleCodioFetch(cid));
    return;
  }

  // Pass through
  event.respondWith(fetch(event.request));
});

async function handleCodioFetch(cidOrUrl) {
  const cid = extractCID(cidOrUrl);

  try {
    // Try P2P first
    const content = await codioCDN.download(cid);
    return new Response(content, {
      status: 200,
      headers: { 'Content-Type': 'application/octet-stream' }
    });
  } catch (e) {
    // Fallback to HTTPS gateway
    return fetch(`https://ipfs.io/ipfs/${cid}`);
  }
}
```

**2. Browser Extension** (`browser/extension/`):
- `manifest.json` (Chrome/Firefox)
- `popup.html` (UI showing peer stats)
- `background.js` (manages WASM instance)

**Extension UI:**
```html
<div class="codio-extension">
  <h3>Codio CDN Status</h3>
  <div class="stats">
    <div>Peers: <span id="peer-count">0</span></div>
    <div>Downloaded: <span id="bytes-downloaded">0 MB</span></div>
    <div>Uploaded: <span id="bytes-uploaded">0 MB</span></div>
  </div>
  <div class="active-transfers" id="transfers"></div>
</div>
```

**Success Criteria:**
- Service Worker registers successfully
- Intercepts `codio://` requests
- Falls back to HTTPS when P2P fails
- Extension shows real-time stats
- Works in Chrome and Firefox

---

### AGENT THETA: HTTPS Gateway & Testing
**Duration:** 2.5 hours
**Priority:** MEDIUM
**Dependencies:** EPSILON, ZETA, ETA
**Branch:** `theta`

**Task:** Implement HTTPS gateway fallback and comprehensive testing

**Deliverables:**

**1. Gateway Service** (`crates/gateway/src/lib.rs`):
```rust
pub struct GatewayConfig {
    pub primary: String,      // https://ipfs.io/ipfs/
    pub fallbacks: Vec<String>, // Alternative gateways
    pub timeout: Duration,
}

pub struct Gateway {
    config: GatewayConfig,
    client: reqwest::Client,
}

impl Gateway {
    pub async fn fetch_cid(
        &self,
        cid: &ContentId
    ) -> Result<Vec<u8>> {
        // Try primary gateway
        match self.try_gateway(&self.config.primary, cid).await {
            Ok(content) => return Ok(content),
            Err(e) => warn!("Primary gateway failed: {}", e),
        }

        // Try fallbacks
        for gateway in &self.config.fallbacks {
            match self.try_gateway(gateway, cid).await {
                Ok(content) => return Ok(content),
                Err(_) => continue,
            }
        }

        Err(anyhow!("All gateways failed"))
    }
}
```

**2. Integration Tests** (`tests/phase2_integration.rs`):
```rust
#[tokio::test]
async fn test_webrtc_peer_connection() {
    // Create 2 peers
    let peer1 = TransferManager::new().await.unwrap();
    let peer2 = TransferManager::new().await.unwrap();

    // Establish connection
    peer1.connect_to_peer(peer2.id()).await.unwrap();

    // Transfer content
    let content = b"Hello, P2P world!";
    let cid = ContentId::new(content);

    peer1.serve_content(peer2.id(), &cid, content).unwrap();
    let received = peer2.download_from_peer(peer1.id(), &cid).await.unwrap();

    assert_eq!(content, received.as_slice());
}

#[tokio::test]
async fn test_multi_peer_download() {
    // Create 3 peers with same content
    let peers = create_peer_network(3).await;
    let content = large_test_content(10 * 1024 * 1024); // 10MB
    let cid = ContentId::new(&content);

    // Each peer has the content
    for peer in &peers {
        peer.announce_content(&cid).await;
    }

    // Download from multiple peers simultaneously
    let downloader = TransferManager::new().await.unwrap();
    let peer_ids: Vec<_> = peers.iter().map(|p| p.id()).collect();

    let received = downloader
        .download_from_multiple_peers(peer_ids, &cid)
        .await
        .unwrap();

    assert_eq!(content, received);
}

#[tokio::test]
async fn test_https_gateway_fallback() {
    let gateway = Gateway::new(GatewayConfig::default());

    // Use known IPFS CID
    let cid = ContentId::from_str(
        "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"
    ).unwrap();

    let content = gateway.fetch_cid(&cid).await.unwrap();
    assert!(!content.is_empty());
}
```

**3. Browser E2E Tests** (`browser/test/e2e.test.js`):
```javascript
describe('Codio CDN Browser Integration', () => {
  it('downloads content via P2P', async () => {
    const cdn = await CodioCDN.new();
    const cid = 'QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG';

    const content = await cdn.download(cid);
    expect(content).toBeInstanceOf(Uint8Array);
    expect(content.length).toBeGreaterThan(0);
  });

  it('falls back to HTTPS when no peers', async () => {
    const cdn = await CodioCDN.new();
    // Disconnect from DHT to force fallback
    await cdn.disconnect();

    const cid = 'QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG';
    const content = await cdn.download(cid);

    expect(content.length).toBeGreaterThan(0);
  });
});
```

**Success Criteria:**
- Gateway can fetch from IPFS
- All integration tests pass
- Browser E2E tests pass
- Fallback works when P2P unavailable

---

## TECHNICAL DEPENDENCIES

### New Rust Crates
```toml
[workspace.dependencies]
# WebRTC (Phase 2)
webrtc = "0.9"
tokio-tungstenite = "0.21"  # WebSocket for signaling

# WASM (Phase 2)
wasm-bindgen = "0.2"
web-sys = "0.3"
js-sys = "0.3"

# Gateway (Phase 2)
reqwest = { version = "0.11", features = ["json"] }
```

### Browser Dependencies
```json
{
  "devDependencies": {
    "wasm-pack": "^0.12.1",
    "webpack": "^5.89.0",
    "jest": "^29.7.0",
    "puppeteer": "^21.6.0"
  }
}
```

---

## DATA FLOW (Phase 2)

### Download Flow
```
Browser User Requests: codio://QmYwAPJ...
    ↓
Service Worker Intercepts
    ↓
WASM CodioCDN.download(cid)
    ↓
DHT: Find Providers → [Peer1, Peer2, Peer3]
    ↓
WebRTC: Connect to Peers
    ↓
Transfer: Download chunks in parallel
    ├─ Peer1: bytes 0-1MB
    ├─ Peer2: bytes 1-2MB
    └─ Peer3: bytes 2-3MB
    ↓
Reassemble & Verify (SHA256 match)
    ↓
Return to Browser
    ↓
User sees content

IF P2P FAILS:
    ↓
Gateway: HTTPS fetch from ipfs.io
    ↓
Return to Browser
```

---

## EXECUTION TIMELINE

```
Hour 0:00 - ROOT CCL: Create Phase 2 issues
Hour 0:15 - Spawn 4 agents (EPSILON, ZETA, ETA, THETA)

Hour 0:15 - EPSILON starts: WebRTC transfer layer
Hour 0:15 - ZETA starts: WASM bindings (waits for EPSILON types)
Hour 0:15 - ETA starts: Service Worker (waits for ZETA)
Hour 0:15 - THETA starts: Gateway & tests (can work independently)

Hour 2:30 - THETA completes: Gateway + initial tests
Hour 3:00 - EPSILON completes: WebRTC transfer working
Hour 3:30 - ZETA completes: WASM compiles, JS bindings work
Hour 5:00 - ETA completes: Service Worker + Extension

Hour 5:00 - ROOT CCL: Integration testing
Hour 6:00 - ROOT CCL: Browser demo working
Hour 7:00 - ROOT CCL: Documentation
Hour 8:00 - Phase 2 COMPLETE
```

**Critical Path:** EPSILON (3h) → ZETA (2.5h) → ETA (3h) = 8.5 hours

---

## SUCCESS CRITERIA

### Must Have (MVP)

✅ **Functional:**
- WebRTC connection between 2 browser tabs
- Can transfer 1MB file via P2P
- Service Worker intercepts requests
- HTTPS fallback works
- Browser extension shows stats

✅ **Quality:**
- Integration tests pass
- Browser E2E tests pass
- WASM compiles without errors
- Works in Chrome and Firefox

✅ **Performance:**
- P2P transfer: >1MB/s
- Multi-peer: faster than single peer
- Fallback: <2s to HTTPS

### Nice to Have (Stretch)

⭐ **Enhanced:**
- Safari support
- Resume interrupted downloads
- Caching layer
- Bandwidth limiting

⭐ **UX:**
- Progress bars in extension
- Peer map visualization
- Upload speed display

---

## DEMO REQUIREMENTS

**At completion, we must demonstrate:**

```html
<!-- demo.html -->
<!DOCTYPE html>
<html>
<head>
  <title>Codio CDN Demo</title>
  <script type="module">
    import init, { CodioCDN } from './pkg/codio_wasm.js';

    async function demo() {
      await init();
      const cdn = await CodioCDN.new();

      // Download via P2P
      const cid = 'QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG';
      const content = await cdn.download(cid);

      console.log('Downloaded:', content.length, 'bytes via P2P!');
    }

    demo();
  </script>
</head>
<body>
  <h1>Codio CDN Browser Demo</h1>
  <div id="status">Loading...</div>
</body>
</html>
```

**Demo proves:**
1. WASM loads in browser
2. Can download content by CID
3. P2P transfer works
4. Fallback to HTTPS works

---

## RISK ASSESSMENT

### Technical Risks

**Risk 1: WebRTC Complexity**
- **Probability:** HIGH
- **Impact:** HIGH (core functionality)
- **Mitigation:** Use proven webrtc-rs crate, start simple
- **Contingency:** Use HTTP transfer for Phase 2, WebRTC in Phase 3

**Risk 2: WASM Size**
- **Probability:** MEDIUM
- **Impact:** MEDIUM (slow page loads)
- **Mitigation:** wasm-opt optimization, lazy loading
- **Contingency:** Split into smaller WASM modules

**Risk 3: Browser Compatibility**
- **Probability:** MEDIUM
- **Impact:** MEDIUM (Safari issues)
- **Mitigation:** Test early, use polyfills
- **Contingency:** Chrome/Firefox only for Phase 2

### Resource Risks

**Risk 4: Timeline (8-12 hours)**
- **Probability:** MEDIUM
- **Impact:** HIGH
- **Mitigation:** Start with EPSILON (critical path)
- **Contingency:** Skip ETA (extension) if needed

**Risk 5: Agent Coordination**
- **Probability:** LOW (proven in Phase 1)
- **Impact:** LOW
- **Mitigation:** Clear dependencies, sequential where needed
- **Contingency:** ROOT CCL takes over

---

## BUDGET

**Estimated Agent Costs:**
```
EPSILON (WebRTC):      3h × $75/hr (Sonnet)  = $225
ZETA (WASM):           2.5h × $75/hr (Sonnet) = $188
ETA (Service Worker):  3h × $30/hr (Haiku)   = $90
THETA (Gateway/Tests): 2.5h × $30/hr (Haiku) = $75
ROOT CCL:              3h × $75/hr (Sonnet)   = $225
──────────────────────────────────────────────────
TOTAL:                                         = $803

RECOMMENDED BUDGET: $850 (6% buffer)
```

---

## APPROVAL CHECKLIST

**Stakeholder Sign-Off:**

- [ ] Technical approach approved (WebRTC + WASM)
- [ ] Budget approved ($850)
- [ ] Timeline acceptable (8-12 hours)
- [ ] Dependencies on Phase 1 acknowledged
- [ ] Success criteria clear

**Approved by:** ______________________
**Date:** ______________________
**Budget:** $ ______________________

---

## NEXT STEPS (UPON APPROVAL)

1. Create GitHub issues (#5-8)
2. Create agent prompt files
3. Spawn 4 agents (EPSILON, ZETA, ETA, THETA)
4. Monitor progress
5. Integration testing
6. Demo deployment

---

**Phase 2 Status:** ⏸️ AWAITING APPROVAL
**Recommendation:** GREEN LIGHT
**Confidence:** HIGH (Phase 1 proven pattern)

**"From foundation to browser - decentralization ships."** 🚀
