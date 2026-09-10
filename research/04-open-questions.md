# Open questions

Eight questions the project set out to settle, each with a recorded result —
including the negative ones and the ones that could not be settled.

A ninth question asked whether a proposal document sent to a third party contained
claims inconsistent with this record. It does not publish here: no such document
exists, so the corrections in
[01-claims-corrections.md](./01-claims-corrections.md) are the whole of the
problem and correcting this repository corrects it in full.

---


# U1 · Does the construction survive the unnecessary-input heuristic once randomised padding and shuffling are applied?

**Result: no, and the question turns out to be aimed at the wrong heuristic.**

## The heuristic does not apply as published

Ghesmati et al. define UIH1 and UIH2 only over transactions "with more than one
input and exactly two outputs which is the common template of recent PayJoin
transactions". They say so again in the discussion: "In its current state, PayJoin
is only described for transactions with more than one input and two outputs."

A collaborative Lightning channel open has **three** outputs — the funding output
and two changes. It is outside the published scope of the heuristic and outside
the anonymity set the paper measures, which it puts at "15.4% compared to the
total number of transactions … for September 2020".

So the literal answer to U1 is that the heuristic as published does not classify
these transactions at all. That is not reassurance. It means the transaction is
not hiding in the population the heuristic was written about.

## Applied anyway, the answer is that it does not distinguish much

The harness applies all three published UIH2 variants to the two change outputs,
treating the funding output as the payment — the closest faithful generalisation.
Over 20,000 samples:

| Variant | Construction A | Construction B | Ghesmati base rate for ordinary 2-output txs |
|---|---|---|---|
| BlockSci UIH2 | 0.00% | 0.00% | 41.96% |
| BlockStream UIH2 | 68.33% | 68.33% | 41.77% |
| Gibson UIH2 | 100.00% | 100.00% | 27.18% |

Two things stand out. The remediations change nothing — the shares are identical
to two decimal places, because shuffling and fee jitter do not move min, max or
sum. And Gibson-UIH2 fires on 100% of these transactions against a 27% base rate,
so it is a strong distinguisher rather than a partitioning tool.

The base-rate column also cuts against the framing of the whole question. Roughly
42% of *ordinary* multi-input two-output transactions already trip UIH2. Tripping
UIH2 is weak evidence about anything. Ghesmati et al. say so: "there is a degree
of uncertainty over the definition of UIH2, as different coin selection algorithms
or input consolidation can violate this heuristic."

## The attack that matters is not UIH at all

The MVP frames U1 around the unnecessary-input heuristic. Two stronger attacks
were run against the same samples: subset-sum partitioning, which the MVP also
names, and a near-equality attack that uses no fee arithmetic.

| Attack | Construction A | Construction B |
|---|---|---|
| Subset-sum | 99.86% | 99.85% |
| Near-equality | 99.75% | 99.76% |
| Near-equality, all outputs P2TR | — | 99.49% |

Chance is 50%. Randomised apportionment and shuffling — the "randomised padding
and shuffling" U1 asks about — move these by hundredths of a percentage point.

**U1 is answered: no.** The reason is in the central result, and it is structural. A contributor
who takes their input back as change leaves an input and an output within one
transaction fee of each other, while the initiator's pair differ by the whole
channel capacity. No fee scheme and no ordering changes that.

## Caveats

The generalisation of UIH to three outputs is the harness's choice and is not what
the paper specifies; a different generalisation would give different percentages.
The base rates in the third column are from a one-week-per-year sample of Bitcoin
blocks ending in September 2020 and are not directly comparable to synthetic
three-output transactions. Neither column is a measurement of real chain data.

---

# U2 · What proportion of realistic UTXO distributions can be made to resist partitioning, and at what fee cost?

**Result: none resist, and the answer does not depend on the distribution. The fee
cost is now measured against real coin values rather than assumed.**

## The proportion is zero, for a reason no distribution changes

Write `V` and `C` for the contributor's input and change, `I` and `D` for the
initiator's, `K` for capacity, `F` for total fee. The contributor pays only fees:

    |V - C| = contributor's fee share  <=  F
    |I - D| = K + initiator's fee share  >=  K

Neither `V` nor `I` appears. Whenever `K > F` — every channel worth opening — the
contributor's pair is the closest in the transaction by orders of magnitude, and an
observer sorts pairs by absolute difference and takes the smallest.

Checked empirically over UTXO values spanning three and a half orders of magnitude
at fee rates 1–60 sat/vB and capacities 0.5M–5M sat: the near-equality attack
succeeded on 99.75% of 20,000 samples, stable across three seeds, with no region
of the sampled space resisting.

**Why no fitted distribution was substituted.** The research brief asks for a
defensible source for a realistic UTXO distribution. Real distributions were
obtained and are reported below — but they cannot change this result, because the
attack does not read the distribution. Reporting them as if they made the finding
more precise would be dressing.

## Real value distributions

Two independent samples, both from mainnet, 31 blocks spanning 29.8 days ending at
height 966,300.

**(a) FLOW** — every spendable (non-OP_RETURN) output created in those blocks,
parsed from raw consensus bytes. n = 229,347.

**(b) SPENT** — the prevout values of inputs actually spent, from a subsample of
8,970 transactions across 30 of the blocks. n = 22,801. This is the better proxy
for a contributor's coin, because a contributor offers a coin they are willing to
move.

| Quantile | FLOW (sat) | SPENT (sat) |
|---|---|---|
| p5 | 330 | 546 |
| p10 | 600 | 1,000 |
| p25 | 14,190 | 9,000 |
| p50 | **70,910** | **147,795** |
| p75 | 464,662 | 1,546,560 |
| p90 | 1,968,371 | 18,066,300 |
| p95 | 10,911,915 | 73,418,852 |

The two agree in order of magnitude through the middle and diverge in the upper
tail, where SPENT is dominated by a smaller number of large coins. Note the SPENT
p5 sitting exactly on 546 sat — the dust threshold — which is a real feature of
mainnet, not an artefact.

## Which coins can participate

The library rejects a contributor whose UTXO cannot pay `99 × feerate` and still
leave 546 sat of non-dust change (`src/psbt/builder.rs:11`, `:180-190`). Measured
against real values:

| Fee rate | Floor (sat) | FLOW eligible | SPENT eligible |
|---|---|---|---|
| 1 sat/vB | 645 | 89.73% | 94.05% |
| 2 | 744 | 89.41% | 93.86% |
| 5 | 1,041 | 89.02% | **78.18%** |
| 10 | 1,536 | 88.46% | 77.58% |
| 50 | 5,496 | 85.82% | 76.14% |
| 100 | 10,446 | 82.12% | 74.43% |
| 200 | 20,346 | 66.11% | 71.74% |
| 400 | 40,146 | 58.10% | 66.79% |

Eligibility is not the binding constraint. Even at 400 sat/vB, two thirds of real
spent coins clear the floor. The sharp SPENT drop between 2 and 5 sat/vB is the
mass of coins sitting at exactly 546 and 1,000 sat falling below it.

## The fee cost, which is the constraint that bites

| Fee rate | Cost (sat) | % of FLOW median coin | % of SPENT median coin |
|---|---|---|---|
| 1 sat/vB | 99 | 0.140% | 0.067% |
| 10 | 990 | 1.396% | 0.670% |
| 50 | 4,950 | 6.981% | 3.349% |
| 100 | 9,900 | **13.961%** | **6.698%** |
| 200 | 19,800 | 27.923% | 13.397% |
| 400 | 39,600 | 55.845% | 26.794% |

At 1 sat/vB contributing costs a rounding error. At 100 sat/vB it costs between 7%
and 14% of a median coin; at 400 sat/vB, between 27% and 56%.

**This is pivot trigger T2 with numbers attached.** T2 anticipates that "V1 passes
only at fee costs the contributor will not accept". The contributor's return for
this expenditure is nothing: they receive their own money back, gain no channel
balance, and take on the griefing exposure R-P6 describes. A volunteer model that
asks for double-digit percentages of a coin during a fee spike, for no benefit, is
not an economics problem waiting to be discovered — it is one that can be read off
the table.

Note that T2's condition is not reached the way it was written, because V1 does not
pass at *any* fee cost. The economics are a second, independent obstacle rather
than the binding one.

## Caveats

The FLOW sample is what was created in one month, not the standing UTXO set — the
flow, not the stock. The SPENT sample is drawn from the first 300 transactions of
each block, which is a position-based subsample rather than a random one, and
block ordering is not random with respect to fee rate. Both are one month ending
2026-09; another period would give different numbers, particularly given this
period's heavy OP_RETURN traffic. The cost table divides by a median, which is a
crude summary of a heavy-tailed distribution.

The earlier version of this file relied on Delgado-Segura et al.'s 2017 snapshot,
which was the only distribution available at the time. That source is now
superseded by direct measurement for the value questions and is retained only for
its definition of an unprofitable output: "the output of a transaction that holds
less value than the fee necessary to be spent".

---

# U3 · Is the transaction structurally indistinguishable from ordinary multi-party transactions on locktime, sequence, and output ordering?

**Result: no. Settled against real mainnet data. V5 fails.**

An earlier version of this file answered against specifications and wallet source
because no chain corpus was available. That corpus now exists — 150,769
non-coinbase transactions from 31 blocks spanning 29.8 days — and it changes both
the answer's confidence and, on one field, its direction.

## The answer in one table

| Filter | Count | Share |
|---|---|---|
| all non-coinbase transactions | 150,769 | 100% |
| ...with any P2WSH output | 3,362 | 2.23% |
| ...2-in 3-out, any metadata | 968 | 0.64% |
| ...+ nLockTime 0 + all sequences final | 805 | 0.53% |
| ...+ exactly one P2WSH and two P2WPKH outputs | **2** | **0.0013%** |

The library's exact structural signature appears in about **1 transaction in
75,000**. V5's pass condition — "Distribution is consistent with ordinary
transactions; no field identifies the library" — fails, and not narrowly.

## The three fields the question asks about

**nSequence.** All inputs `0xffffffff`. Real traffic: `0xfffffffd` on 54.48% of
inputs, `0xffffffff` on 41.05%. So the value is a minority but not an anomaly —
this is weaker evidence than the specifications alone suggested, and it is worth
being accurate about. BOLT 2 still forbids it for interactive construction and
Bitcoin Core's wallet still never emits it.

**nLockTime.** Zero, in 100% of the library's output. Real traffic: **95.34%**
zero. The library is in the overwhelming majority here, and the earlier
recommendation to adopt anti-fee-sniping was wrong — it would move the transaction
into a 4.46% minority. See R-P3 for the corrected recommendation and the tension
behind it.

**Output ordering.** Fixed, funding output always at index 0. This turns out not to
matter for identification at all: the signature above is an order-independent
multiset of script types plus input and output counts. **Build item B3 shuffles the
order and changes none of the numbers in that table.** Ordering is a library
fingerprint — it says "built by this code" — but it is not what makes the
transaction rare.

## The field the question does not ask about, which dominates

Output count. 84.41% of mainnet transactions have exactly two outputs; only 2.84%
have three. Input count compounds it: 90.91% have one input, 5.12% have two. Before
any script type or metadata is considered, a 2-in 3-out transaction is already in
the 0.64%.

This corroborates Kappos et al.'s calibration of their channel-open property
heuristic, where "99.91% had at most two outputs" and "99.91% had a single P2WSH
output address". Measured directly a month ago rather than 2018-2020, the same
picture holds.

The consequence is that the library cannot reach indistinguishability by adjusting
fields. Three outputs is what the construction requires — two parties' inputs, a
funding output, two changes — and three outputs is the anomaly.

## Taproot changes it by 3.5×, which is not enough

A simple taproot channel would produce 2-in 3-out with all three outputs P2TR. That
shape occurs **7 times in 150,769**, about 1 in 21,500, against 1 in 75,000 for the
P2WSH shape. A real improvement, and still rare enough for an analyst to enumerate
a month of candidates on a laptop.

It also does nothing for the separate near-equality attack, which the central result measures at
99.49% against taproot outputs.

## What would actually help

Nothing in the current build scope. The two changes that would move these numbers
are structural:

- **Fewer outputs.** A construction with two outputs rather than three lands in the
  84% majority. That means the contributor does not take separate change — which is
  the absorbed construction the central result finds also defeats subset-sum partitioning. The
  two findings point at the same fix from different directions.
- **More parties.** A batched multi-party open has more outputs, not fewer, so it
  goes the other way on this axis while going the right way on partitioning. The
  trade was not measured.

## Caveats

The sample is one month ending at height 966,300; percentages would differ in
another period. This period carries heavy OP_RETURN traffic — 31.10% of all outputs
— which inflates the denominator with non-payment transactions and makes the
library's share smaller than a payment-only denominator would. With only two
matching transactions the 1-in-75,000 figure carries roughly a factor-of-three
Poisson interval; the finding does not depend on the precision. Full threats to
validity in the experiment's `analysis.md`.

---

# U4 · Can contributor UTXO probing be bounded without breaking legitimate use?

**Result: bounded, yes. Bounded to the degree V6 currently demands, no — and the
two sources that have thought hardest about this deliberately decline to make it
that tight.**

The mechanism, the prior art and the confirmation that Optech flagged this class
are all in R-P5. This file records what follows for the verification target.

## The mitigation exists and is well attested

Both BIP-78 and BOLT 2 converge on the same answer — reuse the exposed UTXO rather
than exposing a fresh one — and Optech #131 records a third family of approaches
in which the initiator posts something they lose if they abandon the session
(PoDLEs, `SIGHASH_SINGLE|SIGHASH_ANYONECANPAY` half-signed transactions, or Lloyd
Fournier's signed-but-unbroadcast good-faith transaction).

Reuse is cheap and requires no new cryptography. BOLT 2 argues the cost is
acceptable because "on-chain funding attempts are relatively infrequent
operations" and "failed attempts can simply be retried at no cost".

## Why V6 as written cannot be met

V6 requires: "Contributor exposes no more than one distinct UTXO to that peer
regardless of N."

BIP-78 refuses to promise this, and explains why in a sentence that is easy to
read past: "While the exposed UTXO will be reused in priority to not leak other
UTXOs, there is no strong guarantee about it. **This prevents the attacker from
detecting with certainty the next payjoin of the merchant to another peer.**"

A strict one-UTXO-per-peer rule is itself an oracle. An attacker who probes once,
learns UTXO `u`, and later sees `u` spent, knows the contributor's next
presentation to them will be something new — and, worse, that any transaction
spending `u` involved this contributor. Determinism in the defence creates
linkability elsewhere.

There is a second, more mundane failure: the reserved UTXO will eventually be
spent for an unrelated reason, and the contributor must then present a different
one. Over a long enough horizon N, exposure exceeds one UTXO regardless of policy.

**V6 should be restated.** Something closer to: *over N aborted sessions from one
peer within a window in which the reserved UTXO remains unspent, the contributor
exposes exactly one UTXO; when it is spent, replacement is drawn in a way that
does not signal the spend.* That is testable and achievable. The current wording
is neither.

## The BIP-78 trigger does not port

BIP-78's reuse rule fires when the receiver sees the original transaction
broadcast or double-spent. There is no broadcastable original in a Lightning
funding flow (R-P6), so the contributor must key reuse on abort and timeout. This
is a design difference that has to be worked out rather than inherited.

## "Without breaking legitimate use"

Not settled. BOLT 2 accepts that reuse "will also create conflicts between
concurrent sessions with honest nodes" and judges the trade acceptable for
infrequent funding operations. Whether that judgement holds for a contributor role
intended to run continuously on a node accepting channels from strangers was not
measured here. If it does not, that is pivot trigger T3, whose response is to
restrict the contributor role to peers with an existing relationship.

BIP-78's own framing supports treating this as serious: it says probing "are only
a problem for automated payment systems … End-user wallets with payjoin
capabilities are not affected". A node accepting channels from strangers is the
automated case.

---

# U5 · Does the peer-contributor path remain sound for unannounced channels, or does something else identify the parties?

**Result: something else identifies the parties, it has been measured, and it is
not something Layer 2 touches.**

## The measurement

Kappos et al. attacked exactly this question on real chain data. Their tracing
heuristic follows the peeling chain that node operators create when they fund each
channel from the previous one's change, and they report:

> "Out of the 27,183 transactions we identified as representing the opening of
> private channels, we were able to identify both participants in 2,035 (7.5%),
> one participant in 21,557 (79.3%), and no participants in 3,591 (13.2%)."

At least one participant was identified in **86.8%** of private-channel opens.
Not by gossip — these channels published none — but by coin flow.

The pattern they exploit is stated plainly:

> "it was common for users opening channels to do so in a 'peeling chain' pattern.
> This meant they would (1) use the change in a channel opening transaction to
> continue to create channels and (2) use the outputs in a channel closing
> transaction to open new channels. Furthermore, they would often (3) co-spend
> change with closing outputs."

## Why this reorders the project's priorities

The tracing heuristic reads **transitions between transactions**, not the internal
structure of any one of them. Layer 2 — funding transaction construction, the
project's declared core — does not touch it. A collaboratively constructed funding
transaction whose change is then spent into the next channel open is caught by
this heuristic exactly as a single-funder one is.

What does touch it is **Layer 1**: coin selection that never funds a channel from
a previous open's change, and never co-spends change with closing outputs. That is
build item B6, currently sitting sixth in an ordered list behind four fixes to
Layer 2.

The project definition already identifies this as "a second, compounding pattern"
in §2.3 and says it "must be addressed separately". The measurement says it is not
secondary. On the only published numbers for private channels, the peeling chain
identified a participant 86.8% of the time, while the central result finds Layer 2's
contribution to be nil in its current and remediated forms.

## The answer to the question as posed

The peer-contributor path is not made unsound *by the peer relationship* for
unannounced channels — the MVP's reasoning there is correct, in that no
announcement names the two parties. It is made unsound by the fact that the
funding transaction sits in a chain of the operator's own transactions, and that
chain has been shown to identify the operator without gossip.

So the peer-as-contributor choice is not the weak point. The weak point is
upstream and downstream of the transaction the library builds.

## Caveats

Kappos et al.'s figures cover transactions between 12 January 2018 and 7 September
2020. They are not current, and channel-opening practice may have changed; LSP-run
mobile wallets in particular may not peel the way the operators in that dataset
did. No attempt was made here to reproduce the heuristic or to update the figures.
Their private-channel population is itself an inference — they call the property
heuristic "just an upper bound, since there are other reasons to use 2-of-2
multisigs in this way that have nothing to do with Lightning" — so the denominator
carries error the paper does not quantify.

---

# U6 · Can the round-trip complete inside the unfunded-channel window?

**Result: yes, with about four orders of magnitude to spare. The constant is
verified, the "approximately one hour" reading is correct, and the window is the
integrator's choice rather than a protocol guarantee.**

## The constant

In rust-lightning at tag `v0.2.2`, commit
`0695da995fbe2810e11fd8824dd43eece994d111`, `lightning/src/ln/channel.rs:1367`:

```rust
/// The number of ticks that may elapse while we're waiting for an unfunded outbound/inbound channel
/// to be promoted to a [`FundedChannel`] since the unfunded channel was created. An unfunded channel
/// exceeding this age limit will be force-closed and purged from memory.
pub(crate) const UNFUNDED_CHANNEL_AGE_LIMIT_TICKS: usize = 60;
```

Enforced at `channel.rs:2290-2293`:

```rust
pub fn should_expire_unfunded_channel(&mut self) -> bool {
    self.unfunded_channel_age_ticks += 1;
    self.unfunded_channel_age_ticks >= UNFUNDED_CHANNEL_AGE_LIMIT_TICKS
}
```

The counter increments before the comparison, so expiry happens on the 60th tick,
not after 60 further ticks.

## Why it is about an hour, and why that is softer than it sounds

A tick is one call to `ChannelManager::timer_tick_occurred`. LDK does not schedule
it; the integrator does. The guidance, at `channelmanager.rs:2572`, is:

> "[`timer_tick_occurred`] roughly once per minute, though it doesn't have to be
> perfect."

60 ticks at roughly one minute is roughly one hour, so the MVP document's
"approximately one hour" is right. But it follows from a *recommended cadence*,
not from a constant. An integrator ticking every 30 seconds has a 30-minute
window. This should be documented as "60 ticks, which is about an hour at LDK's
recommended cadence and is whatever the integrator's cadence makes it", not as an
hour.

## The round-trip, measured

Experiment 06 drives all four protocol steps through the library's public API —
`prepare_original`, `propose_privacy_input`, `validate_privacy_input_proposal`,
`finalize_validated_proposal` — across 160,000 sessions on 8 threads sharing 2
CPUs, deliberately oversubscribed 4:1 so the figures include scheduling
contention.

| | |
|---|---|
| p50 | 3.3 µs |
| p99 | 4.7 µs |
| p99.9 | 10,019 µs |
| max | 123,997 µs |
| throughput | 139,465 sessions/s |

Worst observed compute is 0.124 s, which is **0.0034%** of a 3,600 s window — a
headroom factor of about 29,000×. The gap between p99 and p99.9 is the thread
being descheduled, not protocol cost.

Adding a round-trip time on top of the worst compute:

| RTT | total | share of window |
|---|---|---|
| 0.5 s | 0.6 s | 0.017% |
| 10 s | 10.1 s | 0.281% |
| 60 s | 60.1 s | 1.670% |
| 300 s | 300.1 s | 8.337% |

A contributor taking five minutes to answer still uses 8% of the budget. **V7
passes.** Timing is not a constraint on the MVP path, and it was never likely to
be: two peers mid-handshake on an open BOLT 8 connection have an hour to exchange
two messages.

One design consequence worth carrying: because compute is free relative to the
budget, no privacy fix can be rejected on performance grounds. Shuffling, careful
weight calculation, privacy-aware coin selection and the bond-and-verify schemes
R-P5 discusses all cost far less than the margin available.

## What was not settled

Only compute was timed. The RTT table above is arithmetic on top of that
measurement, not an observation: no transport was exercised, nothing was
serialised over a real link, and no peer was slow or hostile. `MockDirectory` is
an in-memory map, so the sessions never crossed a process boundary.

Both coordinators ran in one thread per session, so the measured figure is the sum
of the two parties' compute — the right total, but not the right per-party
latency.

The third-party path is untouched. U8's timing question is about finding and
reaching a stranger over an undesigned rendezvous, and nothing here bears on it.

A contributor requiring human approval was not modelled, and neither was a node
doing Lightning work concurrently.

Finally, the window itself is softer than the headline suggests. 60 ticks is an
hour only at LDK's *recommended* cadence. An integrator ticking every 10 seconds
has a 10-minute window, and the RTT table should then be read against 600 s. Even
there the 5-minute-contributor row is 50% of budget rather than 8%.

---

# U7 · Do Core Lightning and LND expose a manual-funding hook equivalent to LDK's?

**Milestone 2, researched now. Result: yes, both do, and LND's is arguably richer
than LDK's.**

The project definition promises "an assessment of the equivalent hooks in Core
Lightning and LND, with an honest statement of what is and is not currently
reachable in each". This is that assessment at the API level. Neither was
exercised.

## Core Lightning

`fundchannel_start` and `fundchannel_complete`, present in the RPC schemas at tag
`v26.06`. From `fundchannel_start`'s own description:

> "`fundchannel_start` is a lower level RPC command. It allows a user to initiate
> channel establishment with a connected peer.
>
> Note that the funding transaction MUST NOT be broadcast until after channel
> establishment has been successfully completed by running `fundchannel_complete`,
> as the commitment transactions for this channel are not secured until the
> complete command succeeds. Broadcasting transaction before that can lead to
> unrecoverable loss of funds."

`fundchannel_start` takes `amount, announce, channel_type, close_to, feerate, id,
mindepth, push_msat, reserve` and returns the funding script.
`fundchannel_complete` takes `id, psbt, withhold` — it accepts the constructed PSBT
directly. `fundchannel_cancel` aborts.

The mapping onto the library's flow is one to one: `fundchannel_start` stands in
for `Event::FundingGenerationReady`, `fundchannel_complete` for
`unsafe_manual_funding_transaction_generated`, and the same
broadcast-after-completion rule applies. Note also that `announce` is a parameter
of `fundchannel_start`, so the Profile A/B choice is made at the same call.

Channel establishment v2 has its own set — `openchannel_init`,
`openchannel_update`, `openchannel_signed`, `openchannel_abort`, `openchannel_bump`
— which is the dual-funding path rather than the manual-funding one.

## LND

The PSBT funding shim, defined in `lnrpc/lightning.proto` at tag `v0.21.0-beta`,
driven through the `FundingStateStep` RPC. `OpenChannel` takes a `FundingShim`,
one variant of which is `PsbtShim`:

```protobuf
message PsbtShim {
    bytes pending_chan_id = 1;
    // An optional base PSBT the new channel output will be added to.
    bytes base_psbt = 2;
    // ... prevents this particular channel from broadcasting the transaction
    // after the negotiation with the remote peer.
    bool no_publish = 3;
}
```

Then `FundingPsbtVerify` and `FundingPsbtFinalize` step the state machine.

Two things make this a better fit than LDK's, not merely an equal one.

**`base_psbt`.** LND will add the channel output to a PSBT the caller supplies.
That is the collaborative construction shape directly, rather than the library
having to construct around an outpoint LDK hands back.

**`no_publish` plus `skip_finalize`.** `no_publish` defers broadcast to the caller.
`FundingPsbtVerify.skip_finalize` then lets the caller take full responsibility,
with a warning that states the same ordering constraint the project has identified
for LDK:

> "IT IS ABSOLUTELY IMPERATIVE that the TXID of the transaction that is eventually
> published does have the _same TXID_ as the verified PSBT. That means no inputs or
> outputs can change, only signatures can be added. If the TXID changes between
> this call and the publish step then the channel will never be created and the
> funds will be in limbo."

## What this changes

The project definition presents LDK's manual funding path as the integration
route, and the README's architecture table describes node integration as
"LDK-compatible, FFI interface for CLN/LND". On the evidence, CLN and LND expose
first-class RPC hooks for exactly this and need no FFI at all — an out-of-process
client speaking their existing RPC is sufficient. The framing of LDK as the
uniquely capable host is not supported.

That does not make the work smaller. Each hook has its own state machine, its own
abort semantics and its own failure modes, none of which were exercised here.

## Not verified

No CLN or LND node was run. Every statement above is read from the RPC schema at
`ElementsProject/lightning` tag `v26.06` and the protobuf definitions at
`lightningnetwork/lnd` tag `v0.21.0-beta`, both accessed 2026-09-10. Whether the
flows behave as documented, and whether the collaborative round-trip fits inside
each implementation's own unfunded-channel timeout, is untested.

---

# U8 · Is a third-party contributor pool viable within the same timing window?

**Milestone 2. Scoped, not settled — as the brief directs.**

## What the timing budget actually is

From U6: LDK force-closes an unfunded channel after
`UNFUNDED_CHANNEL_AGE_LIMIT_TICKS = 60` ticks, roughly an hour at the recommended
one-tick-per-minute cadence, but the cadence is the integrator's choice.

The MVP's peer-contributor path spends almost none of that budget: the two parties
are already connected over BOLT 8 and mid-handshake. A third-party pool spends the
budget on things the MVP path does not have — finding a willing contributor,
reaching them over a transport that does not link them to the initiator, and
surviving their non-response.

## The questions that have to be answered, none of which were

1. **Rendezvous.** BIP-78 assumes an HTTP endpoint the sender already knows from a
   BIP21 URI. There is no analogue here: the initiator has no prior relationship
   with the contributor and no address for them. Something must introduce them,
   and that something sees both parties.
2. **What the introducer learns.** A directory that pairs initiators with
   contributors observes who is opening channels and when, which is a large part
   of what the project is trying to conceal. The project definition already puts a
   production relay and OHTTP out of scope, so the threat model for the thing that
   replaces them is undefined.
3. **Non-response budget.** A contributor who does not answer costs a round-trip
   and a retry. How many retries fit in the window depends on the transport, and
   Tor — which Layer 4 composes — has latency and failure characteristics that
   were not measured.
4. **Whether the pool changes the privacy result at all.** This is the question
   that should be asked first. the central result finds the two-party construction partitionable
   because the contributor's input returns to them as change. A third-party
   contributor has the same property. Sourcing the input from a stranger changes
   *who* the observer identifies, not *whether* the transaction can be partitioned.

Point 4 is the one that matters for sequencing. The MVP document treats
third-party sourcing as the Milestone 2 upgrade that makes Profile B work, and §7
raises the possibility it becomes necessary for Profile A too. Neither is reached
if the construction is partitionable regardless of who contributes.

## What would settle it

Answering 4 first, on paper, using the same harness: generate the construction with
a third-party contributor and run the near-equality and subset-sum attacks. If the
result matches construction B, the pool question is moot until the construction
changes, and questions 1 to 3 need not be answered yet. That experiment was not run
here; it is small and should come before any pool design work.

**Recorded as unverified.** No timing measurement, no transport measurement, no
pool design evaluated.
