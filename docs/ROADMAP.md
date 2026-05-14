# Roadmap — lightning-payjoin-kit

This document outlines the delivery timeline, key milestones, and KPIs for `lightning-payjoin-kit`. Our goal is to provide a production-ready, LDK-compatible Payjoin coordination engine for Lightning channel funding.

---

## Phase 1: Core Protocol & Async Coordination
**Target:** Weeks 1-4
**Status:** In Progress

The first phase focuses on building the standalone coordination engine capable of handling the offline-receiver problem inherent in Lightning channel openings.

### Objectives
- [x] Define Relay API specifications and payload encryption.
- [x] Implement the `PayjoinCoordinator` state machine.
- [x] Build PSBT round-trip handling (initial proposal -> counterparty signature -> final assembly).
- [ ] Implement UTXO selection algorithm optimized for multi-party privacy.
- [ ] Create a lightweight mock relay server for local integration testing.

### Deliverables
- A stable, zero-dependency `lightning-payjoin-kit` core crate.
- Complete unit test coverage for the coordination state machine.

---

## Phase 2: LDK Integration & Channel Funding
**Target:** Weeks 5-8
**Status:** Planned

Phase 2 bridges the standalone coordination engine with the Lightning Development Kit (LDK), enabling seamless Payjoin channel openings.

### Objectives
- [ ] Implement `PayjoinChannelFunder` trait matching LDK's channel creation flow.
- [ ] Handle fallback scenarios (e.g., graceful degradation to standard single-funder if the counterparty rejects Payjoin).
- [ ] Integrate fee estimation and network-specific validation.
- [ ] End-to-end testing with LDK nodes on Regtest.

### Deliverables
- `ldk` feature flag enabling LDK-specific adapters.
- E2E Regtest demonstration of a privately funded channel.

---

## Phase 3: Developer Tooling & Ecosystem Release
**Target:** Weeks 9-12
**Status:** Planned

The final phase ensures the library is accessible, easily distributable, and heavily tested for real-world usage.

### Objectives
- [ ] Build a robust CLI tool (`lightning-payjoin-kit open-channel`) for easy testing and usage by node operators.
- [ ] Complete security and privacy audits of the UTXO selection and PSBT handling.
- [ ] Draft comprehensive API documentation on `docs.rs`.
- [ ] Publish v1.0 release to crates.io.

### Deliverables
- CLI executable.
- Stable crates.io release.
- Setup guides for CLN and LND via FFI or plugin interfaces (stretch goal).

---

## KPIs & Success Metrics

To measure the success and adoption of this project, we are tracking the following metrics:
1. **Performance:** Sub-90s full coordination time for an online counterparty.
2. **Reliability:** 100% fallback success rate if Payjoin coordination fails (never block a channel opening).
3. **Privacy:** UTXO selection must provably defeat standard single-funder heuristics in all successful Payjoin openings.
4. **Adoption:** Successful integration into at least one major node implementation or wallet client.
