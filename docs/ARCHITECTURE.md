# Architecture — lightning-payjoin-kit

This document defines a refined architecture for building `lightning-payjoin-kit` to a credible Proof of Concept. The design is intentionally conservative: reuse existing Payjoin and Lightning protocol surfaces where possible, avoid custom cryptography, and prove the idea on regtest before claiming production readiness.

---

## Executive Summary

`lightning-payjoin-kit` explores collaborative Lightning channel funding transactions that weaken the single-funder on-chain heuristic.

The original design framed the project as "BIP-78 Payjoin over an async relay." The refined design is:

1. **Use BIP-77-style async Payjoin transport** for encrypted store-and-forward coordination.
2. **Use BOLT v2 interactive transaction construction** as the natural Lightning funding surface.
3. **Support a PoC mode where the counterparty contributes an input and receives equivalent change**, so the funding transaction has inputs from both peers while the counterparty does not intentionally add channel liquidity.
4. **Fall back to normal single-funder channel opening** when collaborative funding cannot complete.

This is not a claim that channel funding becomes anonymous. It is a claim that successful collaborative opens make common-input ownership and single-funder attribution less reliable.

---

## Design Goals

### Primary Goals

- Construct a valid Lightning channel funding transaction with inputs from both channel peers.
- Preserve correct Lightning channel accounting and commitment transaction safety.
- Demonstrate the flow end-to-end on regtest.
- Reuse BIP-77, PSBT, BOLT 2, and rust-bitcoin primitives instead of inventing a new wire protocol.
- Provide a small Rust API that can later be adapted to LDK, CLN, or LND integration.

### Non-Goals For PoC

- Mainnet production use.
- Full wallet-grade coin selection privacy.
- New Lightning specification changes.
- Hiding public channel capacity.
- Hiding peer network metadata without OHTTP/Tor deployment.
- Supporting every channel type, anchor variant, splice flow, and RBF policy in the first version.

---

## Protocol Foundations

### Payjoin Layer

The transport model should follow **BIP-77 Async Payjoin**, not a custom "encrypted BIP-78 relay."

BIP-77 provides:

- Store-and-forward directory semantics.
- End-to-end encrypted Payjoin messages.
- Sender and receiver mailboxes.
- Optional metadata protection through OHTTP.
- A fallback transaction model where either party can abandon the Payjoin proposal.

For the PoC, the implementation may begin with a local mock directory and direct HTTP polling. The data model should still match BIP-77 concepts so the transport can later be swapped for a compliant directory/OHTTP implementation.

### Lightning Layer

The Lightning-native surface is **BOLT v2 channel establishment with interactive transaction construction**.

Relevant concepts:

- `open_channel2`
- `accept_channel2`
- `tx_add_input`
- `tx_add_output`
- `tx_complete`
- `commitment_signed`
- `tx_signatures`

The PoC can initially model these messages locally instead of implementing a full peer stack. A stronger PoC should drive a real regtest node implementation that supports interactive funding.

---

## Funding Model

There are two distinct modes. The architecture must keep them separate.

### Mode A: True Dual Funding

Both peers contribute liquidity to the channel.

Example:

```text
Inputs:
  A: 700_000 sats
  B: 300_000 sats

Outputs:
  Channel funding output: 1_000_000 sats
  A change
  B change
```

This uses BOLT v2 dual funding directly. It improves the on-chain single-funder pattern, but the counterparty must actually lock capital into the channel.

### Mode B: Privacy-Input Funding

The initiator funds the channel. The counterparty contributes an input and receives equivalent change, minus any agreed fee contribution.

Example:

```text
Inputs:
  A: 1_050_000 sats
  B:   200_000 sats

Outputs:
  Channel funding output: 1_000_000 sats
  A change: about 45_000 sats
  B change: about 199_000 sats
  Miner fee: about 6_000 sats
```

The counterparty input exists to break the simple single-funder input pattern. It does not mean the counterparty receives channel balance.

This is the core research mode for `lightning-payjoin-kit`.

Important constraints:

- The channel funding output value must equal the Lightning-negotiated channel capacity.
- The counterparty's Lightning balance must not increase unless this is true dual funding.
- The counterparty must validate that its input is returned as change according to the agreed policy.
- The initiator must validate that its channel funding output and change are not reduced beyond policy limits.

---

## System Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                    lightning-payjoin-kit                        │
│                                                                 │
│  ┌──────────────────────┐        ┌──────────────────────────┐   │
│  │ Funding Orchestrator │        │ Payjoin Session Engine   │   │
│  │                      │◀──────▶│ BIP-77-style state       │   │
│  └──────────┬───────────┘        └───────────┬──────────────┘   │
│             │                                │                  │
│             ▼                                ▼                  │
│  ┌──────────────────────┐        ┌──────────────────────────┐   │
│  │ PSBT Funding Builder │        │ Directory Client         │   │
│  │ rust-bitcoin PSBT    │        │ mock first, BIP-77 later │   │
│  └──────────┬───────────┘        └───────────┬──────────────┘   │
│             │                                │                  │
│             ▼                                ▼                  │
│  ┌──────────────────────┐        ┌──────────────────────────┐   │
│  │ Policy Validator     │        │ Wallet Adapter           │   │
│  │ sender/receiver      │        │ inputs, change, signing  │   │
│  └──────────┬───────────┘        └───────────┬──────────────┘   │
│             │                                │                  │
│             └───────────────┬────────────────┘                  │
│                             ▼                                   │
│                  Broadcast-ready funding tx                     │
└─────────────────────────────────────────────────────────────────┘
```

---

## Core Components

### Funding Orchestrator

Coordinates the complete channel funding attempt.

Responsibilities:

- Start a collaborative funding session.
- Choose funding mode: true dual funding or privacy-input funding.
- Track deadlines and fallback paths.
- Expose a deterministic state machine for tests.
- Hand completed transactions to the Lightning integration layer.

Suggested states:

```rust
pub enum FundingState {
    Idle,
    OriginalPrepared,
    ProposalRequested,
    ProposalReceived,
    ProposalValidated,
    FinalSigned,
    BroadcastReady,
    FallbackReady,
    Failed,
}
```

### Payjoin Session Engine

Handles BIP-77-style message exchange.

Responsibilities:

- Create session identifiers and mailbox references.
- Encrypt and decrypt session payloads.
- Poll or post through a directory client.
- Keep transport concerns separate from transaction validation.

For PoC, this can use a mock directory:

```text
POST /sessions
POST /sessions/{id}/original
GET  /sessions/{id}/original
POST /sessions/{id}/proposal
GET  /sessions/{id}/proposal
```

For production, this should converge toward BIP-77-compatible directory/OHTTP behavior rather than preserving the mock API.

### PSBT Funding Builder

Builds and modifies the Bitcoin transaction.

Responsibilities:

- Build an original PSBT with a valid fallback funding transaction.
- Add counterparty inputs and change outputs.
- Preserve the channel funding output.
- Compute fees from complete UTXO data.
- Support SegWit v0 and Taproot inputs where rust-bitcoin support is sufficient.

The first PoC should support a narrow input matrix:

- P2WPKH funding inputs.
- Confirmed regtest UTXOs.
- One initiator input and one counterparty input.
- One channel funding output.
- One change output per party.

### Policy Validator

Enforces Payjoin and Lightning safety rules before signing.

Initiator checks:

- The channel funding output is present.
- The channel funding output script and amount are unchanged.
- All original initiator inputs remain present.
- Initiator change is not reduced beyond fee policy.
- Transaction version, locktime, and sequences remain policy-valid.
- Counterparty inputs include complete UTXO data.
- The fee rate is within the agreed range.

Counterparty checks:

- The original/fallback transaction is valid enough for the agreed threat model.
- Counterparty input is not exposed repeatedly across failed sessions without policy.
- Counterparty change output pays the expected script.
- Counterparty value loss is bounded by the agreed fee contribution.
- No unexpected output steals counterparty value.

Lightning checks:

- Funding output value equals negotiated channel capacity.
- Funding script matches the 2-of-2 channel funding script.
- Commitment transaction signatures commit to the final funding outpoint.
- The transaction is not broadcast before Lightning signatures are safely exchanged.

### Wallet Adapter

Abstracts wallet-specific operations.

Responsibilities:

- List spendable UTXOs.
- Reserve and release UTXOs.
- Derive change scripts.
- Provide complete UTXO data for PSBT inputs.
- Sign only owned inputs.
- Refuse to sign unknown or policy-invalid inputs.

PoC adapter:

- In-memory regtest wallet fixtures.
- Deterministic keys.
- Manual UTXO injection for tests.

Production adapters can target Bitcoin Core RPC, BDK, LDK wallet integrations, or node-specific plugin APIs.

---

## End-to-End PoC Flow

### Step 1: Initiator Prepares Fallback

Node A builds a standard single-funder channel funding PSBT:

```text
A input(s) -> channel funding output + A change
```

This transaction must be valid as a fallback.

### Step 2: Initiator Posts Original

Node A posts an encrypted original PSBT payload to the directory session.

For the local PoC, the mock directory can store plaintext or test-encrypted payloads. The Rust API should still model encryption boundaries.

### Step 3: Counterparty Builds Proposal

Node B retrieves the original PSBT, validates it, and adds:

- One B-owned input.
- One B-owned change output.
- Any fee contribution allowed by policy.

Node B signs only B-owned inputs.

### Step 4: Initiator Validates Proposal

Node A retrieves the proposal and validates:

- Channel funding output is unchanged.
- A-owned inputs are unchanged.
- A-owned outputs are policy-valid.
- B inputs are finalized or otherwise verifiably signed.
- Fee rate remains acceptable.

Node A signs only A-owned inputs.

### Step 5: Lightning Funding Finalization

The final transaction is connected to the Lightning channel establishment flow:

- Commitment signatures are exchanged.
- The funding outpoint is fixed.
- The transaction is broadcast only after the channel state is safe.

For a lower-grade PoC, this step may be simulated and asserted in tests. For a strong PoC, it must open a real regtest channel.

---

## Proof-of-Concept Levels

### Level 1: PSBT Demonstrator

Qualifies as a protocol-construction demo.

Requirements:

- Rust crate compiles.
- Builds original PSBT.
- Counterparty adds input and change.
- Both parties validate and sign.
- Final transaction broadcasts on regtest.
- Tests prove fallback transaction remains valid.

Limitations:

- Does not prove Lightning compatibility.
- Does not open a real channel.

### Level 2: Simulated Lightning Funding

Qualifies as a stronger protocol PoC.

Requirements:

- Everything in Level 1.
- Funding output script matches a Lightning 2-of-2 funding script.
- Commitment transaction model references the final funding outpoint.
- Tests assert channel balances are unchanged by privacy-input mode.

Limitations:

- Still does not prove integration with a production node implementation.

### Level 3: Regtest Channel Open

Qualifies as the target PoC for this project.

Requirements:

- Everything in Level 2.
- Two regtest Lightning nodes complete channel establishment.
- Funding transaction contains inputs from both peers.
- Counterparty receives equivalent change in privacy-input mode.
- Channel becomes usable after confirmation.
- Fallback single-funder open succeeds when collaboration fails.

This is the level that should be used for grant, research, or ecosystem validation claims.

---

## Security Model

### Threats Mitigated

| Threat | Mitigation |
|--------|------------|
| Single-funder attribution | Include inputs from both peers in the funding transaction |
| Common-input ownership heuristic | Use Payjoin-style collaborative input construction |
| Malicious proposal modifies funding output | Initiator validates funding output script and value before signing |
| Counterparty value theft | Counterparty validates change output and bounded fee loss |
| Relay/directory message tampering | Authenticated encryption and PSBT validation |
| Relay unavailability | Timeout and fallback transaction |
| Premature broadcast | Broadcast only after Lightning commitment safety is reached |

### Threats Not Solved

- Public channel capacity visibility.
- Channel graph and gossip correlation.
- IP address correlation unless OHTTP/Tor is used.
- Timing correlation between coordination and broadcast.
- Counterparty UTXO probing without receiver-side reuse and exposure policy.
- Wallet fingerprinting through script types, input counts, output ordering, and fee patterns.
- On-chain close transaction analysis.

---

## Privacy Requirements

A successful privacy-input funding transaction should:

- Use script types that do not trivially identify each party by wallet fingerprint.
- Avoid obvious round-number change outputs where possible.
- Randomize or policy-control input/output ordering.
- Preserve a fee rate that does not make the Payjoin transformation obvious.
- Avoid exposing fresh counterparty UTXOs to repeated failed attempts.
- Record enough local metadata to reuse exposed UTXOs after aborts where appropriate.

The PoC only needs to demonstrate the heuristic break. Production work needs a full coin selection and wallet fingerprinting review.

---

## Rust Crate Layout

Proposed initial layout:

```text
src/
  lib.rs
  error.rs
  funding/
    mod.rs
    orchestrator.rs
    state.rs
    mode.rs
  payjoin/
    mod.rs
    session.rs
    payload.rs
    validator.rs
  psbt/
    mod.rs
    builder.rs
    finalize.rs
  wallet/
    mod.rs
    traits.rs
    memory.rs
  directory/
    mod.rs
    client.rs
    mock.rs
  lightning/
    mod.rs
    funding_script.rs
    simulated.rs
tests/
  psbt_roundtrip.rs
  privacy_input_mode.rs
  fallback.rs
  simulated_channel.rs
```

Feature flags:

```toml
[features]
default = ["std"]
std = []
mock-directory = []
regtest = []
ldk = ["dep:lightning"]
```

Core dependencies should be explicit rather than marketed as "zero dependency":

- `bitcoin`
- `secp256k1`
- `thiserror`
- `serde` for transport payloads
- `tokio` or async traits for directory clients, if async transport is included
- Optional `lightning` for LDK experiments

---

## Public API Sketch

```rust
pub struct FundingCoordinator<W, D> {
    wallet: W,
    directory: D,
    policy: FundingPolicy,
}

pub enum FundingMode {
    TrueDualFunding,
    PrivacyInput,
}

pub struct FundingRequest {
    pub channel_value_sats: u64,
    pub funding_script: bitcoin::ScriptBuf,
    pub mode: FundingMode,
    pub fee_rate_sat_vb: f32,
    pub deadline: std::time::Duration,
}

pub struct FundingResult {
    pub transaction: bitcoin::Transaction,
    pub funding_outpoint: bitcoin::OutPoint,
    pub fallback_used: bool,
}
```

The API should avoid pretending it can open a channel by itself. It constructs and coordinates the funding transaction; the Lightning node integration is responsible for channel state transitions.

---

## Implementation Roadmap

### Phase 1: PSBT Core

- Create Rust crate.
- Implement wallet traits and in-memory regtest wallet.
- Build fallback funding PSBT.
- Implement proposal construction for privacy-input mode.
- Implement initiator and counterparty validators.
- Add unit tests for value conservation and bounded fee loss.

### Phase 2: Mock Async Coordination

- Implement mock directory client.
- Add state machine around original/proposal/final flow.
- Add timeout and fallback behavior.
- Add integration tests for online, delayed, rejected, and timeout sessions.

### Phase 3: Simulated Lightning Funding

- Generate 2-of-2 funding scripts.
- Model funding outpoint and commitment preconditions.
- Assert that privacy-input mode does not alter channel balances.
- Broadcast final transaction on regtest.

### Phase 4: Real Node PoC

- Integrate with one Lightning implementation on regtest.
- Prefer an implementation path that exposes interactive funding hooks.
- Demonstrate a usable channel after confirmation.
- Document unsupported channel types and policy constraints.

### Phase 5: BIP-77/OHTTP Compliance

- Replace mock directory with BIP-77-compatible payloads.
- Add OHTTP support or document Tor-only test transport.
- Add padding and metadata minimization.
- Review against BIP-77 sender/receiver checklists.

---

## Success Criteria

The project reaches a credible PoC when:

1. A regtest transaction funding a Lightning channel contains inputs from both peers.
2. The counterparty's privacy input is returned as change in privacy-input mode.
3. The channel funding output and channel accounting remain correct.
4. The transaction confirms.
5. The resulting channel becomes usable, or the simulation proves the exact funding outpoint and commitment preconditions.
6. Collaboration failure falls back to a valid single-funder path.
7. Tests cover malicious proposal attempts that modify funding output, steal change, alter fees, or request signatures for unknown inputs.

---

## Open Research Questions

- Can privacy-input mode be represented cleanly inside existing BOLT v2 interactive funding implementations, or does it require node-specific extension hooks?
- Which Lightning implementation exposes the least invasive path to a regtest PoC?
- How should receiver UTXO exposure be managed after aborts?
- What coin selection policy minimizes wallet fingerprinting while remaining practical?
- How should fee contribution be negotiated so neither party can grief the other?
- Does using a counterparty input with near-equal change create a recognizable fingerprint in itself?

These questions do not block a PoC, but they do block production claims.

---

## References

- [BIP-77: Async Payjoin](https://bips.dev/77/)
- [BIP-78: A Simple Payjoin Proposal](https://bips.dev/78/)
- [BOLT 2: Peer Protocol for Channel Management](https://github.com/lightning/bolts/blob/master/02-peer-protocol.md)
- [Payjoin Dev Kit](https://github.com/payjoin/rust-payjoin)
- [rust-bitcoin](https://github.com/rust-bitcoin/rust-bitcoin)
- [LDK](https://github.com/lightningdevkit/rust-lightning)
