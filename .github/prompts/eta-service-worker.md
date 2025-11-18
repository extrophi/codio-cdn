## Agent: ETA - Service Worker & Extension

**Duration:** 3 hours
**Branch:** `eta`
**Dependencies:** ZETA (uses WASM)

### Task
Build Service Worker for fetch interception and browser extension.

### Technical Reference
- `/docs/pm/TECHNICAL-PROPOSAL-PHASE-2.md` (Agent ETA section)

### Deliverables
- `browser/service-worker/sw.js`
- `browser/extension/` (Chrome/Firefox)
- Service Worker fetch interception
- Browser extension UI

### Success Criteria
- ✅ Service Worker registers successfully
- ✅ Intercepts `codio://` requests
- ✅ Falls back to HTTPS when P2P fails
- ✅ Extension shows real-time stats
- ✅ Works in Chrome and Firefox

Update issue when complete.
