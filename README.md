# lightning-payjoin-kit

> Asynchronous Rust library bringing collaborative Payjoin privacy (BIP-78) to Lightning Network channel funding

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)
[![Build](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com/ILE-Labs/lightning-payjoin-kit)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)
[![Bitcoin](https://img.shields.io/badge/network-Bitcoin-f7931a)](https://bitcoin.org)
[![Lightning](https://img.shields.io/badge/layer-Lightning-purple)](https://lightning.network)

---

## The Problem

Every time a Lightning Network channel is opened, the funding transaction is broadcast to the public Bitcoin blockchain. Because standard channel openings use single-funder UTXO inputs, anyone analyzing the chain can:

- Identify the exact wallet that funded the channel
- Determine channel size and funding history
- Cluster the node operator's complete financial activity over time

This is not a theoretical risk. It is an active, exploitable metadata leak that affects every Lightning node operator globally — from individuals to businesses running payment infrastructure.

```
# What chain analysis sees today when you open a Lightning channel

TXID: 0x4f2a...b201
Input:  0xYour_Wallet — 0.05 BTC  ← YOUR FUNDING SOURCE. TAGGED.
Output: Channel Funding — 0.05 BTC ← CHANNEL SIZE. VISIBLE.
Output: Change — 0.001 BTC        ← YOUR WALLET. CONFIRMED.

Result: Full node funding history deanonymized.
```

The Payjoin Dev Kit (PDK) addresses this problem for standard on-chain payments. No implementation exists for Lightning channel funding — until now.

---

## The Solution

`lightning-payjoin-kit` brings BIP-78 Payjoin coordination to the Lightning channel establishment flow. Channel openings are constructed as multi-party transactions, eliminating the single-funder heuristic that makes node funding traceable.

```
# What chain analysis sees with lightning-payjoin-kit

TXID: 0x8b3d...c442
Input:  0xParty_A — 0.03 BTC  ← Multiple funders. Who is who?
Input:  0xParty_B — 0.02 BTC  ← Cannot determine.
Output: Channel Funding — 0.05 BTC
Output: Change — scattered

Result: Standard multi-party transaction. Channel funder: unknown.
```

The privacy improvement is automatic and transparent to end users.

---

## Architecture

For a full technical breakdown of the coordination protocol, PSBT construction, async relay design, and security model, see:

📄 **[ARCHITECTURE.md](./docs/ARCHITECTURE.md)**

Key design decisions at a glance:

| Component | Design |
|-----------|--------|
| Coordination | Async relay-based, handles offline receiver |
| Transaction format | BIP-78 Payjoin adapted for channel funding |
| UTXO selection | Multi-party input construction |
| PSBT handling | Round-trip signature coordination |
| Node integration | LDK-compatible, FFI interface for CLN/LND |
| Dependencies | Zero external dependencies in core library |

---

## Features

- **Async coordination engine** — handles the Payjoin request-response cycle even when the funding counterparty is temporarily offline
- **BIP-78 native** — implements the Payjoin standard adapted for Lightning channel contexts, not a custom protocol
- **Zero-dependency core** — the coordination library has no external dependencies beyond standard Bitcoin libraries
- **LDK integration** — first-class support for Lightning Development Kit with examples
- **PSBT round-trip** — complete Partially Signed Bitcoin Transaction handling for multi-party channel funding
- **Relay mechanism** — async relay design solves the offline receiver problem that prevents naive Payjoin from working on Lightning
- **Channel size privacy** — multi-party input construction prevents single-funder heuristic detection

---

## Status

>  This library is under active development. APIs are not yet stable.

| Milestone | Status | Target |
|-----------|--------|--------|
| M1: Async coordination engine |  In progress | Week 4 |
| M2: Channel funding integration |  Planned | Week 8 |
| M3: CLI + crates.io release |  Planned | Week 12 |

See [ROADMAP.md](./docs/ROADMAP.md) for detailed milestone breakdown.

---

## Quick Start

```toml
# Cargo.toml
[dependencies]
lightning-payjoin-kit = "0.1"
```

```rust
use lightning_payjoin_kit::{PayjoinCoordinator, ChannelFundingConfig};

// Initialize coordinator with your node's configuration
let coordinator = PayjoinCoordinator::new(ChannelFundingConfig {
    network: bitcoin::Network::Bitcoin,
    relay_url: "https://relay.payjoin.org".parse()?,
    channel_amount_sats: 1_000_000,
})?;

// Open a channel with Payjoin privacy
// The library handles async coordination automatically
let payjoin_tx = coordinator
    .build_funding_transaction(counterparty_pubkey)
    .await?;

// payjoin_tx is a valid PSBT ready for broadcast
// Channel funding origin is now private
println!("Channel funded privately: {}", payjoin_tx.txid());
```

---

## Installation

### Prerequisites

- Rust 1.75 or later
- Bitcoin node (for testing: bitcoind in regtest mode)
- Lightning node (LDK recommended for integration testing)

### Build from source

```bash
git clone https://github.com/ILE-Labs/lightning-payjoin-kit
cd lightning-payjoin-kit
cargo build
cargo test
```

### Run the CLI (Milestone 3)

```bash
cargo install lightning-payjoin-kit --features cli
lightning-payjoin-kit open-channel --amount 1000000 --peer <node_pubkey>
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](./docs/ARCHITECTURE.md) | Full technical architecture, protocol design, and security model |
| [RESEARCH.md](./docs/RESEARCH.md) | Research process, alternative evaluation, and rationale |
| [ROADMAP.md](./docs/ROADMAP.md) | Detailed milestones, KPIs, and delivery timeline |
| [CONTRIBUTING.md](./CONTRIBUTING.md) | How to contribute to the project |
| [SECURITY.md](./SECURITY.md) | Responsible disclosure policy |

---

## Why This Exists

The Payjoin Dev Kit (PDK), funded by OpenSats, has successfully improved privacy for standard Bitcoin on-chain payments. `lightning-payjoin-kit` is the natural extension of that work into the second layer.

Lightning Network is now the primary payment method for millions of Bitcoin users. The on-chain footprint of their channel management deserves the same privacy treatment that Payjoin has brought to direct on-chain transactions. This library exists to close that gap.

This project is part of ILE Labs' commitment to building open-source Bitcoin infrastructure. We do not build consumer products. We build the layer that other tools depend on.

---

## Team

**ILE Labs** — Open-source blockchain infrastructure

**Onyekachukwu Nweke** — Protocol lead
Threshold signatures, Lightning Network, cross-chain cryptographic protocols

**Gospel Ifeadi** — Core Rust engineering
Systems programming, WASM runtimes, production Rust infrastructure

**Charles Emmanuel** — Project coordination
GitHub: [@CodexEmmzy](https://github.com/CodexEmmzy)
Telegram: [@charlesCode](https://t.me/charlesCode)

---

## Prior Open-Source Work

| Repository | Description |
|------------|-------------|
| [spark-note-poc](https://github.com/ILE-Labs/spark-note-poc) | ZK notes, commitments, nullifiers in Rust — cryptographic primitive foundation |
| [stylus-debug-suite](https://github.com/ILE-Labs/stylus-debug-suite) | Production Rust toolkit, live on crates.io |
| [mx-tx-simulator](https://github.com/ILE-Labs/mx-tx-simulator) | Protocol-level transaction simulator, fixed 4 SDK bugs |
| [solana-cpi-lens](https://github.com/ILE-Labs/solana-cpi-lens) | Transaction call tree reconstruction |

---

## Contributing

This is a free and open-source project for the Bitcoin ecosystem. Contributions are welcome.

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

All contributors must agree to the [Developer Certificate of Origin (DCO)](https://developercertificate.org/).

---

## License

Licensed under the Apache License, Version 2.0.

See [LICENSE](./LICENSE) for the full license text.

```
Copyright 2026 ILE Labs

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0
```

---

## Contact

- **Telegram:** [@charlesCode](https://t.me/charlesCode)
- **Email:** contact@ilelab.org
