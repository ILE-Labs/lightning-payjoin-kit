# Experiment 04 — analysis

## What it means

**The transaction is a fingerprint, not camouflage.** One in 75,000 is not an
anonymity set. An analyst filtering a month of mainnet for this shape gets two
transactions, and both look like they might be channel opens already. Anything the
library produces would be trivially enumerable, and R-P2's fixed ordering is not
what makes it so — the ordering is not even in the signature.

**The binding constraint is the shape, not the metadata.** Of the 150,769
transactions, 84.41% have exactly two outputs and 90.91% have one input. A 2-in
3-out transaction is already in the 0.64% before any script type is considered.
The library cannot fix this by changing fields; the shape is what the construction
requires — two parties' inputs, a funding output and two changes.

That is the same conclusion the central result reached from a completely different direction.
There, the contributor's input and change are near-equal because the design
returns their money. Here, the transaction has three outputs because the design
needs two changes. Both are consequences of the same choice, and neither is
reachable by adjusting a field.

**Taproot's benefit is real and small.** Going from 1 in 75,000 to 1 in 21,500 is
a 3.5× improvement in the anonymity set and still leaves the transaction rare
enough to enumerate. It also does not touch the near-equality attack, which the central result
measures at 99.49% against taproot outputs. Layer 3 helps at the margin on both
axes and is decisive on neither.

**The anti-fee-sniping recommendation was wrong and is corrected.** Before this
experiment the research record recommended adopting Bitcoin Core's
`DiscourageFeeSniping` because Core implements it, BOLT 2 discusses fingerprinting,
and LDK's documentation explicitly asks for it. Chain data says anti-fee-sniping
locktimes appear on 4.46% of mainnet transactions while `nLockTime = 0` appears on
95.34%. Adopting it would move the library out of the majority and into a small
minority.

There is a genuine tension here rather than a simple error, and it should be
recorded as such:

- Anti-fee-sniping exists to align miner incentives, which is a network-health
  goal, not a privacy goal. Bitcoin Core's comment says so directly: "we always
  want the blockchain to move forward."
- Core's own comment does claim a privacy benefit, but a specific one, for
  "transactions that are delayed after signing for whatever reason, e.g.
  high-latency mix networks and some CoinJoin implementations". A collaborative
  funding round-trip is arguably in that class.
- Against that, the measured cost is joining a 4.46% population.

The recommendation that survives both considerations is the sequence change alone:
`0xfffffffd` with `nLockTime = 0` is 64.68% of real traffic, is what BOLT 2
requires of interactive construction, is what Bitcoin Core emits when signalling
RBF, and permits fee-bumping a stuck funding transaction. It is the only change to
these fields that improves every axis at once.

## What it does not mean

**A month is not all of history.** The sample spans 29.8 days ending at height
966,300. Transaction mix changes; a sample from another period would give different
percentages. The order of magnitude on the signature — rare enough to enumerate —
is unlikely to move, but the exact "2 in 150,769" is a point estimate from one
month.

**This period has heavy OP_RETURN traffic.** 31.10% of all outputs are
`op_return`, and the single most common transaction shape is 1-in 2-out with one
OP_RETURN and one P2WPKH at 56.50% of the sample. That is data-carrier traffic
inflating the denominator. It makes ordinary payments a smaller share of the
sample than a naive reading suggests, and it means "0.0013% of transactions" is
partly a statement about how much non-payment traffic is on-chain. Restricting the
denominator to payment-like transactions would raise the library's share, though
not by anything close to the two orders of magnitude that would matter.

**The signature test is exact-match.** It requires 2 inputs, 3 outputs, nLockTime
0, all sequences final, and exactly one P2WSH plus two P2WPKH outputs. A real
analyst would use a looser filter with a higher recall and more false positives.
The nested table is given precisely so a reader can pick their own filter: at
"2-in 3-out + locktime 0 + all final" the share is 0.53%, still well under the 1%
pass condition.

**Two matches is a small number.** With only two, the estimate 1-in-75,000 carries
wide Poisson error — roughly a factor of three either way at 95% confidence. The
finding does not depend on the precision: at the upper end of that interval the
signature is still rarer than 1 in 25,000.

**Nothing here identifies actual Lightning channel opens.** The two matching
transactions are consistent with channel opens but were not confirmed as such. No
gossip data was cross-referenced.

## Threats to validity

**The parser is mine.** It is 130 lines and could in principle mis-parse in a way
that produces plausible-looking output. Two checks make that unlikely: it consumes
exactly the block byte length on all 31 blocks, and its transaction count matches
the explorer's independently-reported count on all 31. A desynchronised parser
would fail both simultaneously, which is hard to do by accident.

**Script classification is by exact shape.** `classify_spk` matches template byte
patterns. A non-standard output that coincidentally matched a template would be
misfiled. The `nonstandard` bucket holds 3 outputs of 332,857, so almost everything
in the sample fits a known template.

**Sampling every 144th block is not random.** It is systematic, which protects
against clustering in one traffic regime but would alias against anything with a
one-day period. Nothing in the fields measured has an obvious daily cycle, but
this was not checked.

**Two explorers, one interpretation.** Block bytes came from blockstream.info and
mempool.space interchangeably. Because the analysis parses raw consensus bytes and
verifies the length, a mirror serving corrupt data would be caught. The transaction
*counts* used as the cross-check come from those same explorers, so that particular
check is not fully independent.
