# Layers 3, 4 and 5: what exists, at what version

The project composes rather than builds at these layers. This is what was actually
shipped, with release evidence. Every date is the publisher's own, taken from the
project's release metadata rather than from a secondary summary.

## Layer 3 — channel type and announcement

| Implementation | Version | Published | Simple taproot channels |
|---|---|---|---|
| LND | `v0.21.0-beta` | 2026-06-05 | Production (final) support added |
| Eclair | `v0.14.0` | 2026-05-21 | "the final version of … taproot channels" |

LND's release notes: "Added support for production (final) simple taproot channels
… using the finalized taproot channel scripts with feature bits 80/81." Taproot
channels are opt-in: `lncli openchannel --channel_type=taproot`.

**The constraint that matters more than the version.** The same LND entry says
taproot channels "must remain private until announced taproot channels are
supported", and the draft extension BOLT says the type "_cannot_ be announced on
the public network (gossip protocol changes are required)" and that a sending node
"MUST not set the `announce_channel` bit".

So Layer 3 is available to unannounced channels only. An announced routing node
cannot compose it today.

**And it does not do what was hoped for unannounced ones either.** Recasting every
output as P2TR removes an observer's ability to tell which output is the channel.
It does not remove their ability to tell which input and output belong to the same
party: the near-equality attack still succeeds 99.49% of the time (see
`02-the-central-result.md`). Concealing the channel and concealing the ownership
partition are different problems, and taproot solves only the first.

Measured against 150,769 mainnet transactions from a 29.8-day sample, it does not
solve the first outright either:

| Shape | Occurrences | Share |
|---|---|---|
| 2-in 3-out, one P2WSH and two P2WPKH outputs | 2 | about 1 in 75,000 |
| 2-in 3-out, all three outputs P2TR | 7 | about 1 in 21,500 |

Taproot multiplies the anonymity set by about three and a half. The binding
constraint is the output count, not the script type — 84.41% of mainnet
transactions have exactly two outputs and only 2.84% have three — and taproot
cannot change how many outputs a construction needs.

## Layer 4 — transport and broadcast

Not researched in this round. Tor for the peer connection and third-party
broadcast are both ordinary operational choices with no version dependency, and
nothing here measures either. Recorded as unexamined rather than as available.

One point does follow from the work above: the MVP path is two peers already
connected over BOLT 8, which is authenticated and encrypted. The library therefore
adds no transport of its own on that path and needs none. A third-party
contributor would need a rendezvous, and that is undesigned.

## Layer 5 — splicing

| Milestone | Evidence | Date |
|---|---|---|
| Merged into the BOLTs | lightning/bolts PR #1160, "Channel Splicing (feature 62/63)" | 2026-03-23 |
| Core Lightning, on by default | v26.04 release notes: "Splicing is now enabled by default!" | 2026-04-20 |
| Eclair | v0.14.0: "the final version of channel splicing" | 2026-05-21 |
| LDK | v0.2 changelog: "Splicing is now supported" | 2025-12-02 |
| LND | Not available. The only mention in v0.21.0 is "laying the groundwork for splice support" | — |

**A chronology worth noting.** LDK shipped splicing in December 2025, three months
before the BOLT merge in March 2026, and its changelog warns that the
implementation "may change feature signaling in a future version as testing
completes, breaking compatibility". Listing the BOLT merge first implies a
spec-then-implementation order that is not what happened.

**A constraint on using splicing as the venue.** Splices run BOLT 2's interactive
transaction construction, which specifies that each peer pays "the fees for the
bytes they contributed". Randomising the fee apportionment — the fix for the
deterministic residue described in `03-construction-risks.md` — diverges from that
rule. That is harmless over v1 channel establishment, which negotiates no
apportionment at all, and it is not harmless for splices. Anyone moving the
construction to splicing inherits this constraint.

## Node integration hooks

The earlier documentation described node integration as "LDK-compatible, FFI
interface for CLN/LND". Both Core Lightning and LND expose first-class RPC hooks
for manually-funded channels and need no FFI.

- **Core Lightning**: `fundchannel_start` returns the funding script,
  `fundchannel_complete` accepts the constructed PSBT, `fundchannel_cancel` aborts.
  `fundchannel_start` also takes `announce`, so the profile choice is made there.
- **LND**: `OpenChannel` with a `PsbtShim` — which accepts an optional `base_psbt`
  the channel output is added to, and a `no_publish` flag — then `FundingPsbtVerify`
  and `FundingPsbtFinalize` through the `FundingStateStep` RPC.

All three implementations impose the same ordering rule. Core Lightning: "the
funding transaction MUST NOT be broadcast until after channel establishment has
been successfully completed". LND: "IT IS ABSOLUTELY IMPERATIVE that the TXID of
the transaction that is eventually published does have the _same TXID_ as the
verified PSBT."

Neither hook was exercised. Both statements are read from the RPC schemas and
protobuf definitions at pinned tags.

---

## Taproot, announcement, and where Layer 3 actually helps


## Summary

Simple taproot channels cannot be publicly announced, by specification and in the
only implementation that has shipped them as production-ready. Layer 3 of the
project's model is therefore available only to Profile A — unannounced channels —
and unavailable to Profile B, where input attribution is the only thing left to
defend. This is the opposite of the arrangement the project's §7 anticipates.

## Evidence

**Specification.** The draft extension BOLT for simple taproot channels states:

> "It's important to note that given the early version of this channel type
> _cannot_ be announced on the public network (gossip protocol changes are
> required), the taproot channels type cannot be used as an interchangeable
> default channel type."

and, in the requirements for the sending node:

> "MUST not set the `announce_channel` bit."

Caveat on status: the file is titled "Extension BOLT XX" and carries no assigned
number. It is present in the lightning/bolts repository at commit
`152897261850d93c4f4597f39cf22d7d22d6ede6` but is not one of the numbered BOLTs.

**Implementation.** LND's release notes for v0.21.0-beta, published 2026-06-05,
under New Features:

> "Added support for production (final) simple taproot channels … Taproot channels
> must be requested explicitly with `lncli openchannel --channel_type=taproot`
> … and must remain private until announced taproot channels are supported."

Eclair v0.14.0, published 2026-05-21: "This release contains the final version of
channel splicing, taproot channels and zero-fee commitments." Whether Eclair
permits announcing them was not checked; the specification says it cannot be done.

## Why this matters to the project

The project's §7 raises the possibility that taproot channels reduce the marginal
value of Layer 2 for Profile A, shifting the centre of gravity to Profile B, "where
input attribution is the only thing left to defend". The evidence cuts across that
in two places.

**Taproot is not composable with Profile B at all.** An announced channel cannot be
a taproot channel today. So the profile that most needs a concealed funding output
is the one that cannot have one, and Layer 3 offers Profile B nothing.

**Taproot does not reduce the value of Layer 2 for Profile A either, but not for
the reason expected.** The harness in the central result recast every output as P2TR and re-ran
the near-equality attack. It succeeded 99.49% of the time against the fully
remediated construction. Taproot conceals which output is the channel; it does not
conceal which input and output belong to the same party. So the premise behind §7's
concern — that an unannounced taproot channel is already unidentifiable, making
input ambiguity marginal — does not hold in the form stated: the transaction
remains partitionable by owner even when it is not identifiable as a channel open.

This does not make Layer 2 valuable for Profile A. It makes both layers
insufficient. An observer who cannot tell that a transaction opened a channel can
still tell that two parties transacted and which coins belonged to whom, and that
is enough to continue clustering.

**And the premise is weaker than §7 assumes in a second way: an unannounced
taproot channel is not unidentifiable.** Measured over 150,769 mainnet
transactions from 31 blocks spanning 29.8 days, the shape a collaborative open
produces is vanishingly rare either way:

| Shape | Occurrences | Share |
|---|---|---|
| 2-in 3-out, one P2WSH and two P2WPKH outputs — the library today | 2 | 0.0013%, about 1 in 75,000 |
| 2-in 3-out, all three outputs P2TR — a simple taproot channel | 7 | 0.0046%, about 1 in 21,500 |

Taproot multiplies the anonymity set by about three and a half, and leaves the
transaction rare enough for an analyst to enumerate a month of candidates on a
laptop. The binding constraint is the output count, not the script type: 84.41% of
mainnet transactions have exactly two outputs, and only 2.84% have three. Taproot
changes what the outputs look like; it cannot change how many there are.

## An outside observation of the same point, from 2023

Dan Gould, writing at payjoin.org on 2023-05-09, made the taproot half of this
argument publicly:

> "Bob's lightning output helps preserve privacy even more so because it belongs
> to two parties, both Bob and his channel peer. When P2TR channels are the norm,
> a stranger would not even know that output0 is for lightning."

The observation is correct about the output. The measurement above is about the
inputs, and the two do not follow from one another.

## Consequence for the profile split

The claims table in the project definition gives Profile A and Profile B identical
entries for "Funding source unlinkable", "Change does not confirm the wallet" and
"Peeling chain broken" — all Yes. On the present evidence none of those three is
Yes for either profile with the construction as designed. The table needs rebuilding
from the measurements, not from the mechanism's intent.
