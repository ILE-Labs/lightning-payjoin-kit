# Experiment 02 — adversarial partitioning harness

The reproducible implementation of the unnecessary-input and subset-sum heuristics
that the project definition names as a deliverable in its own right. It is
self-contained — no dependencies beyond the Rust standard library — so a reviewer
can run it against the constructions defined here, or add their own.

## Hypothesis

The MVP's central bet: a Lightning channel funding transaction can be built
collaboratively such that an analyst applying the unnecessary-input and subset-sum
heuristics cannot partition its inputs and outputs by owner.

## Pass condition, stated before running

V1's condition, made numeric. The adversary must pair the contributor's input with
the contributor's change **no more often than chance**. For a two-input,
two-change transaction there are exactly two pairings, so chance is 50%. The gate
is **attack success ≤ 55%**, allowing 5 percentage points of margin.

Attacks that decline to answer are scored as coin flips, because an adversary who
cannot distinguish still guesses correctly half the time. This can only make a
construction look worse than a generous accounting would, never better.

## Method

Three constructions, generated from the same UTXO draws so the only difference is
the construction:

- **A — current.** Fee apportionment fixed at `99 x feerate`; outputs ordered
  `[funding, initiator change, contributor change]`; `nLockTime = 0`; every
  `nSequence = 0xffffffff`.
- **B — remediated.** Every fix in the MVP build scope: B2 randomised fee
  apportionment (contributor's share drawn uniformly from ±60% of their marginal
  cost), B3 shuffled inputs and outputs, B4 anti-fee-sniping locktime following
  Bitcoin Core's `DiscourageFeeSniping` and `nSequence = 0xfffffffd` per BOLT 2,
  B5 the funding output charged its true 43 bytes.
- **C — absorbed.** Pivot trigger T1's construction: the contributor genuinely
  funds 20–80% of the channel and takes change only on the remainder, so no output
  mirrors their input.

Five attacks, each given only what a chain observer has — values, counts, script
kinds, `nSequence`, `nLockTime`. None reads the ownership labels.

- **A1 fee residue.** Tests each (input, change) pair for a difference that is an
  exact multiple of 99 whose quotient matches the transaction's own apparent fee
  rate.
- **A2 subset-sum.** Enumerates every split of inputs and change outputs between
  two parties and keeps those where each side's arithmetic balances.
- **A2-blind / A2-taproot.** A2 without being told which output is the channel.
  Candidate outputs whose script kind differs from the rest are discarded, so the
  observer is genuinely blind only when every output is the same kind — which is
  what `as_taproot()` models.
- **A3 UIH.** All three published variants from Ghesmati et al. — BlockSci,
  BlockStream and Gibson — applied to the two change outputs with the funding
  output as the payment. This generalises beyond the paper's stated two-output
  scope, and the divergence is reported rather than hidden.
- **A5 near-equality.** Pairs the input and output whose values are closest,
  committing only when that pair is clearly closer than the runner-up. Uses no fee
  arithmetic at all. **A5-blind** treats every output as a candidate change output,
  which is the taproot case.

**A note on construction B's metadata.** B implements the build scope *as written*,
including its anti-fee-sniping locktime. Mainnet measurement later showed that
choice to be wrong for blending in — `nLockTime = 0` is 95.34% of real traffic and
anti-fee-sniping locktimes are 4.46% — so B's A4 row reads 0.00% against a 95.34%
norm. That is deliberate: the harness tests the plan that was written, and the
metadata correction is reported separately. It changes none of the partitioning
results, because A2 and A5 read values, not metadata.

**Sample.** 20,000 transaction pairs per seed. Fee rates 1, 2, 5, 10, 25, 60
sat/vB; capacities 0.5M, 1M, 2M, 5M sat; UTXO values drawn log-uniformly from
20,000 to 50,000,000 sat. The distribution is a **stated assumption**, not a
measurement — see `analysis.md`.

**RNG.** xoshiro256\*\*, seeded, self-contained. Seeds 20260910, 1 and 987654321.
Raw output in `raw/`; the three-seed comparison in `raw/seed-stability.txt`.

**Environment.** rustc 1.98.1, cargo 1.98.1, Linux 6.17.0-1022-azure, 2026-09-10.

## Result: FAIL, and the hypothesis is refuted

Attack success, seed 20260910:

| Attack | A current | B remediated | C absorbed |
|---|---|---|---|
| A1 fee residue | 99.997% | 50.23% ✓ | n/a |
| A2 subset-sum | 99.86% | 99.85% | 49.96% ✓ |
| A5 near-equality | 99.75% | 99.76% | 57.04% |
| A5-blind, all outputs P2TR | — | 99.49% | 53.27% ✓ |

✓ marks a pass at the 55% gate. Stable to within 0.11 percentage points across the
three seeds.

Applying every remediation in the build scope leaves the transaction partitionable
99.85% of the time. Recasting the outputs as taproot does not change that. The
full reading is in [the central result](../02-the-central-result.md); the limits of what this shows
are in `analysis.md`.

## Reproduce

    ./run.sh                       # seed 20260910, 20,000 samples
    SEED=1 SAMPLES=50000 ./run.sh  # override

or directly:

    cargo run --release -- --seed 20260910 --samples 20000 \
        --min-utxo 20000 --max-utxo 50000000 --jitter-pct 60
