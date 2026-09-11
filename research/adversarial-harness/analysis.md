# Experiment 02 — analysis

## What it means

**The remediations work, and they are not the thing that was wrong.** Randomised
fee apportionment takes the fee-residue attack from 99.997% to 50.23%, which is
chance. That is a complete defeat of the attack it targets, and B2 is worth
implementing. It moves subset-sum partitioning from 99.86% to 99.85%.

That gap is the whole finding. The project has identified a real defect (R-P1),
designed a fix that works, and the fix does not deliver privacy, because R-P1 was
never the reason the transaction is partitionable.

**The decisive attack is the one nobody specified.** A5 pairs the input and output
whose values are closest. It reads no fee, no script type, no ordering, no
metadata. It succeeds 99.76% of the time against the fully remediated construction.
Every item in the build scope is invisible to it.

The reason is stated analytically in the central result and does not depend on this experiment:
the contributor's input and change differ by at most one transaction fee, while
the initiator's differ by at least the channel capacity. For any channel worth
opening those are orders of magnitude apart.

**Taproot does not help, and the reason is worth being precise about.** Recasting
every output as P2TR removes the observer's ability to tell *which output is the
channel*. A2 becomes ambiguous — construction B leaves exactly two candidate
pairings in 99.69% of samples, which is genuine ambiguity. But A5-blind still wins
99.49% of the time, because knowing which output is the channel was never necessary
for spotting that an input and an output are nearly equal. Concealing the channel
and concealing the ownership partition are different problems, and Layer 3 solves
only the first.

**Absorbing the contribution is the change that matters.** Construction C defeats
subset-sum outright — no consistent split exists in 99.9% of samples — and brings
A5 from 99.8% to 57.0%, or 53.3% under taproot. This is the pivot T1 names, and it
is the only construction here that resists.

## What it does not mean

**C is not shown to be safe.** It was written as a sketch to test the pivot's
premise, not as a design. It fails the A5 gate at 57.0% without taproot and passes
only narrowly at 53.3% with it. Its parameters — 20–80% funding share, fee split
down the middle — were chosen for coverage, not from any analysis. A real version
needs its own design and its own verification round.

**Nothing here measures real transactions.** Every transaction is synthetic. The
attacks have never been run against chain data, and no result here says anything
about what an actual analyst would find in an actual block.

**The UTXO distribution is an assumption.** Log-uniform over [20,000, 50,000,000]
sat was chosen to span a wide range cheaply, not because it fits anything. It is
exposed as `--min-utxo` and `--max-utxo` so a reviewer can substitute a
distribution they can defend. For the headline finding this does not matter — the
near-equality bound contains no UTXO term, so the result is distribution-independent
(see U2) — but it does matter for anything that reads the percentages as
frequencies.

**A5's false-positive rate in the wild is unknown.** It was never calibrated
against a corpus of ordinary transactions. Its 57.0% against construction C is the
only internal control, and one control is not a calibration. An attack that fires
on ordinary transactions too would be far less useful to a real analyst than these
numbers suggest.

**The UIH figures are not comparable to the published base rates.** Ghesmati et
al.'s 41.96% / 41.77% / 27.18% are measured over real multi-input, exactly-two-output
transactions. These transactions have three outputs, and the harness generalises
the heuristics to reach them. The columns are placed side by side in U1 for
orientation, not as a like-for-like comparison.

**The model is not a Bitcoin transaction.** It carries values, script kinds,
`nSequence` and `nLockTime`, and nothing else. Address reuse, input ordering
conventions, script-type mixing across inputs, `scriptSig` shapes, RBF history,
broadcast timing and mempool observation are all absent. Several of those are
things a real analyst would use first.

**Only two parties.** Anonymity is expected to scale with participants, and the
batched multi-party construction T1 also names was not built or tested.

## Threats to validity

**The attacks were written by the same person as the constructions**, which is the
standard hazard: an attack can be tuned, consciously or not, to the construction it
is aimed at. Two things partially mitigate it. A5 is parameter-light — one margin
ratio, set at 0.25 and not varied — and its behaviour against construction C
(near chance) shows it is not trivially always-right. But an independent
implementation of the same heuristics would be a real improvement, and this harness
is published partly so that someone can write one.

**Scoring undecided attacks as coin flips is a choice.** It is the conservative
choice — it lowers reported success — but it means "50.000%" in the A2-taproot row
denotes "the adversary is reduced to guessing", not "the adversary learns nothing".
The candidate-set counts are reported alongside precisely so that distinction stays
visible.

**The gate is 55% and several results sit near it.** Construction C's 53.3% passes
and its 57.0% fails, on either side of a threshold chosen in advance but chosen
arbitrarily. Neither number should be read as a verdict on construction C; both
should be read as "close to chance, needs proper design and proper evaluation".

**Only one remediation parameterisation was tested.** The fee jitter is ±60%.
Wider jitter would cost the contributor more without touching A5, and was not
swept.
