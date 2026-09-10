//! Adversarial partitioning harness for collaborative Lightning funding
//! transactions.
//!
//! Runs four attacks against two constructions over a sample of drawn UTXO pairs
//! and reports, for each, how often the adversary correctly pairs the
//! contributor's input with the contributor's change output.
//!
//! Reproduce:  cargo run --release -- --seed 20260910 --samples 20000
//!
//! The chance baseline is 50%: a two-input, two-change transaction admits exactly
//! two pairings and an adversary guessing at random is right half the time.

mod generators;
mod heuristics;
mod rng;
mod tx;

use generators::{
    construct_absorbed, construct_current, construct_remediated, draw_utxo, CONTRIBUTOR_VBYTES,
};
use heuristics::{
    a1_fee_residue, a2_blind_subset_sum, a2_subset_sum, a3_uih, a4_fingerprint,
    a5_blind_near_equality, a5_near_equality, Verdict,
};
use rng::Rng;
use tx::{Owner, Tx};

#[derive(Default)]
struct Score {
    committed_correct: usize,
    committed_wrong: usize,
    ambiguous: usize,
    no_answer: usize,
}

impl Score {
    fn record(&mut self, tx: &Tx, v: Verdict) {
        match v {
            Verdict::Pairing { input, change } => {
                let right = tx.inputs[input].owner == Owner::Contributor
                    && tx.outputs[change].owner == Some(Owner::Contributor);
                if right {
                    self.committed_correct += 1;
                } else {
                    self.committed_wrong += 1;
                }
            }
            Verdict::Ambiguous => self.ambiguous += 1,
            Verdict::NoAnswer => self.no_answer += 1,
        }
    }

    fn total(&self) -> usize {
        self.committed_correct + self.committed_wrong + self.ambiguous + self.no_answer
    }

    /// Attack success rate. An attack that declines to answer is scored as a coin
    /// flip, because an adversary who cannot distinguish the two pairings still
    /// gets one right half the time by guessing. This is the conservative
    /// accounting: it can only make a construction look worse, never better.
    fn success_rate(&self) -> f64 {
        let undecided = (self.ambiguous + self.no_answer) as f64 * 0.5;
        (self.committed_correct as f64 + undecided) / self.total() as f64
    }

    fn line(&self, name: &str) -> String {
        format!(
            "{name:<34} correct {:>6}  wrong {:>6}  ambiguous {:>6}  no-answer {:>6}  success {:>7.3}%",
            self.committed_correct,
            self.committed_wrong,
            self.ambiguous,
            self.no_answer,
            self.success_rate() * 100.0
        )
    }
}

#[derive(Default)]
struct CandidateTally {
    n: usize,
    sum: usize,
    zero: usize,
    one: usize,
    two: usize,
    three_plus: usize,
}

impl CandidateTally {
    fn record(&mut self, k: usize) {
        self.n += 1;
        self.sum += k;
        match k {
            0 => self.zero += 1,
            1 => self.one += 1,
            2 => self.two += 1,
            _ => self.three_plus += 1,
        }
    }
    fn line(&self, name: &str) -> String {
        format!(
            "{name:<40} mean {:>5.3}   none {:>6.2}%   exactly one {:>6.2}%   two {:>6.2}%   three+ {:>6.2}%",
            self.sum as f64 / self.n as f64,
            self.zero as f64 / self.n as f64 * 100.0,
            self.one as f64 / self.n as f64 * 100.0,
            self.two as f64 / self.n as f64 * 100.0,
            self.three_plus as f64 / self.n as f64 * 100.0
        )
    }
}

#[derive(Default)]
struct UihTally {
    n: usize,
    blocksci_uih2: usize,
    blockstream_uih2: usize,
    gibson_uih2: usize,
}

fn arg(name: &str, default: u64) -> u64 {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let seed = arg("--seed", 20_260_910);
    let samples = arg("--samples", 20_000) as usize;
    let min_utxo = arg("--min-utxo", 20_000);
    let max_utxo = arg("--max-utxo", 50_000_000);
    let jitter = arg("--jitter-pct", 60);

    println!("# adversarial partitioning harness");
    println!("# seed        = {seed}");
    println!("# samples     = {samples}");
    println!("# utxo draw   = log-uniform over [{min_utxo}, {max_utxo}] sat  (ASSUMPTION, see analysis.md)");
    println!("# fee jitter  = +/-{jitter}% of the contributor's marginal cost, construction B only");
    println!("# chance      = 50.000% (two pairings, one contributor)");
    println!();

    let mut rng = Rng::new(seed);

    let (mut a1_cur, mut a2_cur) = (Score::default(), Score::default());
    let (mut a1_rem, mut a2_rem) = (Score::default(), Score::default());
    let (mut a2b_cur, mut a2b_rem, mut a2b_abs) =
        (Score::default(), Score::default(), Score::default());
    let (mut tp_rem, mut tp_abs) = (Score::default(), Score::default());
    let (mut a5b_rem, mut a5b_abs) = (Score::default(), Score::default());
    let (mut ct_rem, mut ct_abs) = (CandidateTally::default(), CandidateTally::default());
    let (mut a5_cur, mut a5_rem, mut a5_abs) =
        (Score::default(), Score::default(), Score::default());
    let mut a2_abs = Score::default();
    let mut abs_built = 0usize;
    let (mut uih_cur, mut uih_rem) = (UihTally::default(), UihTally::default());
    let (mut fp_cur_seq, mut fp_cur_lt, mut fp_cur_idx0, mut fp_cur_mix) = (0usize, 0usize, 0usize, 0usize);
    let (mut fp_rem_seq, mut fp_rem_lt, mut fp_rem_idx0, mut fp_rem_mix) = (0usize, 0usize, 0usize, 0usize);
    let mut built = 0usize;
    let mut skipped = 0usize;

    let feerates: [u64; 6] = [1, 2, 5, 10, 25, 60];
    let channels: [u64; 4] = [500_000, 1_000_000, 2_000_000, 5_000_000];

    for _ in 0..samples {
        let feerate = feerates[rng.below(feerates.len() as u64) as usize];
        let channel = channels[rng.below(channels.len() as u64) as usize];
        let contributor_utxo = draw_utxo(&mut rng, min_utxo, max_utxo);
        // The initiator must be able to cover the channel; draw above it.
        let initiator_utxo = channel + draw_utxo(&mut rng, min_utxo, max_utxo);
        let tip = 900_000u32;

        let cur = construct_current(initiator_utxo, contributor_utxo, channel, feerate);
        let rem = construct_remediated(
            initiator_utxo,
            contributor_utxo,
            channel,
            feerate,
            tip,
            &mut rng,
            jitter,
        );

        let abs = construct_absorbed(
            initiator_utxo,
            contributor_utxo,
            channel,
            feerate,
            tip,
            &mut rng,
        );

        let (cur, rem) = match (cur, rem) {
            (Some(a), Some(b)) => (a, b),
            _ => {
                skipped += 1;
                continue;
            }
        };
        built += 1;

        if let Some(abs) = abs {
            abs_built += 1;
            a2_abs.record(&abs, a2_subset_sum(&abs, 0));
            let (v, _) = a2_blind_subset_sum(&abs, 0);
            a2b_abs.record(&abs, v);
            let t = abs.as_taproot();
            let (v, k) = a2_blind_subset_sum(&t, 0);
            tp_abs.record(&t, v);
            ct_abs.record(k);
            a5b_abs.record(&t, a5_blind_near_equality(&t, 0.25));
            a5_abs.record(&abs, a5_near_equality(&abs, 0.25));
        }

        a1_cur.record(&cur, a1_fee_residue(&cur, CONTRIBUTOR_VBYTES));
        a2_cur.record(&cur, a2_subset_sum(&cur, 0));
        a1_rem.record(&rem, a1_fee_residue(&rem, CONTRIBUTOR_VBYTES));
        a2_rem.record(&rem, a2_subset_sum(&rem, 0));
        let (v, _) = a2_blind_subset_sum(&cur, 0);
        a2b_cur.record(&cur, v);
        let (v, _) = a2_blind_subset_sum(&rem, 0);
        a2b_rem.record(&rem, v);
        let t = rem.as_taproot();
        let (v, k) = a2_blind_subset_sum(&t, 0);
        tp_rem.record(&t, v);
        ct_rem.record(k);
        a5b_rem.record(&t, a5_blind_near_equality(&t, 0.25));
        a5_cur.record(&cur, a5_near_equality(&cur, 0.25));
        a5_rem.record(&rem, a5_near_equality(&rem, 0.25));

        for (tx, tally) in [(&cur, &mut uih_cur), (&rem, &mut uih_rem)] {
            let f = a3_uih(tx);
            tally.n += 1;
            tally.blocksci_uih2 += f.blocksci_uih2 as usize;
            tally.blockstream_uih2 += f.blockstream_uih2 as usize;
            tally.gibson_uih2 += f.gibson_uih2 as usize;
        }

        let f = a4_fingerprint(&cur);
        fp_cur_seq += f.all_sequences_final as usize;
        fp_cur_lt += f.locktime_zero as usize;
        fp_cur_idx0 += f.funding_output_first as usize;
        fp_cur_mix += f.mixed_sequences as usize;
        let f = a4_fingerprint(&rem);
        fp_rem_seq += f.all_sequences_final as usize;
        fp_rem_lt += f.locktime_zero as usize;
        fp_rem_idx0 += f.funding_output_first as usize;
        fp_rem_mix += f.mixed_sequences as usize;
    }

    println!("built {built} transaction pairs; {skipped} draws rejected by dust or affordability");
    println!();
    println!("## A1 — fee-residue attack (input - change == k x feerate)");
    println!("{}", a1_cur.line("construction A (current)"));
    println!("{}", a1_rem.line("construction B (remediated)"));
    println!();
    println!("## A2 — subset-sum partitioning");
    println!("{}", a2_cur.line("construction A (current)"));
    println!("{}", a2_rem.line("construction B (remediated)"));
    println!("{}", a2_abs.line("construction C (absorbed)"));
    println!();
    println!("## A2-blind — subset-sum with the channel output NOT identified");
    println!("## (the Profile A adversary, and the adversary against a taproot channel)");
    println!("{}", a2b_cur.line("construction A (current)"));
    println!("{}", a2b_rem.line("construction B (remediated)"));
    println!("{}", a2b_abs.line("construction C (absorbed)"));
    println!();
    println!("## A2-taproot — every output recast as P2TR, so the observer truly cannot");
    println!("## tell which output is the channel (simple taproot channels, unannounced)");
    println!("{}", tp_rem.line("construction B (remediated), taproot"));
    println!("{}", tp_abs.line("construction C (absorbed), taproot"));
    println!("{}", ct_rem.line("construction B (remediated), taproot"));
    println!("{}", ct_abs.line("construction C (absorbed), taproot"));
    println!();
    println!("## A5 — near-equality attack (no fee arithmetic used)");
    println!("{}", a5_cur.line("construction A (current)"));
    println!("{}", a5_rem.line("construction B (remediated)"));
    println!("{}", a5_abs.line("construction C (absorbed)"));
    println!();
    println!("## A5-blind — near-equality against all-P2TR outputs, channel not identified");
    println!("{}", a5b_rem.line("construction B (remediated), taproot"));
    println!("{}", a5b_abs.line("construction C (absorbed), taproot"));
    println!();
    println!("## A3 — unnecessary-input heuristic (share of sample flagged)");
    for (name, t) in [("construction A (current)", &uih_cur), ("construction B (remediated)", &uih_rem)] {
        println!(
            "{name:<34} BlockSci-UIH2 {:>6.2}%   BlockStream-UIH2 {:>6.2}%   Gibson-UIH2 {:>6.2}%",
            t.blocksci_uih2 as f64 / t.n as f64 * 100.0,
            t.blockstream_uih2 as f64 / t.n as f64 * 100.0,
            t.gibson_uih2 as f64 / t.n as f64 * 100.0
        );
    }
    println!();
    println!("## A4 — metadata fingerprint (share of sample)");
    println!("## mainnet reference: all-final 28.15%, nLockTime 0 95.34%, mixed sequences 0.12%");
    println!(
        "construction A (current)           all-nSequence-final {:>6.2}%   nLockTime==0 {:>6.2}%   funding at index 0 {:>6.2}%   mixed sequences {:>6.2}%",
        fp_cur_seq as f64 / built as f64 * 100.0,
        fp_cur_lt as f64 / built as f64 * 100.0,
        fp_cur_idx0 as f64 / built as f64 * 100.0,
        fp_cur_mix as f64 / built as f64 * 100.0
    );
    println!(
        "construction B (remediated)        all-nSequence-final {:>6.2}%   nLockTime==0 {:>6.2}%   funding at index 0 {:>6.2}%   mixed sequences {:>6.2}%",
        fp_rem_seq as f64 / built as f64 * 100.0,
        fp_rem_lt as f64 / built as f64 * 100.0,
        fp_rem_idx0 as f64 / built as f64 * 100.0,
        fp_rem_mix as f64 / built as f64 * 100.0
    );
    println!();
    println!("## V1 gate");
    let gate = |s: &Score| if s.success_rate() <= 0.55 { "PASS" } else { "FAIL" };
    println!("pass condition, stated before running: attack success <= 55% (chance 50% + 5pt margin)");
    println!(
        "construction A (current)     A1 {}   A2 {}   A2-blind {}   A5 {}",
        gate(&a1_cur), gate(&a2_cur), gate(&a2b_cur), gate(&a5_cur)
    );
    println!(
        "construction B (remediated)  A1 {}   A2 {}   A2-blind {}   A5 {}",
        gate(&a1_rem), gate(&a2_rem), gate(&a2b_rem), gate(&a5_rem)
    );
    println!(
        "construction C (absorbed)    A1 {}   A2 {}   A2-blind {}   A5 {}",
        "n/a ", gate(&a2_abs), gate(&a2b_abs), gate(&a5_abs)
    );
    println!(
        "taproot outputs:             B A2-taproot {}   B A5-blind {}   C A2-taproot {}   C A5-blind {}",
        gate(&tp_rem), gate(&a5b_rem), gate(&tp_abs), gate(&a5b_abs)
    );
    println!();
    println!("construction C built in {abs_built}/{built} draws");
}
