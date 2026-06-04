# Implementation Notes

This document describes what has been implemented in the current
`lightning-payjoin-kit` proof of concept, what is working, how the funding flow
is exercised, and how to test it locally.

The PoC goal is narrow and concrete: prove that a Lightning channel funding
transaction can be collaboratively constructed in a Payjoin-style flow, signed
by both on-chain input owners, handed through the Lightning funding safety
boundary, broadcast on Bitcoin Core regtest, and accepted by real LDK channel
state machines.

This is not a production node implementation yet. It is a working protocol and
integration proof.

---

## What Is Implemented

### Core Funding Coordinator

The main orchestration surface is `FundingCoordinator` in
`src/funding/orchestrator.rs`.

It owns:

- a wallet implementing `Wallet`
- a directory transport implementing `DirectoryClient`
- a `FundingPolicy`
- a funding state machine represented by `FundingState`

Implemented coordinator operations:

- Build the initiator's original fallback PSBT.
- Post the original PSBT into a directory session.
- Let the counterparty fetch the original PSBT and add a privacy input.
- Validate the counterparty proposal from the initiator side.
- Finalize the validated proposal by signing owned inputs.
- Produce a `FundingResult` containing the final transaction and funding
  outpoint.
- Broadcast the final transaction through any `Broadcaster`.
- Fall back to a single-party funding result when privacy-input coordination is
  not used.

The relevant public types are re-exported from `src/lib.rs`:

- `FundingCoordinator`
- `FundingMode`
- `FundingPolicy`
- `FundingRequest`
- `FundingResult`
- `FundingState`

### Funding Modes

The PoC supports the funding mode abstraction used by the rest of the crate.
The important implemented mode is `FundingMode::PrivacyInput`.

In privacy-input mode:

1. The initiator creates a normal-looking channel funding PSBT.
2. The counterparty adds an extra input.
3. The counterparty receives a change output.
4. The channel funding output remains unchanged.
5. The initiator validates that the proposal did not mutate the original
   funding terms.
6. Both parties sign their owned inputs.
7. The final transaction becomes the channel funding transaction.

This mode improves against the simple single-funder heuristic: the transaction
has inputs from more than one party, while the channel output remains a normal
Lightning funding output.

### PSBT Construction

The PSBT implementation lives under `src/psbt`.

Implemented pieces:

- `FundingPsbtBuilder`
- `FallbackFunding`
- `PrivacyInputProposal`
- `FinalizedFunding`

The builder currently handles:

- v2 Bitcoin transactions
- P2WPKH input fee estimation for the PoC
- channel funding output creation
- initiator change output creation when above dust
- counterparty privacy input insertion
- counterparty change output insertion
- `witness_utxo` population for signing and validation
- funding outpoint extraction after final transaction extraction

The channel funding output is intentionally kept at output index `0` in the PoC.
The validators enforce that this output is not changed by the counterparty
proposal.

### Payjoin Proposal Validation

The validation logic lives in `src/payjoin/validator.rs`.

Implemented validators:

- `CounterpartyOriginalValidator`
- `InitiatorProposalValidator`

The counterparty-side validator checks:

- the original PSBT has the expected channel funding value
- the counterparty input is not already present in the original transaction
- the required fee contribution is within policy
- the counterparty change output would not be dust
- the original PSBT has valid input/output accounting

The initiator-side validator checks:

- transaction version was not changed
- locktime was not changed
- original inputs were not modified
- the channel funding output was not changed
- at least one counterparty input was added
- at least one counterparty output was added
- absolute transaction fee was not reduced
- added fee does not exceed policy

This is enough for PoC safety. Production privacy policy will need stronger
checks around output ordering, amount fingerprinting, input selection strategy,
fee contribution negotiation, and denial-of-service boundaries.

### Directory Coordination

The directory abstraction lives under `src/directory`.

Implemented pieces:

- `DirectoryClient` trait
- `MockDirectory`
- session IDs, mailboxes, and payload storage by payload kind

The current PoC directory is in-memory. It proves the async relay shape without
running a network service.

The implemented directory flow is:

1. Initiator creates a session.
2. Initiator posts the original PSBT as `PayjoinPayloadKind::Original`.
3. Counterparty fetches the original payload.
4. Counterparty posts a proposal as `PayjoinPayloadKind::Proposal`.
5. Initiator fetches the proposal and finalizes it.

The mock directory is shared through cloneable in-memory state, so tests can
model both sides without a server.

### Wallet Support

The wallet abstraction lives under `src/wallet`.

Implemented pieces:

- `Wallet` trait
- `Utxo`
- `SigningSummary`
- `MemoryWallet`

`MemoryWallet` supports:

- static UTXO listing
- deterministic P2WPKH wallet creation for tests
- change script allocation
- signing owned P2WPKH inputs
- final witness population for signed inputs

This is enough to produce real signed P2WPKH spends in regtest. It is not a
production wallet. It has no persistence, descriptor support, coin selection
policy, gap limit handling, hardware signer flow, or external wallet RPC
integration.

### Lightning Funding Primitives

The Lightning-facing primitives live under `src/lightning`.

Implemented pieces:

- `FundingScript`
- `p2wsh_2of2_funding_script`
- `ChannelFundingHandoff`
- `CommitmentSafety`
- `PayjoinChannelFunder`
- `SimulatedChannelFunder`
- `SimulatedChannel`

`FundingScript::new_2of2` builds the 2-of-2 P2WSH funding script used by the
PoC. Public keys are sorted before script construction, giving deterministic
funding scripts.

`ChannelFundingHandoff` is the internal boundary between transaction
construction and Lightning channel acceptance. It includes:

- final funding transaction
- funding outpoint
- funding script pubkey
- channel balance assignment
- funding mode
- commitment safety state

`CommitmentSafety::CommitmentsExchanged` is required before the simulated
channel funder accepts a funding transaction. This models the important safety
rule: do not broadcast a channel funding transaction until the Lightning
commitment exchange has reached the broadcast-safe point.

### LDK Adapter

The optional LDK adapter lives in `src/lightning/ldk.rs` and is compiled with
the `ldk` feature.

Implemented pieces:

- `LdkFundingGeneration`
- `LdkFundingReference`
- `LdkManualFunding`
- `LdkManualFundingCallback`
- `LdkBroadcastSafe`
- `LdkFundingSession`
- `LdkFundingAdapter`
- `commitment_safe_handoff`
- `ldk_outpoint`

The adapter maps real LDK events into the PoC funding flow:

1. `Event::FundingGenerationReady` is converted into `LdkFundingGeneration`.
2. `LdkFundingGeneration` exposes a normal `FundingRequest`.
3. The collaborative funding coordinator builds a final `FundingResult`.
4. The result is converted into an `LdkFundingReference`.
5. `LdkManualFunding` calls LDK's manual funding callback:
   `unsafe_manual_funding_transaction_generated`.
6. LDK later emits `Event::FundingTxBroadcastSafe`.
7. `LdkFundingSession` observes that event and produces a commitment-safe
   `ChannelFundingHandoff`.

The callback trait is implemented for closures and for LDK's real
`ChannelManager` type under the `ldk` feature.

### Bitcoin Core Regtest Adapter

The optional Bitcoin Core adapter lives in `src/chain/corepc.rs` and is compiled
with the `corepc` feature.

Implemented pieces:

- `CorepcAuth`
- `CorepcRegtestClient`
- `Broadcaster` implementation for `CorepcRegtestClient`

The adapter uses `corepc-client`, not `bitcoincore-rpc`.

Implemented RPC operations:

- create wallet
- load wallet
- create new address
- generate blocks to address
- list unspent
- send to address
- fetch decoded wallet transaction
- broadcast raw transaction
- get block count

The live regtest test uses Docker Bitcoin Core and this adapter to fund two
test wallets, build a real signed collaborative funding transaction, broadcast
it, and mine it into a block.

---

## End-To-End PoC Flow

### Flow 1: Collaborative Funding Construction

This is the base flow used by unit tests, the Bitcoin Core regtest test, and
the LDK harness.

1. Build a Lightning funding script.
2. Create a `FundingRequest` with:
   - channel value
   - funding script pubkey
   - funding mode
   - fee rate
   - coordination deadline
3. Initiator wallet selects enough confirmed UTXOs.
4. Initiator builds an original fallback PSBT.
5. Counterparty validates the original PSBT.
6. Counterparty selects a privacy input.
7. Counterparty adds its input and change output.
8. Counterparty signs its owned input.
9. Initiator validates the proposal.
10. Initiator signs its owned input.
11. The finalized transaction is extracted.
12. The funding outpoint is computed from the transaction txid and funding
    output index.

The result is a valid Bitcoin transaction with:

- at least one initiator input
- at least one counterparty input
- a Lightning channel funding output
- change output handling
- final witnesses for owned P2WPKH inputs

### Flow 2: Directory-Mediated Coordination

The directory flow models the async relay architecture.

1. Initiator creates a directory session.
2. Initiator posts the original PSBT payload.
3. Counterparty fetches the original payload by session ID.
4. Counterparty builds and signs a privacy-input proposal.
5. Counterparty posts the proposal payload.
6. Initiator fetches the proposal payload.
7. Initiator validates and finalizes the proposal.

The current implementation uses `MockDirectory`. A production implementation
would replace this with an authenticated, encrypted, rate-limited relay.

### Flow 3: Bitcoin Core Regtest Broadcast

The Bitcoin Core regtest flow proves that the transaction is accepted by a real
Bitcoin Core node.

1. Docker starts a `bitcoind` regtest service.
2. The test creates a temporary Bitcoin Core wallet.
3. The wallet mines mature regtest funds.
4. The wallet pays two deterministic P2WPKH addresses:
   - initiator address
   - counterparty address
5. A block is mined to confirm those inputs.
6. `MemoryWallet` instances are created using the funded outpoints and matching
   private keys.
7. The collaborative funding flow builds and signs the transaction.
8. The transaction is handed through the simulated Lightning funding boundary.
9. The transaction is broadcast through `CorepcRegtestClient`.
10. A block is mined to confirm the collaborative funding transaction.

This proves the PoC transaction is not just structurally valid. It is accepted
by Bitcoin Core regtest and can be mined.

### Flow 4: Real Two-Node LDK Manual Funding Harness

The LDK harness proves the funding transaction can be accepted by real LDK
channel state machines.

The test lives in `tests/ldk_two_node_harness.rs` and is compiled with
`ldk-test-utils`.

The flow is:

1. Create two real LDK `ChannelManager` instances using LDK test utilities.
2. Initiator calls `create_channel`.
3. Initiator sends `open_channel`.
4. Counterparty handles `open_channel`.
5. Counterparty sends `accept_channel`.
6. Initiator handles `accept_channel`.
7. Initiator emits `FundingGenerationReady`.
8. The PoC converts that event into `LdkFundingGeneration`.
9. The collaborative funding flow builds a signed funding transaction for the
   LDK-provided funding script and channel value.
10. The transaction is converted into `LdkFundingReference`.
11. `LdkFundingSession` builds the manual funding payload.
12. The session applies manual funding to the real initiator `ChannelManager`.
13. Initiator sends `funding_created`.
14. Counterparty handles `funding_created`.
15. Counterparty sends `funding_signed`.
16. Initiator handles `funding_signed`.
17. Initiator emits `FundingTxBroadcastSafe`.
18. `LdkFundingSession` observes the broadcast-safe event.
19. The harness confirms the collaborative funding transaction.
20. LDK completes the channel-ready flow.
21. Both nodes report one usable channel.

This is the strongest PoC validation currently in the repository. It proves
that the architecture can cross the actual LDK manual-funding boundary and
reach a usable channel state.

---

## Feature Flags

The implemented feature flags are:

| Feature | Purpose |
|---------|---------|
| `std` | Default standard-library support. |
| `mock-directory` | Reserved feature for mock directory work. |
| `regtest` | Reserved feature for regtest-oriented work. |
| `corepc` | Enables the Bitcoin Core regtest RPC adapter using `corepc-client`. |
| `ldk` | Enables LDK adapter types and `ChannelManager` callback integration. |
| `ldk-test-utils` | Enables the ignored two-node LDK PoC harness through LDK `_test_utils`. |

`ldk-test-utils` is for tests only. It should not be treated as a production
runtime dependency.

---

## How To Test

### Standard Unit And Integration Tests

Run the default test suite:

```bash
cargo test
```

Run all feature combinations that are currently compiled by the crate:

```bash
cargo test --all-features
```

Run clippy with warnings denied:

```bash
cargo clippy --all-targets --all-features --keep-going -- --deny warnings
```

### Compile Optional Adapters

Compile the Bitcoin Core adapter:

```bash
cargo check --features corepc
```

Compile the LDK adapter:

```bash
cargo check --features ldk
```

Compile the LDK test harness:

```bash
cargo check --features ldk-test-utils --test ldk_two_node_harness
```

### Run The Bitcoin Core Regtest PoC

Start Bitcoin Core regtest:

```bash
docker compose up -d bitcoind
```

Run the ignored live regtest test:

```bash
cargo test --features corepc --test corepc_regtest -- --ignored
```

Run the same test while also compiling the LDK adapter:

```bash
cargo test --features corepc,ldk --test corepc_regtest -- --ignored
```

Defaults:

- RPC URL: `http://127.0.0.1:18443`
- RPC user: `lpk`
- RPC password: `lpk`

Overrides:

```bash
LPK_COREPC_URL=http://127.0.0.1:18443 \
LPK_COREPC_USER=lpk \
LPK_COREPC_PASSWORD=lpk \
cargo test --features corepc --test corepc_regtest -- --ignored
```

### Run The Two-Node LDK PoC

Run the ignored LDK harness:

```bash
cargo test --features ldk-test-utils --test ldk_two_node_harness -- --ignored
```

This does not require Docker. It uses LDK's in-process test utilities and real
LDK `ChannelManager` instances.

### Full Local PoC Verification

For a complete local verification pass:

```bash
docker compose up -d bitcoind
cargo test --features corepc --test corepc_regtest -- --ignored
cargo test --features ldk-test-utils --test ldk_two_node_harness -- --ignored
cargo test --all-features
cargo clippy --all-targets --all-features --keep-going -- --deny warnings
```

Expected result:

- the Bitcoin Core regtest test passes
- the LDK two-node harness passes
- all non-ignored tests pass
- clippy reports no warnings

---

## Test Coverage Map

The current tests cover these areas:

| Test | What It Proves |
|------|----------------|
| `tests/psbt_roundtrip.rs` | PSBT creation, proposal, signing, and final extraction. |
| `tests/privacy_input_flow.rs` | Privacy-input coordinator flow. |
| `tests/privacy_input_mode.rs` | Funding mode behavior. |
| `tests/proposal_validator.rs` | Initiator-side proposal validation. |
| `tests/counterparty_validator.rs` | Counterparty-side original PSBT validation. |
| `tests/directory_flow.rs` | Directory-mediated original/proposal exchange. |
| `tests/simulated_channel.rs` | Simulated channel handoff behavior. |
| `tests/simulated_lightning_funding.rs` | Funding handoff into simulated Lightning channel flow. |
| `tests/lightning_handoff.rs` | Commitment-safety boundary checks. |
| `tests/fallback.rs` | Fallback funding path. |
| `tests/broadcast.rs` | Broadcaster abstraction and broadcast state transition. |
| `tests/ldk_adapter.rs` | LDK adapter/session mapping without full node harness. |
| `tests/corepc_regtest.rs` | Real Bitcoin Core regtest broadcast and mining. |
| `tests/ldk_two_node_harness.rs` | Real two-node LDK manual funding lifecycle to usable channel. |

---

## What Is Working

The current PoC can:

- construct a Lightning channel funding output
- build an initiator fallback PSBT
- add a counterparty privacy input
- validate proposal invariants
- sign deterministic P2WPKH inputs
- extract a final Bitcoin transaction
- compute the funding outpoint
- enforce a commitment-safe handoff boundary
- broadcast through Bitcoin Core regtest using `corepc-client`
- mine the funding transaction on regtest
- convert LDK funding generation events into funding requests
- call the real LDK manual funding callback
- observe LDK's broadcast-safe event
- confirm the collaborative funding transaction in an LDK harness
- reach a usable channel between two real LDK `ChannelManager`s

This is enough to qualify as a PoC for the proposed architecture.

---

## What Is Not Production-Ready Yet

The current implementation intentionally leaves several production concerns out
of scope:

- no production relay server
- no encrypted relay payloads
- no authentication, replay protection, or rate limiting for relay traffic
- no production wallet backend
- no descriptor wallet support
- no hardware signer support
- no production-grade coin selection
- no advanced Payjoin output substitution policy
- no robust fee negotiation protocol
- no persistent channel funding session storage
- no async networking runtime integration
- no CLI implementation
- no CLN or LND plugin/FFI integration
- no mainnet/testnet deployment configuration
- no external security audit

These are productionization tasks. They do not block the current PoC claim.

---

## PoC Completion Criteria

The PoC should be considered complete when the following commands pass:

```bash
cargo test --all-features
cargo clippy --all-targets --all-features --keep-going -- --deny warnings
cargo test --features ldk-test-utils --test ldk_two_node_harness -- --ignored
```

For the live chain boundary, also run:

```bash
docker compose up -d bitcoind
cargo test --features corepc --test corepc_regtest -- --ignored
```

When these pass, the repository demonstrates:

- collaborative funding transaction construction
- real Bitcoin Core regtest acceptance
- real LDK manual-funding acceptance
- channel usability after confirmation

That is the current proof-of-concept target.
