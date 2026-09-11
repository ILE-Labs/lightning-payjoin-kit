//! The adversary.
//!
//! Four attacks. Each is given only what a chain observer has: values, counts,
//! script kinds, nSequence, nLockTime. None of them reads the ownership labels.
//!
//! Attacks A1-A3 answer one question: **which input and which change output
//! belong to the same party?** For a two-input, two-change transaction there are
//! exactly two possible pairings, so an adversary guessing at random is right
//! half the time. That 50% is the chance baseline the V1 gate is measured against.

use crate::tx::{Output, Tx};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The attack committed to a pairing.
    Pairing { input: usize, change: usize },
    /// The attack found more than one consistent answer and declined to choose.
    Ambiguous,
    /// The attack found no consistent answer at all.
    NoAnswer,
}

/// A1 — the fee-residue attack.
///
/// Specific to a construction that charges a contributor a fixed number of
/// vbytes. For every (input, change) pair it tests whether the difference is an
/// exact multiple of `vbytes` whose quotient equals the fee rate the transaction
/// itself discloses. It needs no ground truth and no prior about the parties.
pub fn a1_fee_residue(tx: &Tx, vbytes: u64) -> Verdict {
    let observed = tx.observed_feerate();
    let mut hits = Vec::new();
    for (ii, input) in tx.inputs.iter().enumerate() {
        for ci in tx.change_indices() {
            let out = &tx.outputs[ci];
            if input.value <= out.value {
                continue;
            }
            let residue = input.value - out.value;
            if residue % vbytes != 0 {
                continue;
            }
            let implied = (residue / vbytes) as f64;
            // The implied rate must agree with the transaction's own apparent
            // rate. Tolerance is generous: an observer only needs the right
            // order of magnitude to rule the alternative out.
            if (implied - observed).abs() / observed.max(1e-9) <= 0.25 {
                hits.push((ii, ci));
            }
        }
    }
    match hits.len() {
        0 => Verdict::NoAnswer,
        1 => Verdict::Pairing { input: hits[0].0, change: hits[0].1 },
        _ => Verdict::Ambiguous,
    }
}

/// A2 — subset-sum partitioning.
///
/// The general attack named in the MVP verification plan. It enumerates every way
/// of splitting inputs and change outputs between two parties, requires the shared
/// funding output to be paid for jointly, and keeps only splits where each party's
/// arithmetic balances: their inputs minus their change equals their share of the
/// funding output plus their share of the fee, for some non-negative split of both.
///
/// A construction survives this attack when more than one split balances, because
/// the adversary then cannot tell which is real.
pub fn a2_subset_sum(tx: &Tx, tolerance: u64) -> Verdict {
    let n_in = tx.inputs.len();
    let changes = tx.change_indices();
    let funding = tx.outputs[tx.funding_index].value;
    let fee = tx.fee();
    let mut consistent: Vec<(usize, usize)> = Vec::new();

    // Enumerate assignments of inputs to party A (bitmask) and of change outputs
    // to party A. Party B gets the complement.
    for in_mask in 1u32..(1u32 << n_in) - 1 {
        for ch_mask in 0u32..(1u32 << changes.len()) {
            let a_in: u64 = (0..n_in)
                .filter(|i| in_mask >> i & 1 == 1)
                .map(|i| tx.inputs[i].value)
                .sum();
            let b_in: u64 = (0..n_in)
                .filter(|i| in_mask >> i & 1 == 0)
                .map(|i| tx.inputs[i].value)
                .sum();
            let a_ch: u64 = changes
                .iter()
                .enumerate()
                .filter(|(k, _)| ch_mask >> k & 1 == 1)
                .map(|(_, &c)| tx.outputs[c].value)
                .sum();
            let b_ch: u64 = changes
                .iter()
                .enumerate()
                .filter(|(k, _)| ch_mask >> k & 1 == 0)
                .map(|(_, &c)| tx.outputs[c].value)
                .sum();

            if a_in < a_ch || b_in < b_ch {
                continue;
            }
            // What each side put in beyond what it took back out.
            let a_spend = a_in - a_ch;
            let b_spend = b_in - b_ch;
            if a_spend + b_spend != funding + fee {
                continue;
            }
            // A split is credible if each side's contribution is consistent with
            // *some* non-negative division of the funding output and the fee. The
            // interesting case for this project is a contributor who funds none of
            // the channel: their whole spend is fee. Record the split when one
            // side's spend is small enough to be pure fee at the observed rate.
            let fee_only_side = a_spend <= fee + tolerance || b_spend <= fee + tolerance;
            if !fee_only_side {
                continue;
            }
            // Identify the candidate contributor: the side that funded no channel
            // value. Record one representative (input, change) pair for it.
            let (c_in_mask, c_ch_mask) = if a_spend <= b_spend {
                (in_mask, ch_mask)
            } else {
                (!in_mask & ((1 << n_in) - 1), !ch_mask & ((1 << changes.len()) - 1))
            };
            let ci = (0..n_in).find(|i| c_in_mask >> i & 1 == 1);
            let cc = changes
                .iter()
                .enumerate()
                .find(|(k, _)| c_ch_mask >> k & 1 == 1)
                .map(|(_, &c)| c);
            if let (Some(ci), Some(cc)) = (ci, cc) {
                if !consistent.contains(&(ci, cc)) {
                    consistent.push((ci, cc));
                }
            }
        }
    }

    match consistent.len() {
        0 => Verdict::NoAnswer,
        1 => Verdict::Pairing { input: consistent[0].0, change: consistent[0].1 },
        _ => Verdict::Ambiguous,
    }
}

/// A3 — the unnecessary-input heuristic, in the three published forms.
///
/// Ghesmati et al. define these only over transactions with more than one input
/// and **exactly two outputs**. A collaborative channel open has three. The
/// definitions are applied here to the two change outputs, with the shared funding
/// output treated as the payment, which is the closest faithful generalisation;
/// the divergence from the published scope is reported alongside the result rather
/// than hidden.
pub struct UihFlags {
    /// The complement of BlockSci UIH2. Reported by the published definition as a
    /// pair, so both halves are kept even though the harness scores only UIH2.
    #[allow(dead_code)]
    pub blocksci_uih1: bool,
    pub blocksci_uih2: bool,
    pub blockstream_uih2: bool,
    pub gibson_uih2: bool,
}

pub fn a3_uih(tx: &Tx) -> UihFlags {
    let min_in = tx.inputs.iter().map(|i| i.value).min().unwrap();
    let max_in = tx.inputs.iter().map(|i| i.value).max().unwrap();
    let sum_in: u64 = tx.inputs.iter().map(|i| i.value).sum();
    let outs: Vec<u64> = tx.outputs.iter().map(|o| o.value).collect();
    let min_out = *outs.iter().min().unwrap();
    let max_out = *outs.iter().max().unwrap();
    let sum_out: u64 = outs.iter().sum();
    let fee = tx.fee();

    UihFlags {
        // BlockSci: !UIH1. Ghesmati Algorithm 1.
        blocksci_uih1: min_out < min_in,
        blocksci_uih2: !(min_out < min_in),
        // BlockStream: Ghesmati Algorithm 2, first branch.
        blockstream_uih2: sum_in - min_in >= max_out + fee,
        // Gibson: Ghesmati Algorithm 3, second clause.
        gibson_uih2: max_in > max_out,
    }
    .normalise(sum_out)
}

impl UihFlags {
    fn normalise(self, _sum_out: u64) -> Self {
        self
    }
}

/// A4 — metadata fingerprint. Not a partitioning attack: it asks the prior
/// question of whether the transaction is identifiable as this library's output
/// at all, before any value analysis begins.
pub struct Fingerprint {
    pub all_sequences_final: bool,
    pub locktime_zero: bool,
    pub funding_output_first: bool,
    /// Measured on mainnet at 0.12% of transactions, so a construction that mixed
    /// sequence values across the two parties' inputs would stand out sharply.
    pub mixed_sequences: bool,
    /// Kept so a reviewer can filter by script shape without re-reading outputs.
    #[allow(dead_code)]
    pub has_p2wsh_output: bool,
}

pub fn a4_fingerprint(tx: &Tx) -> Fingerprint {
    let seqs: Vec<u32> = tx.inputs.iter().map(|i| i.sequence).collect();
    let first = seqs[0];
    Fingerprint {
        all_sequences_final: seqs.iter().all(|&s| s == 0xffff_ffff),
        locktime_zero: tx.lock_time == 0,
        mixed_sequences: seqs.iter().any(|&s| s != first),
        funding_output_first: tx.funding_index == 0,
        has_p2wsh_output: tx.outputs.iter().any(|o: &Output| o.kind == crate::tx::ScriptKind::P2wsh),
    }
}

/// A2-blind — subset-sum partitioning **without** being told which output is the
/// channel.
///
/// This is the Profile A adversary, and the taproot adversary. When the funding
/// output is a plain P2TR it is not distinguishable from an ordinary
/// single-signature output, so the observer must try every output in turn as the
/// candidate channel and keep every split that balances. The construction wins
/// here if the extra candidates create real ambiguity.
pub fn a2_blind_subset_sum(tx: &Tx, tolerance: u64) -> (Verdict, usize) {
    let mut all: Vec<(usize, usize)> = Vec::new();
    for cand in 0..tx.outputs.len() {
        // An observer who can read script types will not entertain a candidate
        // channel output whose script kind differs from the others. Only when
        // every output is the same kind is the observer genuinely blind.
        let uniform = tx.outputs.iter().all(|o| o.kind == tx.outputs[0].kind);
        if !uniform && cand != tx.funding_index {
            continue;
        }
        let mut probe = tx.clone();
        probe.funding_index = cand;
        if let Verdict::Pairing { input, change } = a2_subset_sum(&probe, tolerance) {
            if !all.contains(&(input, change)) {
                all.push((input, change));
            }
        }
    }
    let n = all.len();
    let v = match n {
        0 => Verdict::NoAnswer,
        1 => Verdict::Pairing { input: all[0].0, change: all[0].1 },
        _ => Verdict::Ambiguous,
    };
    (v, n)
}

/// A5 — the near-equality attack.
///
/// The simplest attack there is, and it uses no fee arithmetic at all. A
/// contributor who supplies an input and takes it straight back as change leaves
/// an input and an output of almost the same value. The attack pairs the input
/// and change output whose values are closest, and commits when that pair is
/// closer than any other by a clear margin.
///
/// It is included because it is invariant to everything the build scope proposes:
/// randomising the fee split, shuffling the outputs and rewriting the metadata all
/// leave the near-equality untouched.
pub fn a5_near_equality(tx: &Tx, margin_ratio: f64) -> Verdict {
    a5_inner(tx, margin_ratio, false)
}

/// A5-blind — the same attack for an observer who has not been told which output
/// is the channel, so every output is a candidate change output. This is the
/// attack that a taproot channel must survive, because taproot removes the script
/// type that would otherwise give the funding output away.
pub fn a5_blind_near_equality(tx: &Tx, margin_ratio: f64) -> Verdict {
    a5_inner(tx, margin_ratio, true)
}

fn a5_inner(tx: &Tx, margin_ratio: f64, blind: bool) -> Verdict {
    let candidates: Vec<usize> = if blind {
        (0..tx.outputs.len()).collect()
    } else {
        tx.change_indices()
    };
    let mut scored: Vec<(u64, usize, usize)> = Vec::new();
    for (ii, input) in tx.inputs.iter().enumerate() {
        for ci in candidates.iter().copied() {
            let out = &tx.outputs[ci];
            let d = input.value.abs_diff(out.value);
            scored.push((d, ii, ci));
        }
    }
    scored.sort();
    if scored.is_empty() {
        return Verdict::NoAnswer;
    }
    if scored.len() == 1 {
        return Verdict::Pairing { input: scored[0].1, change: scored[0].2 };
    }
    let best = scored[0];
    // The runner-up must involve a different input, otherwise it is the same
    // hypothesis about who the contributor is.
    let rival = scored.iter().skip(1).find(|c| c.1 != best.1);
    match rival {
        None => Verdict::Pairing { input: best.1, change: best.2 },
        Some(r) => {
            if (best.0 as f64) < (r.0 as f64) * margin_ratio {
                Verdict::Pairing { input: best.1, change: best.2 }
            } else {
                Verdict::Ambiguous
            }
        }
    }
}
