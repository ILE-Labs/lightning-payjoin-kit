# Experiment 01 — fee residue, weight estimation, marginal cost

**Hypotheses.**

- **H1 (R-P1).** For every contributor UTXO value `V` and fee rate `r` the library
  accepts, the contributor's change is exactly `V - 99r`, so `V - change = 99r`
  and `r` is recoverable as `(V - change) / 99`.
- **H2 (R-P4).** The estimator charges the P2WSH funding output at the P2WPKH size
  of 31 vbytes, so every transaction pays below its target fee rate.
- **H3 (claim C7).** The collaborative construction adds approximately 99 vbytes
  over the single-funder fallback.

**Pass conditions, stated before running.**

- H1 holds if the residue equals `99r` in **every** configuration that produces a
  transaction. A single counterexample refutes it.
- H2 holds if the measured effective fee rate is **below** target at every sampled
  rate, and the shortfall is consistent with the P2WSH output being undercharged.
- H3 holds if the measured vsize difference is within 5 vbytes of 99.

**Method.** The crate under audit is compiled **unmodified** as a path dependency;
nothing in `src/` is copied or edited. Transactions are built through the library's
own `FundingPsbtBuilder`. Sizes are measured by serializing, not by re-implementing
the estimator: `TxOut` sizes via `consensus_encode`, transaction vsize via
`Transaction::vsize` on a clone populated with realistic P2WPKH witnesses (72-byte
signature, 33-byte pubkey).

Fully deterministic. No RNG. Output is byte-identical across runs.

**Sample.** Fee rates 1, 2, 3, 5, 8, 13, 21, 50, 100, 300 sat/vB crossed with
contributor values 20,000 / 55,000 / 199,999 / 200,000 / 1,234,567 / 5,000,000 sat
— 60 configurations. Channel value 1,000,000 sat, initiator UTXO 1,100,000 sat.

**Environment.** rustc 1.98.1, cargo 1.98.1, Linux 6.17.0-1022-azure, 2026-09-10.

**Results.**

- **H1: CONFIRMED.** 59 of 60 configurations produced a transaction; in 59 of 59
  the residue equalled `99r` exactly and `residue / 99` recovered `r` exactly. The
  60th (20,000 sat at 300 sat/vB) was rejected by library policy before a
  transaction existed, because `99 x 300 = 29,700` exceeds the UTXO.
- **H2: CONFIRMED, and the shortfall is larger than R-P4 states.** Measured P2WPKH
  `TxOut` = 31 bytes, P2WSH `TxOut` = 43 bytes. Effective rate is 94.84% of target
  at every sampled rate — a constant 5.16% shortfall. The gap is 13 vbytes, not 12:
  12 from the P2WSH output, 0.5 from the segwit marker and flag which
  `TX_OVERHEAD_VBYTES = 10` omits, 0.5 from rounding up to whole vbytes.
  **V4 (within 5% of target) fails 7 of 7 sampled rates.**
- **H3: CONFIRMED, exactly rather than approximately.** Fallback 153 vbytes,
  collaborative 252 vbytes, difference exactly 99. The library charges the
  contributor exactly `99r`, so the contributor's share is correctly priced and the
  whole 13-vbyte error sits in the single-funder base construction.

**Reproduce.** `./run.sh`
