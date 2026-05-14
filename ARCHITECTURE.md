# Architecture — lightning-payjoin-kit

This document describes the full technical architecture, protocol design, coordination mechanism, and security model for `lightning-payjoin-kit`.

---

## Overview

`lightning-payjoin-kit` solves a specific problem: standard Lightning channel openings use single-funder UTXO inputs that make node funding traceable on-chain. The library brings BIP-78 Payjoin coordination to the channel establishment flow so that funding transactions appear as multi-party inputs.

The core challenge — and why this has not been built before — is that BIP-78 requires both parties to be online and coordinating in real time. Lightning channel openings happen asynchronously. This library solves the synchronization problem through a relay-based async coordination design.

---

## System Components

```
┌─────────────────────────────────────────────────────────────────┐
│                    lightning-payjoin-kit                        │
│                                                                 │
│  ┌─────────────────┐    ┌──────────────────┐                   │
│  │  Coordination   │    │   PSBT Builder   │                   │
│  │     Engine      │───▶│                  │                   │
│  │  (async relay)  │    │  BIP-78 adapter  │                   │
│  └────────┬────────┘    └────────┬─────────┘                   │
│           │                      │                             │
│  ┌────────▼────────┐    ┌────────▼─────────┐                   │
│  │  Relay Client   │    │  UTXO Selector   │                   │
│  │                 │    │                  │                   │
│  │  Async message  │    │  Multi-party     │                   │
│  │  coordination   │    │  input builder   │                   │
│  └────────┬────────┘    └────────┬─────────┘                   │
│           │                      │                             │
│  ┌────────▼──────────────────────▼─────────┐                   │
│  │            Channel Funding TX            │                   │
│  │                                         │                   │
│  │  Valid Payjoin PSBT ready for broadcast  │                   │
│  └─────────────────────────────────────────┘                   │
└─────────────────────────────────────────────────────────────────┘
         │                          │
         ▼                          ▼
  ┌─────────────┐          ┌──────────────────┐
  │  LDK Node   │          │  Bitcoin Network  │
  │ Integration │          │   (on-chain tx)   │
  └─────────────┘          └──────────────────┘
```

---

## Protocol Design

### Why BIP-78

BIP-78 (Payjoin) is a recognized Bitcoin standard for constructing multi-party transactions where inputs come from both the sender and receiver. The key insight is that a transaction with inputs from multiple parties cannot be attributed to a single wallet by standard chain analysis heuristics.

`lightning-payjoin-kit` adapts BIP-78 for the specific context of Lightning channel funding, where:

1. The "sender" is the node operator opening a channel
2. The "receiver" is the channel counterparty
3. The "payment" is the channel funding output
4. The timing is asynchronous — both parties do not need to be online simultaneously

---

### The Offline Receiver Problem

Standard BIP-78 assumes both parties are online during coordination. This fails for Lightning because:

- Channel openings are often initiated while the counterparty node is offline
- Lightning nodes go offline for maintenance, routing, and other operational reasons
- Real-time coordination requirements would make Payjoin impractical for channel management

**Solution: Relay-based async coordination**

```
Node A (Initiator)                  Relay Server               Node B (Counterparty)
       │                                 │                              │
       │── POST /v2/payjoin ────────────▶│                              │
       │   (initial PSBT)                │                              │
       │                                 │◀─── GET /v2/poll ────────────│
       │                                 │     (Node B comes online)    │
       │                                 │─── PSBT for signing ────────▶│
       │                                 │◀── Signed PSBT ──────────────│
       │◀── Completed PSBT ─────────────│                              │
       │                                 │                              │
       │── Broadcast to Bitcoin network  │                              │
```

The relay holds the partial PSBT until the counterparty is available. The relay sees only encrypted PSBT data — it cannot read funding amounts or wallet addresses.

---

## PSBT Construction

### Single-Funder (Current — Traceable)

```
Transaction Inputs:
  [0] 0xNode_A_Wallet:0 — 0.05 BTC    ← SINGLE FUNDER. TAGGED.

Transaction Outputs:
  [0] Channel Funding Script — 0.05 BTC
  [1] Change to 0xNode_A_Wallet — 0.001 BTC
```

Chain analysis conclusion: Node A funded this channel from a known wallet. Channel size visible. Funding history clusterable.

---

### Multi-Party Payjoin (With lightning-payjoin-kit — Private)

```
Transaction Inputs:
  [0] 0xNode_A_Wallet:0 — 0.03 BTC    ─┐
  [1] 0xNode_B_Wallet:0 — 0.02 BTC    ─┴── WHO FUNDED WHAT? UNKNOWN.

Transaction Outputs:
  [0] Channel Funding Script — 0.05 BTC
  [1] Change (scattered) — cannot attribute
```

Chain analysis conclusion: Standard multi-input transaction. Cannot determine which input funded the channel. Cannot cluster either wallet's history from this transaction.

---

## UTXO Selection

The UTXO selection algorithm constructs inputs that:

1. Sum to the required channel funding amount
2. Come from at least two distinct wallet addresses (one per party minimum)
3. Produce change outputs that do not reveal the original funding split
4. Are valid inputs for a standard Bitcoin transaction

```rust
pub struct UtxoSelector {
    min_inputs: usize,       // Minimum 2 (one per party)
    target_amount: u64,      // Channel funding amount in sats
    fee_rate: FeeRate,       // Current network fee rate
    coin_selection: CoinSelectionAlgorithm,
}
```

---

## Security Model

### Threat Model

| Threat | Mitigation |
|--------|------------|
| Chain surveillance deanonymizing channel funder | Multi-party inputs prevent single-funder attribution |
| Relay server learning funding amounts | PSBT data encrypted before transmission to relay |
| Man-in-the-middle tampering with PSBT | Signature verification on all PSBT rounds |
| Double-spend via malicious PSBT injection | Input validation before signing in each round |
| Relay denial of service blocking coordination | Timeout with fallback to standard channel opening |
| Counterparty aborting after seeing inputs | Atomic PSBT completion — broadcast only on full signature |

### What This Library Does NOT Protect Against

- IP address correlation (use Tor separately)
- Channel graph analysis (routing privacy requires separate techniques)
- On-chain analysis after channel close (closing transactions are out of scope)
- Timing correlation across multiple channel openings

---

## LDK Integration

`lightning-payjoin-kit` provides a first-class LDK integration interface:

```rust
use lightning::chain::chaininterface::FeeEstimator;
use lightning_payjoin_kit::ldk::PayjoinChannelFunder;

// Replace standard channel funder with Payjoin-enabled version
let payjoin_funder = PayjoinChannelFunder::new(
    standard_channel_manager,
    payjoin_coordinator,
    relay_config,
);

// Channel openings now automatically use Payjoin when counterparty supports it
// Falls back to standard opening if counterparty does not support Payjoin
payjoin_funder.create_channel(
    node_pubkey,
    channel_value_satoshis,
    push_msat,
    user_channel_id,
    override_config,
).await?;
```

---

## Relay Protocol

The relay server implements a minimal HTTP API:

```
POST /v2/payjoin
  Body: Encrypted initial PSBT + session token
  Response: Session ID

GET  /v2/session/{id}
  Response: Current session state + counterparty PSBT if available

POST /v2/session/{id}/sign
  Body: Signed PSBT round
  Response: Next PSBT round or completion signal
```

The relay server is stateless beyond session storage. It cannot read transaction contents. Session tokens are derived from the channel funding pubkeys so only the two channel parties can access their session.

---

## Cryptographic Primitives

| Primitive | Usage | Implementation |
|-----------|-------|----------------|
| secp256k1 | PSBT signing, key derivation | rust-secp256k1 |
| ECDH | Session key derivation between parties | secp256k1 ECDH |
| AES-256-GCM | PSBT encryption for relay transmission | Standard AEAD |
| SHA-256 | Session token derivation | bitcoin_hashes |
| BIP-340 Schnorr | Taproot input signing (where applicable) | rust-bitcoin |

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum PayjoinError {
    #[error("Relay unreachable: {0}")]
    RelayUnreachable(String),

    #[error("Coordination timeout after {seconds}s")]
    CoordinationTimeout { seconds: u64 },

    #[error("Invalid PSBT from counterparty: {0}")]
    InvalidPsbt(String),

    #[error("UTXO selection failed: insufficient funds")]
    InsufficientFunds,

    #[error("Counterparty does not support Payjoin")]
    PayjoinUnsupported,
}
```

All errors include a fallback path to standard channel opening so that Payjoin failure never blocks channel establishment.

---

## Performance

Target performance benchmarks (to be verified in M2 testing):

| Operation | Target |
|-----------|--------|
| PSBT construction | < 50ms |
| Relay round-trip (online counterparty) | < 500ms |
| Relay round-trip (offline counterparty, 30s poll) | < 60s |
| Full coordination to broadcast-ready PSBT | < 90s (online) |

---

## Prior Art and References

- [BIP-78: Payjoin](https://github.com/bitcoin/bips/blob/master/bip-0078.mediawiki)
- [Payjoin Dev Kit (PDK)](https://github.com/payjoin/rust-payjoin) — on-chain Payjoin, OpenSats funded
- [BOLT 2: Channel Establishment](https://github.com/lightning/bolts/blob/master/02-peer-protocol.md)
- [LDK Channel Manager](https://docs.rs/lightning/latest/lightning/ln/channelmanager/struct.ChannelManager.html)
- [rust-bitcoin](https://github.com/rust-bitcoin/rust-bitcoin)
