## Agent: ZETA - WASM Browser Bindings

**Duration:** 2.5 hours
**Branch:** `zeta`
**Dependencies:** EPSILON (uses transfer layer)

### Task
Create WASM bindings for browser integration.

### Technical Reference
- `/docs/pm/TECHNICAL-PROPOSAL-PHASE-2.md` (Agent ZETA section)

### Deliverables
- `crates/wasm/src/lib.rs` (200-300 lines)
- WASM-pack configuration
- JavaScript bindings
- Browser API wrappers
- Content download functions

### Success Criteria
- ✅ WASM compiles successfully
- ✅ JavaScript can call `CodioCDN.download(cid)`
- ✅ Returns content as Uint8Array
- ✅ Works in Chrome, Firefox, Safari

Update issue when complete.
