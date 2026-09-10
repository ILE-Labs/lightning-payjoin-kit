# Experiment 01 — analysis

## What it means

**The residue is not noise, it is an identity.** `V - change = 99r` is exact, so an
observer does not need to estimate anything. They compute the transaction's fee
rate from its own fee and vsize, multiply by 99, and look for a pair of values
differing by that amount. Experiment 02 measures how often that succeeds: 99.997%
of 20,000 samples.

**The fee under-estimate is a pre-existing bug in the fallback path.** This is the
part worth stating carefully, because it is easy to get backwards. The
single-funder fallback estimates 140 vbytes and measures 153. The collaborative
step then adds exactly 99 estimated and exactly 99 measured. So the collaborative
path is correctly priced and the base path is wrong by 13 vbytes. Fixing R-P4 does
not change what the contributor pays; it changes what the initiator pays.

**The 5.16% shortfall is constant, not random.** It does not vary with fee rate,
because it is a fixed vbyte error over a fixed structure. That makes it both easy
to fix and — until fixed — a stable fingerprint in its own right: a transaction
whose fee is exactly `239r` for a 2-in-3-out shape is a transaction built by this
estimator.

**99 vbytes is the right marginal figure and it is not "no extra fees".** At 1
sat/vB the contributor pays 99 sat; at 100 sat/vB, 9,900 sat. Whether that is
acceptable is an economics question, and R-P6 argues the contributor has no
offsetting benefit at all.

## What it does not mean

- **It does not measure privacy.** It measures arithmetic. That the residue exists
  is necessary for the fee-residue attack but the attack's success rate is
  experiment 02's result, not this one's.
- **It does not establish that fixing R-P1 helps.** Experiment 02 finds it moves
  subset-sum partitioning by 0.01 of a percentage point.
- **The witness size is an estimate, not a measurement.** A 72-byte DER signature
  is the maximum; Bitcoin Core grinds for low-R signatures, which are 71 bytes, so
  real transactions are marginally smaller and the real effective rate marginally
  better than 94.84%. The direction of the error is unchanged.
- **Only P2WPKH contributors were tested.** A contributor spending P2TR would
  contribute different weight and a different constant, and the library has no
  branch for that — it applies `P2WPKH_INPUT_VBYTES` to every input regardless of
  script type. That is a further under- or over-charge not quantified here.
- **Only the two-party, one-input-each case was tested.** Multiple contributor
  inputs, or an initiator needing several inputs, were not sampled.

## Threats to validity

**The reference is the same library's own transaction model.** Sizes come from
`bitcoin`'s `consensus_encode` and `Transaction::vsize`, which is an independent
implementation from the library's estimator, so the comparison is meaningful. But
both agree on what a transaction *is*; a consensus-level surprise would not be
caught.

**The dust threshold is the library's own 546 sat constant.** Bitcoin Core's dust
limit is script-type dependent and is not 546 for every output type. The rejection
observed at 20,000 sat / 300 sat/vB is a library-policy rejection, not a consensus
or relay one.

**Nothing was broadcast.** No transaction from this experiment was submitted to a
node, so relay acceptance is untested here. `tests/corepc_regtest.rs` exists for
that and was not run.

## Connection to the specification

The 99-vbyte figure is not arbitrary and not this library's invention. BOLT 2's
"Fee Responsibility" assigns each peer the fees for the bytes they contributed, so
a peer adding one P2WPKH input and one P2WPKH change output owes exactly this
weight. See the BOLT 2 fee-rule note: the residue is a property of the interactive-tx fee rule, which
means dual-funded opens share it, and which means randomising it — build item B2 —
diverges from the specification anywhere interactive-tx applies.
