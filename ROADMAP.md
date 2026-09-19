# Roadmap — lightning-payjoin-kit

This roadmap is verification-led. Each phase states what must be **measured**
before the next one begins, and no phase that writes library code starts until the
construction it would implement has already passed the gate on paper and in the
harness.

That ordering is deliberate. The previous roadmap scheduled implementation first
and verification last, and the result was a working prototype built around a
construction that measurement later showed could not deliver its privacy property.
The harnesses in [`research/`](./research/) now exist, so a candidate construction
can be evaluated in days rather than discovered to be unsound after a milestone
has been spent on it.

The superseded schedule is kept at
[`docs/ROADMAP.md`](./docs/ROADMAP.md) for the record.

---

## Phase 0 — MVP verification · **complete**

Settle whether the original construction — the contributor supplies an input and
receives the same value back as change — resists partitioning.

**Result: it does not.** The finding is recorded in
[`research/02-the-central-result.md`](./research/02-the-central-result.md) and is
reproducible with one command.

| Target | Pass condition | Result |
|---|---|---|
| V1 adversarial partitioning | No better than chance (50%) | **99.85%** — fail |
| V2 LDK two-node regression | Two `ChannelManager`s reach a usable channel | Pass |
| V3 Bitcoin Core regtest | Transaction accepted and mined | Pass |
| V4 fee accuracy | Within 5% of target rate | **94.84%** of target — fail |
| V5 structural indistinguishability | Consistent with ordinary transactions | **2 of 150,769** — fail |
| V6 probing resistance | One UTXO exposed regardless of N aborts | Not testable as written |
| V7 round-trip timing | Inside the unfunded-channel window | Pass, ~4 orders of magnitude spare |
| V8 value conservation | Capacity exact, malicious proposals rejected | Covered by the test suite |

V1 was the gate. V1 failed, and pivot trigger T1 was invoked on the evidence.

Phase 0 also produced the reusable part: four harnesses, a mainnet structural
baseline of 150,769 transactions, and a source record. Those are the foundation
every later phase is measured against, and they are independent of which
construction the project ends up choosing.

---

## Phase 1 — Evaluate improved funding constructions · **current**

**This phase produces no library code.** It evaluates candidate constructions
against the existing harness and selects one, or establishes that none of them
clears the gate.

The candidates, in the order the evidence favours them:

**C1 · Contribution absorbed into the funding output.** The contributor genuinely
funds part of the channel rather than taking their value back as change. Removes
the third output, which is what makes the transaction both rare and partitionable.
Evaluated only as a sketch so far: defeats subset-sum outright (49.96%), but reaches
only 57.04% against near-equality, or 53.27% with taproot outputs. Needs a real
design — contribution sizing, fee apportionment, and the Lightning accounting for a
contributor who now holds channel balance.

**C2 · Batched multi-party opens.** Anonymity scales with participants. The BOLT
interactive-tx rationale names multi-party coinjoin opens as an explicit design
goal. Untested; only the two-party case has been examined.

**C3 · Funding from a real incoming payment.** Named alongside C1 in pivot trigger
T1. The contributed value arrives as a payment the operator was receiving anyway,
so no counterpart output exists to pair against.

**C4 · Applying the construction to splices rather than opens.** Splices run the
same interactive transaction protocol and occur far more often than opens. If a
construction works, the splice surface is larger and more recurrent.

### Exit gate

Phase 1 ends when one of these is true:

- **A candidate passes.** Sustained at or below **55%** against every attack in the
  harness — fee residue, subset-sum, and near-equality, with and without taproot
  outputs — across at least three seeds and the full fee-rate and capacity matrix.
  The 55% figure is chance plus a deliberate margin; a construction that only just
  clears 50% on one seed has not been shown to clear it at all.
- **No candidate passes**, and that is recorded as a second negative result with
  the same standard of evidence as the first.

### Also in this phase

- **Restate V6.** The current pass condition cannot be met by any design, so it
  measures nothing. `research/04-open-questions.md` proposes a replacement in terms
  of UTXOs exposed per N aborted sessions.
- **Resolve the BOLT 2 conformance question.** The deterministic fee residue is what
  BOLT 2 requires of every peer in an interactively constructed transaction. Any fix
  makes the construction non-conformant wherever interactive-tx applies. Whether
  that is acceptable, or whether it belongs in a spec discussion, is a decision this
  phase has to reach rather than inherit.
- **Settle the taproot question.** Simple taproot channels cannot be announced, so
  Layer 3 composes for Profile A and not for Profile B. If an unannounced taproot
  channel is already unidentifiable as a channel, the marginal value of this work
  for Profile A falls and the project's centre of gravity moves to Profile B.

---

## Phase 2 — Implement the selected construction · **conditional on Phase 1**

Begins only if Phase 1 produces a candidate that clears the gate. Scope is written
after the candidate is known, because C1 and C2 imply materially different work:
C1 changes the Lightning accounting, C2 changes the coordination model.

Carried into this phase regardless of candidate, because they are defects in the
builder rather than in the construction:

- Correct P2WSH weight estimation (V4 currently fails at 94.84% of target).
- Anti-fee-sniping and RBF-signalling decisions, informed by the measurement that
  `nSequence = 0xfffffffd` with `nLockTime = 0` matches 64.68% of mainnet traffic
  while anti-fee-sniping would move the transaction into a 4.46% minority.
- Contributor protections: per-peer rate limiting and a reuse-after-abort policy,
  against the restated V6.

**Exit gate:** V1 through V8 pass against the implemented construction, measured by
the same harnesses, not by inspection.

---

## Phase 3 — Privacy-aware coin selection · **independent of Phases 1 and 2**

This addresses the attack that actually deanonymises operators today, and it does
not depend on which construction Phase 1 selects.

Kappos et al. identified at least one participant in **86.8%** of private-channel
opens using coin flow alone, with no gossip data. Their heuristic follows the
peeling chain *between* transactions: operators fund each new channel from the
previous open's change. No construction inside a single transaction touches it.

Scope: never fund a channel from a prior open's change; avoid round-number change;
match script types with the other party.

**Exit gate:** a measurement against the peeling-chain heuristic showing the
selection policy breaks the linkage, held to the same standard as V1 — a harness a
third party can run, not an argument.

Phase 3 is cheaper than Phase 1 and addresses a larger measured harm. It can run in
parallel, and there is a reasonable case for running it first.

---

## Phase 4 — Integration assessment · **not started**

Establish honestly what is reachable in each node implementation.

- LDK: verified working through the manual funding path (V2, V3).
- Core Lightning and LND: whether an equivalent manual-funding hook exists is open
  question U7, researched but not settled by experiment.

**Exit gate:** for each implementation, a statement of what is reachable today with
evidence, including where the answer is "not reachable".

---

## Release gates

A crates.io release requires all of:

- A construction that has passed the Phase 1 gate and the Phase 2 exit gate.
- Every claim in the README and the research record carrying a current label.
- The adversarial harness runnable by a third party against the library's own
  output with one documented command.

Until then the crate is not published. A release under this name while the privacy
property is unproven would promise something the code does not do.

---

## Standing pivot triggers

Defined in advance so that a negative result reads as a finding rather than an
argument. T1 has already been invoked.

| | Condition | Response |
|---|---|---|
| T1 | V1 cannot pass with contributed value returned as change | **Invoked.** Move to constructions where value is absorbed — Phase 1 |
| T2 | A construction passes only at fee costs a contributor will not accept | Re-model as a paid or LSP-operated role |
| T3 | Probing resistance cannot be met without breaking legitimate use | Restrict the contributor role to peers with an existing relationship, and document it as a hard constraint |
| T4 | Peer coordination cannot be implemented over BOLT 8 custom messages without forking LDK | Reassess transport; does not affect the privacy result |
| T5 | Profile A's population does not want this | Re-target at LSP operators as the integrator |

---

## What this roadmap deliberately does not schedule

Mainnet deployment. A production relay or directory service. OHTTP transport. A
command-line tool. Core Lightning and LND plugins or FFI bindings. External
security audit. Wallet-grade coin selection beyond the privacy properties named in
Phase 3.

None of these are ruled out permanently. None of them is the constraint right now.
