# lightning-payjoin-kit

A Rust library for constructing Lightning channel funding transactions
collaboratively, together with the research record and harnesses used to measure
whether that construction actually delivers privacy.

**Status: research prototype.** Two things are true at once, and both matter:

- **The prototype works end to end.** It compiles, 33 tests pass, two real LDK
  `ChannelManager` instances accept its funding outpoint and reach a usable
  channel, and Bitcoin Core accepts and mines its transactions on regtest.
- **The privacy question it was built to answer is now settled**, and the answer is
  a measured negative that generalises beyond this library. That result, and the
  harnesses that produced it, are the project's main output to date.

The library constructs and validates funding transactions. It does not open
channels; channel state transitions remain the responsibility of the node
implementation.

### Start here

| | |
|---|---|
| **[The research finding](./research/02-the-central-result.md)** | Why collaborative funding with returned change is partitionable, and what that rules out |
| **[Research record](./research/)** | Corrections, measurements, sources — the evidence base |
| **[Open questions](./research/04-open-questions.md)** | Eight questions, each with a recorded result including the negatives |
| **[Prior art](./research/06-prior-art.md)** | Dual funding, nolooking, and what this project adds |
| **[Roadmap](./ROADMAP.md)** | Verification-led; the current phase evaluates constructions before implementing any |
| **[Build and test](#build-and-test)** | Commands, including the LDK and Bitcoin Core integration tests |

---

## What works today

This part is verified and is not affected by the privacy finding.

**LDK integration, through the manual funding path.** `Event::FundingGenerationReady`
supplies the funding script, collaborative construction fixes the final outpoint,
`unsafe_manual_funding_transaction_generated` commits the channel to that outpoint,
and broadcast waits for `Event::FundingTxBroadcastSafe`. The ordering matters:
collaborative construction changes the transaction id, so the outpoint must be
settled before commitment signatures are exchanged, and the transaction must not be
broadcast until those signatures make it safe.

This works over **v1 channel establishment**, so the counterparty node needs no
dual-funding support and no modification. Two real `ChannelManager` instances reach
a usable channel in the two-node harness.

**Bitcoin Core.** A collaborative funding transaction built by the library is
accepted and mined on regtest through `corepc-client`.

**Proposal validation.** Malicious proposals that modify the funding output, steal
change, or inflate fees are rejected, and the channel funding output value equals
the negotiated capacity.

```bash
cargo test                                                          # 33 tests
cargo test --features ldk-test-utils --test ldk_two_node_harness -- --ignored
cargo test --features corepc --test corepc_regtest -- --ignored
```

---

## The problem

Opening a Lightning channel requires an on-chain transaction. In the standard case
one party supplies every input, so the common-input-ownership heuristic applies
correctly and clustering software attributes the funding source to the operator.
From there an observer follows the operator's inputs backwards until they reach a
KYC'd withdrawal, and the node has a name attached.

This is measured, not hypothetical. Kappos et al. identified at least one
participant in **86.8%** of private-channel opening transactions — 7.5% both
participants, 79.3% one — using on-chain coin flow alone, with no gossip data. For
public channels they identified the opening participant in 89.0%.

Their attack follows the "peeling chain": operators fund each new channel from the
change of the previous open, and spend closing outputs into new opens. That is a
coin-selection pattern, and no amount of care inside a single transaction touches
it.

---

## The research finding

The construction under test: a second party contributes an input to the funding
transaction and receives the same value back as change, so the transaction has
inputs under more than one ownership while the contributor locks up no capital and
takes no channel balance.

**Measurement finds that this construction can be partitioned by an observer with
near-certainty, and that every remediation planned for it leaves it that way.**

The reason is structural rather than arithmetic, which is what makes the result
general. Write `V` for the contributor's input and `C` for their change. They pay
only fees, so `|V − C| ≤ F`, the total transaction fee. The initiator pays for the
channel, so their input and change differ by at least the capacity `K`. For every
channel worth opening `K` is orders of magnitude larger than `F` — from about 33×
at the least favourable corner tested to about 2,000× at the most common one. An
observer sorts input/output pairs by absolute difference and takes the smallest.

Three consequences follow, and each is the point:

- **Randomising the fee apportionment does not help.** It changes the contributor's
  share, but that share is bounded by `F` however it is drawn, and the bound is what
  the attack uses.
- **Shuffling the outputs does not help.** Sorting by absolute difference does not
  care what order the outputs arrive in.
- **The result does not depend on the UTXO distribution.** Neither input value
  appears in the inequalities, so no distribution of realistic UTXOs repairs it.

Attack success over 20,000 generated transactions against a 50% chance baseline,
stable to within 0.11 percentage points across three seeds:

| Attack | As built | Every planned fix applied | Contribution absorbed |
|---|---|---|---|
| Fee residue | 99.997% | 50.23% | n/a |
| Subset-sum partitioning | 99.86% | 99.85% | 49.96% |
| Near-equality (no fee arithmetic) | 99.75% | 99.76% | 57.04% |
| Near-equality, all outputs taproot | — | 99.49% | 53.27% |

Randomised fee apportionment defeats the fee-residue attack completely. It moves
subset-sum partitioning by one hundredth of a percentage point. The decisive attack
uses no fee arithmetic at all, and taproot outputs do not reach it either:
concealing *which* output is the channel does not conceal which input and output
belong to the same party.

### Two findings that reach past this library

**The deterministic fee residue is what BOLT 2 specifies.** The apportionment rule
that produces it is what BOLT 2 requires of every peer in an interactively
constructed transaction. So the residue is not a defect unique to this
implementation — it is shared with dual-funded channel opens — and any fix makes the
construction non-conformant wherever interactive-tx applies.

**The anti-fee-sniping fix would make the transaction rarer, not commoner.**
`nSequence = 0xfffffffd` with `nLockTime = 0` matches 64.68% of mainnet traffic,
against 28.09% for the current settings. Anti-fee-sniping, which LDK recommends,
would move the transaction into a 4.46% minority.

### A second, independent problem

The transaction's structural signature — 2 inputs, 3 outputs, one P2WSH and two
P2WPKH — matches **2 of 150,769** mainnet transactions in a 29.8-day sample, about
1 in 75,000. Recast as taproot it matches 7, about 1 in 21,500.

Being that rare means an analyst can enumerate candidates cheaply; being
partitionable means each one then gives up its ownership structure. Both share a
root: a contributor who takes their money back needs a third output, and 84.41% of
mainnet transactions have exactly two.

### What this rules in

A construction in which the contributed value is **absorbed into the funding
output** resists subset-sum partitioning outright and needs one fewer output. That
is where the work now points, and it is what [the roadmap](./ROADMAP.md) evaluates
before any of it is implemented. It is also, notably, what BOLT 2 dual funding
already does.

The full finding, including everything it does not establish, is in
[research/02-the-central-result.md](./research/02-the-central-result.md). The
harness that produced it is in
[research/adversarial-harness/](./research/adversarial-harness/) and runs with one
command.

---

## What the library actually is today

- A **synchronous** Rust library. No async, no runtime, no I/O of its own.
- **Payjoin-inspired, not BIP-78.** No HTTP endpoint, no BIP21 parameters, none of
  the BIP-78 wire format. Messages are serialized PSBTs tagged with a payload kind.
- **Five mandatory dependencies**: `bitcoin`, `secp256k1`, `serde`, `serde_json`,
  `thiserror`. `corepc-client` and `lightning` are optional and feature-gated.
- **No cryptography of its own.** Earlier documentation described an AES-256-GCM
  layer with ECDH-derived session keys. It does not exist and never did; the claim
  has been withdrawn.
- **Not published.** Not on crates.io. There is no command-line tool.

Nine claims from the earlier documentation did not survive verification. All nine,
and their replacements, are in
[research/01-claims-corrections.md](./research/01-claims-corrections.md).

---

## Known defects in the construction

Six, each with the correct construction and a citation for it, in
[research/03-construction-risks.md](./research/03-construction-risks.md).

| | Defect | Status |
|---|---|---|
| R-P1 | Contributor's change is their input minus exactly `99 × feerate`, so the fee rate is recoverable | Confirmed. 59 of 59 sampled transactions — and the rule is what BOLT 2 specifies |
| R-P2 | Fixed output ordering; funding output always at index 0 | Confirmed. 100% of samples |
| R-P3 | `nSequence = 0xffffffff` and `nLockTime = 0` | Confirmed, and the fix is narrower than it first appeared |
| R-P4 | The P2WSH funding output is charged at the P2WPKH size of 31 bytes, not its true 43 | Confirmed. Effective fee rate is 94.84% of target at every rate tested |
| R-P5 | No rate limiting and no reuse-after-abort policy, so contributor UTXOs can be enumerated for free | Confirmed. Optech flagged this class for dual funding in 2021 |
| R-P6 | The contributor signs first and, unlike BIP-78's receiver, has no broadcastable fallback | Confirmed. Discussed against BIP-78's ordering and its rationale |

R-P1 is the one the project had treated as critical. Fixing it does not deliver
privacy, and fixing it *alone* would be the most misleading outcome available: the
one attack the project had named would stop working while the transaction stayed
fully partitionable.

---

## Verification status

| Target | Result |
|---|---|
| V1 adversarial partitioning | **FAIL** — 99.85% with every planned fix applied |
| V2 LDK two-node regression | **PASS** |
| V3 Bitcoin Core regtest | **PASS** — mined into a real block |
| V4 fee accuracy within 5% | **FAIL** — 94.84% of target at every rate tested |
| V5 structural indistinguishability | **FAIL** — signature matches 2 of 150,769 mainnet transactions |
| V6 probing resistance | **Not testable as written** — the pass condition cannot be met by any design; the record proposes a restatement |
| V7 round-trip timing | **PASS** — about four orders of magnitude of headroom |
| V8 value conservation | Covered by the test suite; not independently re-verified |

V1 was the gate. V1 failed, and pivot trigger T1 was invoked on the evidence — which
is the mechanism working as designed, not the project stopping.

---

## Two privacy profiles

The library is announcement-agnostic. What differs between announced and
unannounced channels is not the mechanism but the achievable result, and the two
do not share a claims table.

**Profile A, unannounced channels.** No `channel_announcement`, so an observer has
no pointer to the funding output, no capacity figure and no node identities. Simple
taproot channels compose here.

**Profile B, announced channels.** Gossip publishes the funding outpoint, both node
identities and the capacity. Those three are permanently public; routing requires
them. Simple taproot channels **do not** compose here: the specification says the
type "cannot be announced on the public network", and LND's release notes say
taproot channels "must remain private until announced taproot channels are
supported".

So Layer 3 is available only to the profile that needs it least, which inverts the
arrangement earlier planning assumed. Measured against real traffic, taproot
multiplies the anonymity set by about 3.5× — from 1 in 75,000 to 1 in 21,500 — a
real improvement and not a solution. Set out in
[research/05-composed-layers.md](./research/05-composed-layers.md).

**On the measurements to date, neither profile gets funding-source unlinkability
from this construction.** An earlier claims table gave both profiles "Yes" for
unlinkable funding source, unconfirmed change, and a broken peeling chain. None of
those three is currently supported for either profile, and the table will be
rebuilt from measurements rather than from the mechanism's intent.

---

## Honest limitations

**Permanently out of reach.** An announced channel's capacity, because routing
requires it. A coin's history before the operator acquired it. The fact that a
transaction occurred at all.

**Out of reach of this layer specifically.** The peeling chain. Kappos et al.'s
tracing heuristic reads transitions *between* transactions, and no construction
inside a single transaction touches it. This is the attack that identified 86.8% of
private-channel participants, and the answer to it is privacy-aware coin selection
— Phase 3 of [the roadmap](./ROADMAP.md), which is cheaper than the construction
work and addresses a larger measured harm.

**Residual even in the good case.** A colluding contributor knows which input was
theirs. Statistical analysis across many opens by one operator may still cluster.

**The standard this project holds itself to.** The technique is a probabilistic
degradation of an adversary's confidence, not a proof of unlinkability. This
project does not claim, and its documentation will not claim, that attribution
becomes impossible. The claim is that it becomes unreliable — and on current
measurements the library does not yet deliver even that.

---

## Prior art

Collaborative funding of a Lightning channel is not new. BOLT 2 channel
establishment v2 has both peers contribute inputs and is live in Core Lightning and
Eclair. [nolooking](https://github.com/payjoin/nolooking) has opened channels from
inbound BIP-78 payjoins since 2022.

What this library adds is narrower: a construction in which the second party
contributes an input and receives it back in full as change, taking on no channel
balance and no capital lockup, over v1 channel establishment, so the peer needs no
dual-funding support and no modification.

That is a difference in mechanism, not in privacy achieved — and the measurements
above find it is the reason the construction does not work. See
[research/06-prior-art.md](./research/06-prior-art.md).

---

## Build and test

Requires Rust 1.75 or later.

```bash
git clone https://github.com/ILE-Labs/lightning-payjoin-kit
cd lightning-payjoin-kit
cargo build
cargo test          # 33 tests, default feature set
```

Optional adapters:

```bash
cargo check --features corepc   # Bitcoin Core RPC
cargo check --features ldk      # LDK funding adapter
```

Integration tests that need external services are `#[ignore]`d:

```bash
docker compose up -d bitcoind
cargo test --features corepc --test corepc_regtest -- --ignored
cargo test --features ldk-test-utils --test ldk_two_node_harness -- --ignored
```

The regtest test defaults to `http://127.0.0.1:18443` with RPC credentials
`lpk`/`lpk`; override with `LPK_COREPC_URL`, `LPK_COREPC_USER` and
`LPK_COREPC_PASSWORD`.

## Run the research harnesses

```bash
cd research/adversarial-harness && ./run.sh   # partitioning attacks, seeded
cd research/mainnet-baseline    && ./run.sh   # fetches 31 mainnet blocks, parses locally
cd research/roundtrip-timing    && ./run.sh   # round-trip latency under load
cd research/fee-arithmetic      && ./run.sh   # effective fee rate against target
```

The adversarial harness has no dependencies beyond the Rust standard library and is
deterministic given a seed, so output is byte-identical across runs on the same
toolchain. Override with `SEED` and `SAMPLES`.

The mainnet baseline ships a manifest pinning the anchor height, the stride and all
31 block hashes, so `./run.sh` refetches an identical sample rather than a similar
one. It verifies it consumed exactly each block's byte length and that its
transaction count matches the explorer's, so a mirror serving bad data is caught
rather than silently analysed.

---

## Documentation

| Document | Description |
|---|---|
| [research/](./research/) | The research record: corrections, measurements, open questions, sources |
| [ROADMAP.md](./ROADMAP.md) | Verification-led roadmap and release gates |
| [IMPLEMENTATION.md](./IMPLEMENTATION.md) | What the prototype implements, the funding flows, and how to exercise them locally |
| [CONTRIBUTING.md](./CONTRIBUTING.md) | How to contribute |
| [SECURITY.md](./SECURITY.md) | Responsible disclosure |

Earlier design documents — `docs/ARCHITECTURE.md`, `docs/RESEARCH.md` and
`docs/ROADMAP.md` — are superseded. They are kept unedited, each behind a banner
pointing at the corrections, so that readers who saw the originals can see exactly
what was claimed and what replaced it.

## License

Apache 2.0. See [LICENSE](./LICENSE).
