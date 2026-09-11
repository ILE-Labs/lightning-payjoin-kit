# Experiment 04 — mainnet structural baseline

Settles U3 and V5, and supplies the real value distributions U2 asked for. Uses
real Bitcoin mainnet data rather than specifications or assumptions.

## Hypotheses

- **H1 (U3/V5).** The library's transaction is structurally indistinguishable from
  ordinary transactions on locktime, sequence and output ordering.
- **H2 (R-P3).** Adopting Bitcoin Core's anti-fee-sniping locktime makes the
  transaction blend in better.
- **H3 (U2).** Real UTXO values constrain which coins can serve as a contributor
  input, and the 99-vbyte cost is material at high fee rates.

## Pass conditions, stated before running

- H1 holds if the library's structural signature — input count, output count,
  nLockTime, nSequence, output script types — appears in at least 1% of real
  transactions. Below that it is a fingerprint, not camouflage.
- H2 holds if anti-fee-sniping locktimes are more common on mainnet than
  `nLockTime = 0`.
- H3 is descriptive; it reports measured shares rather than passing or failing.

## Method

**Sample.** 31 blocks, taken every 144th block (about one per day) walking back
from height 966,300, so the sample spans about a month rather than one stretch of
traffic. Anchor and stride are pinned in `raw/manifest.json`.

**Data.** Each block fetched as raw consensus bytes and parsed locally by
`src/parse_block.py`. Nothing is taken on trust from an explorer's JSON
interpretation.

**Parser self-check.** For every block the parser asserts that it consumed exactly
the block's byte length and that the transaction count matches the explorer's
header count. A parser that silently desynchronised would fail both. All 31 blocks
pass both assertions.

**Totals.** 150,800 transactions, 150,769 excluding coinbases, 246,647 inputs,
spanning 29.8 days.

**Input values.** Raw blocks carry outpoints, not the values being spent, so
prevout values come from a separate subsample fetched through the explorer's
transaction endpoint. That sample is much smaller and is reported separately.

**Environment.** Python 3.12.3, Linux 6.17.0-1022-azure, 2026-09-10. Block data
from blockstream.info and mempool.space.

## Result: H1 REFUTED, H2 REFUTED, H3 measured

### H1 — the library's signature is rare to the point of being unique

| Filter | Count | Share |
|---|---|---|
| all non-coinbase transactions | 150,769 | 100% |
| ...with any P2WSH output | 3,362 | 2.23% |
| ...2-in 3-out, any metadata | 968 | 0.64% |
| ...+ nLockTime 0 + all sequences final | 805 | 0.53% |
| ...+ exactly one P2WSH and two P2WPKH outputs | **2** | **0.0013%** |

The library's exact structural signature matches **2 of 150,769** transactions,
about **1 in 75,000**. V5's pass condition — "no field identifies the library" —
fails decisively.

The two matches are plausibly channel opens themselves: block 964,572 with outputs
`[10,000 / 534,940 / 1,276,338]` and block 963,276 with `[1,708,737 / 147,363 /
799,093]`.

**Shuffling the outputs does not help.** The signature is an order-independent
multiset of script types plus input and output counts. Build item B3 changes the
order and leaves every number in that table unchanged.

**Taproot helps, by a factor of about three and a half, and not enough.** A simple
taproot channel would produce 2-in 3-out with all three outputs P2TR. That shape
occurs 7 times in the sample — 0.0046%, about 1 in 21,500.

### H2 — anti-fee-sniping would make it worse, not better

This contradicts the recommendation the research phase made before seeing chain
data, and the data wins.

| nLockTime | Share |
|---|---|
| zero | **95.34%** |
| height within 100 of its own block (anti-fee-sniping) | 4.46% |
| other block height | 0.10% |
| timestamp | 0.10% |

Bitcoin Core's wallet applies anti-fee-sniping and LDK recommends it, but on real
mainnet traffic it is a 4.5% minority. `nLockTime = 0`, which the library already
sets, is what 95% of transactions do.

The sequence field goes the other way:

| nSequence, per input | Share |
|---|---|
| `0xfffffffd` (BIP125 RBF) | **54.48%** |
| `0xffffffff` (final) | 41.05% |
| `< 0xfffffffd` (other RBF) | 4.23% |
| `0xfffffffe` (non-final) | 0.24% |

Combining the two fields ranks the options unambiguously:

| Combination | Share |
|---|---|
| all RBF-signalling + nLockTime 0 | **67.10%** |
| all `0xfffffffd` + nLockTime 0 | **64.68%** |
| all `0xffffffff` + nLockTime 0 — *the library today* | 28.09% |
| all RBF-signalling + anti-fee-sniping locktime | 4.25% |

**The empirically best metadata is `nSequence = 0xfffffffd` with `nLockTime = 0`.**
It moves the library from 28% to 65% of traffic, satisfies BOLT 2's requirement,
and enables fee-bumping — without the 4.25% exposure anti-fee-sniping brings.

### H3 — see `raw/utxo-dist.txt` and [open question U2](../04-open-questions.md)

## Reproduce

    ./run.sh                          # fetches the sample if absent, then analyses
    python3 src/utxo_dist.py          # value distributions
    python3 src/fetch_prevouts.py     # refresh the prevout subsample

Fetching the 31 blocks takes a few minutes and about 47 MB.
