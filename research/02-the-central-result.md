# The privacy-input construction does not survive partitioning, and fixing the fee arithmetic does not change that

## Summary

A collaborative funding transaction in which the second party contributes an input
and receives the same value back as change can be partitioned by an observer with
near-certainty. This holds after every remediation currently planned — randomised
fee apportionment, shuffled output ordering, anti-fee-sniping locktimes,
RBF-signalling sequences and corrected weight estimation — and it holds when the
funding output is a taproot output indistinguishable from ordinary change.

The reason is structural rather than arithmetic, and it can be stated without
reference to any experiment.

## The argument

Write `V` for the contributor's input value, `C` for the change they receive, `I`
for the initiator's input value, `D` for the initiator's change, `K` for the
channel capacity and `F` for the total transaction fee. The contributor supplies
an input and takes it back, paying only some share `f_c` of the fee, so

    |V - C| = f_c  <=  F

The initiator pays for the channel, so

    |I - D| = K + f_i  >=  K

Every channel worth opening has `K` far larger than `F`. At the fee rates and
capacities used below the ratio ranges from about 33x at the least favourable
corner to about 2,000x at the most common one. The contributor's input and their
change therefore sit orders of magnitude closer together than any other
input/output pair in the transaction, and an observer needs only to sort pairs by
absolute difference and take the smallest.

Three things follow, and each of them is the point.

**Randomising the fee apportionment does not help.** It changes `f_c`, but `f_c`
is bounded by `F` no matter how it is drawn. The bound is what the attack uses.

**Shuffling the outputs does not help.** Sorting by absolute difference does not
care what order the outputs arrive in.

**The result does not depend on the UTXO distribution.** `V` and `I` do not appear
in the inequalities. There is no distribution of realistic UTXOs that repairs
this, because the attack never looks at the distribution.

## What was measured

An adversarial harness generated 20,000 transaction pairs per seed across fee
rates of 1, 2, 5, 10, 25 and 60 sat/vB and channel capacities of 0.5M, 1M, 2M and
5M sat, drawing UTXO values log-uniformly from 20,000 to 50,000,000 sat. Three
constructions were compared under four attacks. The chance baseline is 50%: a
two-input, two-change transaction admits exactly two pairings, so an adversary
guessing at random is right half the time. Attacks that decline to answer are
scored as coin flips, which can only flatter the construction.

- **Construction A** — the library as it stands.
- **Construction B** — A plus every fix in the build scope: randomised fee
  apportionment, shuffled inputs and outputs, anti-fee-sniping locktime,
  `nSequence = 0xfffffffd`, and the funding output charged its true 43 bytes.
- **Construction C** — the contributed value absorbed into the funding output, so
  the contributor genuinely funds part of the channel, as dual funding does.

Attack success rate, seed 20260910, 20,000 samples:

| Attack | A (current) | B (remediated) | C (absorbed) |
|---|---|---|---|
| A1 fee residue | 99.997% | 50.23% | n/a |
| A2 subset-sum partitioning | 99.86% | 99.85% | 49.96% |
| A5 near-equality, no fee arithmetic | 99.75% | 99.76% | 57.04% |
| A5 near-equality, all outputs P2TR | — | 99.49% | 53.27% |

Repeated at seeds 1 and 987654321; every figure is stable to within 0.11
percentage points.

## Reading the table

**The planned fixes work only against the attack they were designed for.**
Randomised fee apportionment takes the fee-residue attack from 99.997% to 50.23%,
which is chance. That fix is real and worth making. It moves subset-sum
partitioning by 0.01 of a percentage point.

**The decisive attack uses no fee arithmetic at all.** The near-equality attack
pairs the input and output whose values are closest and commits when that pair is
clearly closer than the runner-up. It succeeds 99.76% of the time against the
fully remediated construction. Nothing in the build scope touches it.

**Taproot does not rescue it.** With every output recast as P2TR, so that an
observer genuinely cannot tell which output is the channel, near-equality still
succeeds 99.49% of the time. Concealing *which output is the channel* does not
conceal *which input and output belong to the same party*, and it is the second
that this construction was built to conceal.

**Absorbing the contribution does help, substantially.** Construction C defeats
subset-sum outright — no consistent partition exists in 99.9% of samples — and
brings near-equality down from 99.8% to 57.0%, or 53.3% with taproot outputs.
53.3% is close to chance, though still above a 55% gate only in the non-taproot
case.

## Consequence

The distinguishing feature of this construction — that the contributor supplies an
input and takes it straight back, so they lock up no capital and gain no channel
balance — is the same feature that makes the transaction partitionable. The two
cannot be separated by better arithmetic, because the leak is the near-equality
that the design requires.

This is the outcome pivot trigger T1 was written for, and T1 states the response:
move to a construction where contributed value is absorbed into the funding
output. Construction C is a first evaluation of exactly that, and it is the only
construction tested here that resists subset-sum partitioning.

## Corroboration from two directions the harness did not reach

**The construction is partitionable on a real chain, not only in a model.**
A collaborative funding transaction built by the library was broadcast to Bitcoin
Core 28.0 on regtest and mined into block 103. The contributor supplied 200,000
sat and received 199,802 back — a difference of 198, which is 99 × the test's 2
sat/vB. The residue this finding describes is present in a confirmed transaction,
not just in the builder.

**The transaction is also rare enough to enumerate.** Independently of any
partitioning attack, the library's structural signature — 2 inputs, 3 outputs, one
P2WSH and two P2WPKH outputs — matches **2 of 150,769** mainnet transactions in a
29.8-day sample, about 1 in 75,000. Recast as taproot it matches 7, about 1 in
21,500.

These two findings compound rather than overlap. Being enumerable means an analyst
can find the candidate transactions cheaply; being partitionable means that once
found, each one gives up its ownership structure. Neither is fixed by the build
scope, and the shape that causes the second — a contributor who takes their money
back, so a third output is needed — is the same choice that causes the first.

## What this finding does not establish

- It does not show that construction C is safe. C was evaluated as a sketch, with
  the contributor funding a uniform 20-80% of the channel and paying half the fee.
  It fails the near-equality gate in the non-taproot case (57.0%) and passes only
  narrowly with taproot outputs (53.3%). It needs its own design and its own
  verification round.
- It does not measure anything about real transactions. Both constructions and the
  UTXO draw are synthetic. The UTXO distribution is a stated assumption, not a
  measurement, and no distribution was fitted to chain data.
- The near-equality attack was not calibrated against a corpus of ordinary
  transactions, so its false-positive rate in the wild is unknown. Its behaviour
  against construction C — 57.0%, near chance — is the only internal control, and
  one control is not a calibration.
- The model is a value-and-metadata model, not a Bitcoin transaction. It ignores
  address reuse, input ordering conventions, script-type mixing across inputs, and
  every temporal signal.
- Only the two-party case was examined. Anonymity is expected to scale with
  participants, and the batched multi-party construction named in T1 was not
  tested.

## Reproducing

    cd research/adversarial-harness
    ./run.sh

Defaults to seed 20260910 and 20,000 samples; override with the `SEED` and
`SAMPLES` environment variables. The harness has no dependencies beyond the Rust
standard library and its PRNG is seeded and self-contained, so output is
byte-identical across runs on the same toolchain.
