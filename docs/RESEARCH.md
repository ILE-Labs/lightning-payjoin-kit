# Research & Rationale — lightning-payjoin-kit

This document outlines the research process, technical evaluation of alternatives, and the specific rationale behind why `lightning-payjoin-kit` is the optimal approach for bringing channel funding privacy to the Lightning Network.

---

## The Core Research Problem

The Lightning Network is fundamentally an off-chain scaling and privacy solution. However, establishing a Lightning channel requires an on-chain transaction. 

Historically, this has been a **single-funder transaction**: one party supplies the UTXOs, and the other party contributes nothing to the initial on-chain footprint. Chain analysis heuristics rely heavily on this single-funder pattern to deanonymize Lightning node operators, successfully clustering node funding history and tracking financial flows.

Our research objective was to answer one question: **How can we eliminate the single-funder heuristic in Lightning channel openings without requiring breaking protocol changes to the Bitcoin base layer?**

---

## Evaluation of Alternatives

During our research phase, we evaluated several existing privacy primitives to determine their suitability for Lightning channel funding.

### 1. CoinJoin (Standard)
* **Mechanism:** Multiple users mix their UTXOs in a single large transaction with equal-sized outputs.
* **Why it was rejected:** CoinJoin introduces significant friction. It requires a critical mass of participants, entails coordination delays, and produces highly recognizable transaction graphs. For node operators needing to open channels on demand, relying on a global mixing pool is neither practical nor timely.

### 2. Dual-Funded Channels (BOLT V2 Proposals)
* **Mechanism:** Both peers contribute capital to the channel opening.
* **Why it was rejected (as a standalone privacy fix):** While dual-funding allows both parties to contribute liquidity, it does not inherently obfuscate *which* UTXOs belong to *whom*. Without careful UTXO selection and transaction shaping, chain surveillance can still deduce the input ownership based on change outputs. Furthermore, true dual-funding requires the counterparty to lock up their own capital, which is not always desirable or possible in standard routing contexts.

### 3. Taproot / MuSig2
* **Mechanism:** Aggregating public keys so multi-sig outputs look like single-sig outputs.
* **Why it was rejected (as a standalone privacy fix):** Taproot hides the *output* script complexity (making channel opens look like normal P2TR spends). However, it does nothing to obscure the *inputs*. The single-funder heuristic still applies flawlessly to the funding inputs.

---

## The Payjoin Insight (BIP-78)

Our research concluded that **Payjoin (BIP-78)** is the optimal primitive for this specific problem. 

Payjoin breaks the core assumption of chain analysis: the common-input ownership heuristic (the assumption that all inputs in a transaction belong to the same entity). In a standard payment, Payjoin allows the receiver to contribute inputs to the transaction alongside the sender.

### Why Payjoin is the Best Approach for Lightning

1. **No protocol changes required:** It uses standard Bitcoin transactions.
2. **Denial of metadata:** It makes it mathematically impossible for outside observers to determine which inputs funded the channel.
3. **No extra fees:** Unlike CoinJoin, it does not require additional transaction overhead or mixing fees.
4. **No counterparty capital lockup:** The counterparty simply provides inputs and receives them right back as change; they do not have to actually fund the Lightning channel itself.

### The Missing Link: Async Coordination

While BIP-78 is mathematically ideal, our research identified a critical engineering roadblock: standard Payjoin requires synchronous, real-time HTTP interaction between sender and receiver. This works for merchant checkouts, but **fails for Lightning node operations**, where nodes routinely go offline, routing nodes establish channels asynchronously, and direct HTTP connections between peers are often blocked by firewalls or Tor routing layers.

---

## The Solution: Async Relay Coordination

To bridge the gap between BIP-78's real-time requirement and Lightning's asynchronous reality, we engineered the **Async Relay Coordination Protocol**, which forms the backbone of `lightning-payjoin-kit`.

### Our Process
1. **Threat Modeling:** We mapped out the exact data visible to an intermediary relay. We determined that the relay must be blind to transaction values and wallet addresses.
2. **Cryptographic Design:** We implemented an AES-256-GCM encryption layer over the PSBT payload. Session keys are derived via ECDH from the public keys already used in the Lightning channel establishment phase.
3. **State Machine Modeling:** We designed a robust `PayjoinCoordinator` state machine that handles timeouts, retry logic, and seamless fallbacks to single-funder transactions if the counterparty remains offline.

### Why This Project is the Best to Consider

`lightning-payjoin-kit` is not merely a theoretical proof-of-concept; it is purpose-built for production integration. 

* **It addresses an active, exploited vulnerability** (node funding deanonymization).
* **It utilizes established standards** (BIP-78) rather than unproven custom cryptography.
* **It solves the synchronization barrier** that previously prevented Payjoin adoption in Lightning.
* **It degrades gracefully**, meaning it never prevents a channel from opening; it simply defaults back to standard behavior if coordination fails.
* **It provides native LDK integration**, making it instantly usable for the largest ecosystem of Lightning developers.

By implementing this architecture, we provide a definitive, zero-friction solution to the single-funder metadata leak, dramatically enhancing the base-layer privacy of the Lightning Network.
