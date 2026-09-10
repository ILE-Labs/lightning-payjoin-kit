# Construction risks

Six findings against the code, each confirmed against the tree, with the correct
construction researched and cited. Two further sections follow: what BOLT 2's fee
rule implies for the first of them, and a note on the LDK integration.

Every one of these was also confirmed in a transaction Bitcoin Core accepted and
mined on regtest — see [01-claims-corrections.md](./01-claims-corrections.md).

---


# R-P1 · Deterministic fee residue

**Confirmed against the tree.** `src/psbt/builder.rs:254-256`:

```rust
fn privacy_input_added_fee(&self) -> u64 {
    (P2WPKH_INPUT_VBYTES + P2WPKH_OUTPUT_VBYTES) * self.fee_rate_sat_vb
}
```

with `P2WPKH_INPUT_VBYTES = 68` and `P2WPKH_OUTPUT_VBYTES = 31` at lines 8-9, so
the figure is `99 * fee_rate_sat_vb`. At `:180-184` the contributor's change is
`counterparty_utxo.value - required_contribution`. The difference between the
contributor's input and their change is therefore exactly `99 x feerate`, and the
fee rate is recoverable from the transaction.

**Measured.** Experiment 01: 59 of 59 constructed transactions, fee rates 1-300
sat/vB, contributor values 20,000-5,000,000 sat. The residue equalled `99 x
feerate` in every case and `residue / 99` recovered the exact rate in every case.
One of the 60 sampled configurations (20,000 sat at 300 sat/vB) was rejected by
library policy before a transaction existed, because 99 x 300 exceeds the UTXO.

Experiment 02: the fee-residue attack identifies the contributor's pair in
99.997% of 20,000 samples, against a 50% chance baseline.

**The correct construction.** Randomise the apportionment so the difference
carries no fixed relationship to the fee rate. Experiment 02's construction B
draws the contributor's share uniformly from +/-60% of their marginal cost and
takes the attack to 50.23%, which is chance.

**But.** See the central result. Removing the residue moves subset-sum partitioning from
99.86% to 99.85%. R-P1 is real, the fix works against R-P1, and fixing it does
not make the construction private. Fixing R-P1 alone would be the most misleading
possible outcome: the one attack the project has named would stop working while
the transaction stayed fully partitionable.

**Prior art on the rule itself.** See the BOLT 2 fee-rule note: this apportionment is what BOLT 2
"Fee Responsibility" requires of every peer in an interactively constructed
transaction, so the residue is a property of dual-funded opens too, and
randomising it diverges from the spec anywhere interactive-tx applies.

---

# R-P2 · Deterministic structure

**Confirmed against the tree.** The funding output is pushed first in both
constructors (`src/psbt/builder.rs:56-59` and `:79-82`); `FallbackFunding` reports
`funding_output_index: 0` unconditionally (`:149`); initiator change is appended
second and reported as index 1 (`:140-144`); the contributor's input is appended
at the end of the input vector (`:193`) and their change at the end of the output
vector (`:208`). No shuffle anywhere in the file.

**Measured.** Experiment 01, Part E, on a representative construction:
funding output at index 0, initiator change at 1, contributor change at 2.
Experiment 02, across 20,000 samples: funding output at index 0 in 100.00% of
transactions.

**The correct construction.** BOLT 2 specifies the mechanism: "Inputs in the
constructed transaction MUST be sorted by `serial_id`" and "Outputs in the
constructed transaction MUST be sorted by `serial_id`", where `serial_id` "is a
randomly chosen number". Sorting on a random key gives uniform placement and is
what the ecosystem already does for interactively constructed transactions.

Experiment 02's construction B shuffles instead, which is equivalent for one
transaction, and lands the funding output at index 0 in 33.66% of samples —
uniform over three positions, as expected.

**Weight.** On its own this is a library fingerprint rather than a partitioning
vector: it says "this transaction was built by this code", not "this input belongs
to that party". It matters because it makes the population of affected
transactions enumerable, which is what turns a weak per-transaction leak into a
usable corpus.

---

# R-P3 · Anomalous transaction metadata

**Confirmed against the tree.** `lock_time: LockTime::ZERO` at
`src/psbt/builder.rs:54` and `:125`; `sequence: Sequence::MAX` at `:118` and
`:197`; `version: Version::TWO` at `:53` and `:124`.

**Measured.** Experiment 01, Part E: version 2, locktime 0, both inputs
`0xffffffff`. Experiment 02: across 20,000 samples, all sequences final in
100.00%, locktime zero in 100.00%.

**Why `Sequence::MAX` is the field that matters.** `nLockTime = 0` is not on its
own distinctive — Bitcoin Core produces it deliberately when its chain view is
stale, precisely to avoid a fingerprint: "To avoid leaking a potentially unique
'nLockTime fingerprint', set nLockTime to a constant"
(`src/wallet/spend.cpp:1033-1036`). The sequence is different. Two independent
sources say `0xffffffff` is wrong here:

- **BOLT 2** requires a sending node to "set `sequence` to be less than or equal
  to 4294967293 (`0xFFFFFFFD`)" and requires the receiver to "fail the negotiation
  if `sequence` is set to `0xFFFFFFFE` or `0xFFFFFFFF`". The rationale states the
  purpose: "it must signal replaceability, and the same value should be used
  across implementations to avoid on-chain fingerprinting."
- **Bitcoin Core** uses `MAX_BIP125_RBF_SEQUENCE{0xfffffffd}` (`src/util/rbf.h:12`)
  when signalling RBF and `MAX_SEQUENCE_NONFINAL` = `0xfffffffe`
  (`src/primitives/transaction.h:82`) otherwise. It never emits `0xffffffff` from
  the wallet, because a final sequence disables nLockTime and so disables
  anti-fee-sniping: "The sequence number is set to non-maxint so that
  DiscourageFeeSniping works" (`src/wallet/spend.cpp:1286`).

So the library's value is simultaneously the one BOLT 2 names as a negotiation
failure and one Bitcoin Core's wallet will not produce.

**What real mainnet traffic says.** Experiment 04 measured all 150,769
non-coinbase transactions in 31 blocks spanning 29.8 days. This changes the
recommendation, and the earlier version of this file — which advised adopting
Bitcoin Core's anti-fee-sniping — was wrong.

| nSequence, per input | Share of 246,647 inputs |
|---|---|
| `0xfffffffd` (BIP125 RBF) | **54.48%** |
| `0xffffffff` (final) | 41.05% |
| `< 0xfffffffd` (other RBF) | 4.23% |
| `0xfffffffe` (non-final) | 0.24% |

| nLockTime | Share of transactions |
|---|---|
| zero | **95.34%** |
| height within 100 of its own block (anti-fee-sniping) | 4.46% |
| other block height | 0.10% |
| timestamp | 0.10% |

`nSequence = 0xffffffff` is a minority at 41% but nothing like an anomaly — plenty
of real wallets emit it. And `nLockTime = 0`, which the earlier analysis treated as
part of a distinctive combination, is what **95%** of mainnet does. Anti-fee-sniping
is the rare behaviour, not the common one.

Ranking the combinations:

| Combination | Share |
|---|---|
| all RBF-signalling + nLockTime 0 | **67.10%** |
| all `0xfffffffd` + nLockTime 0 | **64.68%** |
| all `0xffffffff` + nLockTime 0 — *the library today* | 28.09% |
| all RBF-signalling + anti-fee-sniping locktime | **4.25%** |

**The correct construction, revised against measurement.** Set
`nSequence = 0xfffffffd` on every input and leave `nLockTime = 0`.

That single change moves the library from 28.09% of traffic to 64.68%, and it is
the only option that improves every axis at once:

- it satisfies BOLT 2's requirement (`<= 0xFFFFFFFD`) and its stated goal that "the
  same value should be used across implementations to avoid on-chain
  fingerprinting";
- it is exactly what Bitcoin Core emits when signalling RBF
  (`MAX_BIP125_RBF_SEQUENCE{0xfffffffd}`, `src/util/rbf.h:12`);
- it enables fee-bumping a stuck funding transaction, which the current
  construction prevents;
- and it more than doubles the share of traffic the transaction's metadata matches.

**On anti-fee-sniping specifically — a tension, not a simple error.** Bitcoin
Core's `DiscourageFeeSniping` (`src/wallet/spend.cpp:997-1042`) sets
`nLockTime = block_height`, and one time in ten steps uniformly back up to 100
blocks. LDK's documentation on the very function this library calls recommends it
by name: "we recommend the wallet software generating the funding transaction to
apply anti fee sniping as implemented by Bitcoin Core wallet."

Both are right about their own goal. Anti-fee-sniping exists to align miner
incentives — Core's comment says "we always want the blockchain to move forward" —
which is a network-health argument, not a privacy one. Core does also claim a
privacy benefit, but a narrow one, for "transactions that are delayed after signing
for whatever reason, e.g. high-latency mix networks and some CoinJoin
implementations", and a collaborative funding round-trip plausibly qualifies.

Weighed against that: the measured cost of adopting it is joining a 4.46%
population instead of a 95.34% one. On the evidence available, blending in wins.
If the project adopts anti-fee-sniping anyway for the miner-incentive reason, that
is a defensible choice and it should be recorded as accepting a fingerprinting cost
for a network-health benefit — not as a privacy improvement.

**A caveat on the whole field-level discussion.** Experiment 04 also finds that
metadata is not what identifies these transactions. The library's exact structural
signature — 2 inputs, 3 outputs, one P2WSH and two P2WPKH outputs — matches 2 of
150,769 transactions, about 1 in 75,000, and the metadata fields barely move that
number. Fixing R-P3 is correct and cheap; it is not what makes the transaction
identifiable. See U3.

**One more metadata signal, from the literature.** Ghesmati et al. observe of a
real payjoin transaction that "different nSequence fields also reveal that the
inputs were added by different wallets", noting wallet fingerprinting is outside
their scope. Uniform sequences across both parties' inputs is therefore the goal,
and BOLT 2's "the same value should be used across implementations" is the reason
a single agreed constant beats each party choosing its own.

---

# R-P4 · Fee under-estimation

**Confirmed against the tree.** `src/psbt/builder.rs:247-252`:

```rust
fn estimated_fee(&self, input_count: usize, output_count: usize) -> u64 {
    let vbytes = TX_OVERHEAD_VBYTES
        + P2WPKH_INPUT_VBYTES * input_count as u64
        + P2WPKH_OUTPUT_VBYTES * output_count as u64;
    vbytes * self.fee_rate_sat_vb
}
```

Every output is charged 31 vbytes, including the P2WSH channel funding output.

**Measured, not assumed.** Experiment 01, Part A, serializes both output types:

| Output | Serialized bytes |
|---|---|
| P2WPKH | 31 |
| P2WSH | 43 |

`8` (value) `+ 1` (script length varint) `+ 22` for P2WPKH's `OP_0 <20-byte push>`;
`8 + 1 + 34` for P2WSH's `OP_0 <32-byte push>`. The estimator undercharges the
funding output by exactly 12 vbytes. R-P4's "approximately 43" is exactly 43.

**The full shortfall is 13 vbytes, not 12.** Experiment 01, Part C, at every
sampled fee rate:

| Fee rate | tx vsize | fee paid | effective rate | % of target |
|---|---|---|---|---|
| 1 | 252 | 239 | 0.9484 | 94.84% |
| 10 | 252 | 2,390 | 9.4841 | 94.84% |
| 100 | 252 | 23,900 | 94.8413 | 94.84% |

Decomposition: 12 vbytes from the P2WSH output, plus the segwit marker and flag
(2 weight units = 0.5 vbytes) which `TX_OVERHEAD_VBYTES = 10` omits, plus 0.5
vbytes of rounding up to the next whole vbyte. `239 + 13 = 252`.

**V4 fails at every rate.** The MVP's verification target V4 requires the
effective rate to fall within 5% of the requested rate. The measured shortfall is
a constant 5.16%. V4 fails 7 of 7 sampled rates — narrowly, and consistently.

**It is the fallback path that is wrong, not the collaborative one.** The
single-funder fallback estimates 140 and measures 153: the same 13-vbyte gap. The
collaborative step then adds exactly 99 estimated and exactly 99 measured. So the
entire error is in the base construction and the contributor's share is correctly
priced. Fixing the estimator does not change what the contributor pays.

**The correct construction.** Charge each output its real serialized size by
script type, add the 2 weight units for the segwit marker and flag, and round the
total up to whole vbytes. `bitcoin`'s own `Transaction::vsize` on a
witness-populated clone is the simplest check, and is what experiment 01 uses as
its reference.

---

# R-P5 · Unbounded contributor probing

**Confirmed against the tree.** The only freshness check is
`CounterpartyOriginalValidator::validate_counterparty_input_is_fresh`
(`src/payjoin/validator.rs:54-69`), which asks whether the contributor's own UTXO
already appears in *this* PSBT. It does not consult any history. There is no
per-peer state, no rate limit, no record of previously exposed UTXOs and no
reuse-after-abort policy anywhere in `src/`. An initiator can open and abandon
sessions repeatedly and be shown a fresh UTXO each time, at no cost.

**The claim that Optech flagged this is verified.** Bitcoin Optech Newsletter
#131, 2021-01-13:

> "Before the initiator can sign the dual funding transaction, they need the
> identities (outpoints) of all of the UTXOs the other party wants to add to the
> transaction. This creates a risk that an abuser will attempt to initiate
> dual-funded channels with many different users, learn their UTXOs, and then
> refuse to sign the funding transaction—harming those users' privacy at no cost
> to the abuser."

The shape is identical here, with the initiator probing the contributor.

**Three known mitigations, none of them free.**

1. **Reuse the exposed UTXO.** BIP-78's approach: "When the receiver detects an
   original transaction being broadcast, or if the receiver detects that the
   original transaction has been double spent, then they will reuse the UTXO that
   was exposed for the next payjoin." BOLT 2 arrives at the same answer for
   liquidity griefing: "It is thus recommended that implementations keep UTXOs
   unlocked and actively reuse them in concurrent sessions."
2. **Make the initiator post a bond.** Optech #131 reports Lloyd Fournier's
   proposal: "the initiator create and sign (but not broadcast) a transaction that
   spends their UTXO back to themselves … if the initiator later fails to sign the
   actual funding transaction, the respondent can broadcast the good-faith
   transaction, forcing the initiator to pay an onchain fee." Two earlier
   proposals used PoDLEs and `SIGHASH_SINGLE|SIGHASH_ANYONECANPAY` half-signed
   transactions.
3. **Restrict the contributor role to known peers**, which is pivot trigger T3's
   response.

**BIP-78's own trigger does not transfer.** Its reuse rule fires when the receiver
observes the original transaction broadcast or double-spent. There is no
broadcastable original here — a Lightning funding transaction cannot be published
before commitment signatures exist — so the contributor has no such signal and
must key reuse on abort and timeout instead. That is a design difference, not a
detail.

**BIP-78 says the guarantee cannot be absolute**, and says so twice: "While we
cannot prevent this type of attack entirely", and, on reuse, "there is no strong
guarantee about it. This prevents the attacker from detecting with certainty the
next payjoin of the merchant to another peer." The second sentence is the
important one: a *strict* one-UTXO-per-peer rule is itself an information leak,
because an attacker who learns the rule learns which UTXO the contributor will
present next.

**This puts V6's pass condition in question.** V6 requires that the contributor
"exposes no more than one distinct UTXO to that peer regardless of N". Reuse
achieves that for as long as the UTXO remains unspent, and BIP-78 deliberately
declines to make it a guarantee for the reason above. V6 as written is
achievable in the abort-only case and is not achievable in general; it should be
restated with the spend case and the deliberate-non-determinism case handled
explicitly.

**One asymmetry in this project's favour, and one against.** BIP-78 notes that
"probing attacks are only a problem for automated payment systems such as BTCPay
Server. End-user wallets with payjoin capabilities are not affected, as the
attacker can't create multiple invoices". A Lightning node that accepts channels
from strangers is the automated case, not the end-user case, so the full problem
applies. Against that, a channel open requires a peer connection and a channel
negotiation, which is more expensive for an attacker than requesting an invoice.
Neither observation was quantified here.

---

# R-P6 · Contributor signs first

**Confirmed against the tree.** The flow in `src/payjoin/` and `src/funding/` has
the contributor build and return the proposal
(`FundingPsbtBuilder::build_privacy_input_proposal`) before the initiator has
committed anything. The initiator then validates through
`InitiatorProposalValidator` and signs.

## Comparison with BIP-78's ordering

BIP-78 §Protocol states the sequence:

> "* The sender creates a signed, finalized PSBT with witness UTXO or previous
> transactions of the inputs. We call this PSBT the `original`.
> * The receiver replies back with a signed PSBT containing his own signed
> inputs/outputs and those of the sender. We call this PSBT `Payjoin proposal`.
> * The sender verifies the proposal, re-signs his inputs and broadcasts the
> transaction to the Bitcoin network."

and requires of the original that it "MUST … Be broadcastable."

So BIP-78's ordering is the reverse of this library's: the *sender* signs first,
and the thing they sign is a complete, broadcastable payment.

## Why BIP-78 orders it that way

The broadcastable original is the receiver's fallback. If the sender walks away,
the receiver broadcasts the original and is paid anyway. That single property does
a great deal of work in BIP-78: it bounds the receiver's downside, it powers the
anti-probing reuse rule (which triggers on observing the original broadcast), and
it is why a receiver "without a full node can decide to create the payjoin
transaction and automatically broadcast the original transaction after a timeout
of 1 minute".

The cost is borne by the sender, and BIP-78 names it: "For a successful payjoin to
happen, the sender needs to sign two transactions double spending each other",
which "means that the security guarantee of the hardware wallet is decreased."

## What the inversion buys and what it costs

R-P6's stated upside is correct and is worth keeping: because the original is not
broadcastable, no orphaned funding output can appear on-chain. A Lightning funding
output published before commitment signatures exist would be funds locked in a
2-of-2 with no way to reclaim them, so this is a genuine safety property, not a
technicality.

The cost is that the contributor gets nothing in exchange for signing first. BIP-78's
receiver is being *paid*; whichever transaction confirms, they receive their money.
This library's contributor supplies an input, receives the same value back minus a
fee, and gains no channel balance. Their best case is to be left exactly where they
started, less 99 vbytes of fees. Their worst case is an exposed UTXO and a
counterparty who never completes.

BOLT 2 records the same exposure for interactive construction under "Liquidity
griefing":

> "When sending `tx_add_input`, senders have no guarantee that the remote node will
> complete the protocol in a timely manner. Malicious remote nodes could delay
> messages or stop responding, which can result in a partially created transaction
> that cannot be broadcast by the honest node."

Its recommended answer is the same as the anti-probing answer: keep UTXOs unlocked
and reuse them across concurrent sessions, accepting conflicts between honest
sessions because "on-chain funding attempts are relatively infrequent operations"
and "failed attempts can simply be retried at no cost".

## The consequence that is not about cryptography

A contributor with no upside and a real downside is a contributor with no reason to
participate. That is pivot trigger T2 — "the volunteer contributor model fails on
economics rather than cryptography" — reached from the protocol's structure rather
than from a fee measurement. It should be evaluated on that basis, alongside T2's
response of re-modelling the role as paid or LSP-operated.

Note that this reasoning is inference from the protocol's incentives, not a
measurement. No contributor behaviour was observed or surveyed.

---

# The deterministic fee residue is what BOLT 2 specifies for dual funding

## Summary

R-P1 describes the contributor's change as `input - 99 x feerate`, giving an exact
arithmetic relationship an observer can invert. That apportionment rule is not a
mistake unique to this library. It is what BOLT 2 requires of every peer in an
interactively constructed transaction. The consequence runs both ways: the flaw is
shared with dual-funded channel opens, and the proposed fix makes the construction
non-conformant anywhere interactive-tx applies.

## Evidence

BOLT 2, "Fee Responsibility":

> "The *initiator* is responsible for paying the fees for the following fields, to
> be referred to as the `common fields`.
>
>   - version
>   - segwit marker + flag
>   - input count
>   - output count
>   - locktime
>
> The rest of the transaction bytes' fees are the responsibility of the peer who
> contributed that input or output via `tx_add_input` or `tx_add_output`, at the
> agreed upon `feerate`."

A peer contributing one P2WPKH input and one P2WPKH change output is responsible
for `(41 + 31) x 4 + 108 = 396` weight units, which is 99 vbytes, and pays
`99 x feerate`. That is the same figure the library uses, arrived at
independently, and it produces the same invertible residue.

Experiment 01 confirms the arithmetic empirically for the library: across 59
constructed transactions spanning fee rates 1-300 sat/vB, the contributor's input
minus their change equalled exactly `99 x feerate` in 59 of 59 cases, and dividing
the difference by 99 recovered the fee rate exactly every time.

## What follows

**The weakness is not confined to this project.** A BOLT 2 dual-funded open in
which one peer contributes an input and change but no channel value leaves the
same residue, because the spec's apportionment rule is deterministic in the
contributed weight and the agreed feerate. Anyone evaluating collaborative funding
privacy should treat this as a property of the interactive-tx fee rule, not of one
implementation.

**Fixing it diverges from the spec.** Randomising the fee split, which is what
build item B2 proposes, breaks the "responsible for the bytes you contributed"
rule. That is harmless where this library operates today — v1 channel
establishment negotiates no fee apportionment at all, so there is nothing to
conform to. It is not harmless if the construction is later applied to splices or
to dual funding, both of which run interactive-tx. The project's own §7 raises
splicing as possibly the better venue; this is a constraint on that move that
should be recorded before it is made.

**It matters less than it appears.** the central result finds that removing the residue
entirely moves subset-sum partitioning by 0.01 of a percentage point, because the
near-equality between the contributor's input and their change survives any fee
scheme. B2 is worth doing and does not come close to sufficing.

## Two neighbouring requirements in the same protocol

While confirming the fee rule, two other BOLT 2 requirements were found that bear
directly on R-P2 and R-P3, and they are recorded here because they establish what
the correct construction looks like in normative text.

**Ordering.** "Inputs in the constructed transaction MUST be sorted by
`serial_id`", and correspondingly for outputs, where `serial_id` "is a randomly
chosen number which uniquely identifies this input". Randomised ordering keyed on
a random identifier is the specified mechanism, and it is what build item B3
should implement.

**Sequence.** The sending node "MUST set `sequence` to be less than or equal to
4294967293 (`0xFFFFFFFD`)", and the receiving node "MUST fail the negotiation if
`sequence` is set to `0xFFFFFFFE` or `0xFFFFFFFF`". The rationale is explicit
about why:

> "`sequence` is the sequence number of this input: it must signal replaceability,
> and the same value should be used across implementations to avoid on-chain
> fingerprinting."

The library sets `Sequence::MAX`, which is `0xFFFFFFFF` — the value the protocol
names as a negotiation failure. Bitcoin Core independently arrives at the same
`0xfffffffd` for RBF-signalling transactions
(`src/util/rbf.h:12`, `MAX_BIP125_RBF_SEQUENCE{0xfffffffd}`). Two independent
sources converge on one value, and the stated reason is fingerprint avoidance.

---

# LDK offers a checked manual-broadcast API, and the integration does not use it

## Summary

The LDK adapter commits the funding outpoint through
`ChannelManager::unsafe_manual_funding_transaction_generated`. LDK also exposes
`ChannelManager::funding_transaction_generated_manual_broadcast`, which has the
same deferred-broadcast semantics but performs LDK's full validation. The
integration is correct as written, and would be safer using the checked call.

## Evidence

In rust-lightning at tag `v0.2.2`, commit
`0695da995fbe2810e11fd8824dd43eece994d111`, `lightning/src/ln/channelmanager.rs`
declares three public entry points for funding a channel:
`funding_transaction_generated` (line 6206),
`unsafe_manual_funding_transaction_generated` (line 6241), and
`funding_transaction_generated_manual_broadcast` (line 6277).

The doc comment on the `unsafe_` variant opens:

> "**Unsafe**: This method does not validate the spent output. It is the caller's
> responsibility to ensure the spent outputs are SegWit, as well as making sure
> the funding transaction has a final absolute locktime, i.e., its locktime is
> lower than the next block height.
>
> For a safer method, please refer to [`ChannelManager::funding_transaction_generated`]."

The doc comment on the manual-broadcast variant:

> "This method executes the same checks as
> [`ChannelManager::funding_transaction_generated`], but it does not
> automatically broadcast the funding transaction.
>
> Call this in response to a [`Event::FundingGenerationReady`] event, only in a
> context where you want to manually control the broadcast of the funding
> transaction."

"Only in a context where you want to manually control the broadcast" is a
description of this project's requirement.

In the repository under audit, `src/lightning/ldk.rs:231-247` calls the `unsafe_`
variant, and the callback trait at `src/lightning/ldk.rs:240-248` is declared in
terms of it. The adapter carries only an `OutPoint`
(`LdkManualFunding::funding_txo`), never the full `Transaction`, so adopting the
checked call is an API change rather than a substitution.

## What the checked path validates

From `batch_funding_transaction_generated_intern` in the same file, the
`CheckedManualBroadcast` arm enforces:

- every input carries a witness — "Funding transaction must be fully signed and
  spend Segwit outputs";
- at most 2^16 outputs;
- an output matching the channel's expected `to_p2wsh()` script *and* the
  negotiated value, with a distinct error if several outputs match;
- absolute-locktime finality, but only conditionally:

```rust
if !funding_transaction.input.iter().all(|input| input.sequence == Sequence::MAX) &&
    funding_transaction.lock_time.is_block_height() &&
        funding_transaction.lock_time.to_consensus_u32() > height + 1
```

## Two consequences worth carrying forward

**The checked path requires a fully signed transaction.** Because segwit txids do
not commit to witnesses, the outpoint is fixed as soon as the input and output
structure is fixed, before anyone signs. A construction that wants the checked
call must therefore have both parties' signatures in hand before it commits the
channel, which is an ordering decision, not a free substitution. It interacts with
the signing-order question in R-P6.

**Adopting RBF-signalling sequences switches the locktime check on.** The
condition is guarded by `!all(sequence == Sequence::MAX)`. The library currently
sets every sequence to `Sequence::MAX`, so the locktime check is skipped today.
Moving to `nSequence = 0xfffffffd` — which R-P3 recommends and BOLT 2 requires for
interactive construction — activates it, and the funding transaction's locktime
must then be no greater than LDK's `best_block` height plus one.

Bitcoin Core's anti-fee-sniping sets `nLockTime` to the current tip height, and
one time in ten to a uniformly chosen height up to 100 blocks earlier, so both
branches satisfy the constraint *provided the library's chain view is not ahead of
LDK's*. The LDK comment names exactly this hazard: "the modules constituting our
Lightning node might not have perfect sync about their blockchain views. Thus, if
the wallet module is ahead of LDK, only allow one more block of headroom." A
funding transaction built against a tip LDK has not yet seen will be rejected.

## LDK's own guidance on anti-fee-sniping

The same doc comment on `unsafe_manual_funding_transaction_generated` says:

> "Note to keep the miner incentives aligned in moving the blockchain forward, we
> recommend the wallet software generating the funding transaction to apply anti
> fee sniping as implemented by Bitcoin Core wallet."

The library does not. This is R-P3, and LDK asks for the fix in the documentation
of the very function the integration calls.
