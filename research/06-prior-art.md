# Prior art, and what this project adds

The earlier documentation said "No implementation exists for Lightning channel
funding — until now." That is not correct. This is the field as it stands, and the
delta stated once, defensibly.

## What already exists

**BOLT 2 channel establishment v2 (dual funding).** "Interactive transaction
construction allows two peers to collaboratively build a transaction for broadcast.
This protocol is the foundation for dual-funded channels establishment (v2)." Both
peers contribute inputs to one funding transaction. Live in Core Lightning and
Eclair, implemented in LDK. Requires both peers to support it.

**nolooking.** A real BIP-78 implementation that opens Lightning channels from an
inbound payjoin payment, created 2022-07-24, requiring "LND v0.15.1 or higher". The
node operator is the payjoin *receiver*; an external wallet — Sparrow, BTCPay,
Wasabi — is the sender. Its stated benefit is efficiency as much as privacy:
merging two transactions into one, "saving 106 vB". It is self-described
"EXPERIMENTAL ALPHA" with "no professional independent Rust and Bitcoin security
review yet", and its last commit is 2023-06-19.

**The idea in public discussion.** Dan Gould, writing at payjoin.org on 2023-05-09:
"Yes, even lightning channel payjoin outputs are viable. Lightning channels are
posted as 2-of-2 P2SH or P2TR addresses." And: "New standard lightning protocols
allow for payjoin output splicing and dual funded channels to achieve the common
input assumption-busting result even without BIP 78 as well."

## The delta

Collaborative funding of a Lightning channel is not new. What this library adds is
narrower: **a construction in which the second party contributes an input and
receives it back in full as change, taking on no channel balance and no capital
lockup, over v1 channel establishment, so the peer needs no dual-funding support
and no modification.**

Each clause is doing work, and each distinguishes it from something:

- *receives it back in full as change* — unlike dual funding, where the
  contribution becomes channel capacity, and unlike nolooking, where the
  contribution is a payment the sender is making anyway.
- *over v1 channel establishment* — unlike dual funding, which requires
  `option_dual_fund` on both sides.
- *the peer needs no modification* — unlike nolooking, which requires the operator
  to run LND and a service alongside it.

## The part that has to be said in the same breath

The delta is a difference in mechanism, not in privacy achieved.

The measurements in `02-the-central-result.md` find that returning the contributed
value to the contributor is precisely what makes the transaction partitionable, and
that a construction absorbing the contribution into the funding output — which is
what dual funding already does — resists the same attacks. On the present evidence
the feature that distinguishes this project from its prior art is the reason it
does not work.

That is not an argument for abandoning the work. It is an argument for being
accurate about where the work now points: toward constructions in which contributed
value ends up somewhere the contributor does not own, which is territory dual
funding and splicing already occupy. The open question is what a privacy-motivated
version of that looks like, not whether the ground is empty. It is not empty, and
saying so was never a good look.
