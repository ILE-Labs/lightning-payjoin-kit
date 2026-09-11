//! Experiment 01 — deterministic fee residue (R-P1), weight estimation (R-P4),
//! and the marginal cost of the collaborative construction (documentation claim 7).
//!
//! Measures the library as it stands. Does not modify it: the crate under test is
//! a path dependency and is compiled unmodified.

use bitcoin::{
    hashes::Hash, Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxOut, Txid, Witness,
};
use lightning_payjoin_kit::psbt::FundingPsbtBuilder;
use lightning_payjoin_kit::wallet::Utxo;

/// Deterministic outpoint generator. No RNG anywhere in this experiment, so every
/// number below is reproducible byte-for-byte.
fn outpoint(seed: u8, vout: u32) -> OutPoint {
    let mut raw = [0u8; 32];
    raw[0] = seed;
    raw[31] = vout as u8;
    OutPoint { txid: Txid::from_byte_array(raw), vout }
}

fn p2wpkh_spk(seed: u8) -> ScriptBuf {
    let mut h = [0u8; 20];
    h[0] = seed;
    ScriptBuf::new_p2wpkh(&bitcoin::WPubkeyHash::from_byte_array(h))
}

fn p2wsh_spk(seed: u8) -> ScriptBuf {
    let mut h = [0u8; 32];
    h[0] = seed;
    ScriptBuf::new_p2wsh(&bitcoin::WScriptHash::from_byte_array(h))
}

fn utxo(seed: u8, vout: u32, sats: u64) -> Utxo {
    Utxo {
        outpoint: outpoint(seed, vout),
        value: Amount::from_sat(sats),
        script_pubkey: p2wpkh_spk(seed),
        confirmed: true,
    }
}

/// Serialized size of a TxOut, measured rather than assumed.
fn txout_serialized_len(spk: &ScriptBuf) -> usize {
    use bitcoin::consensus::Encodable;
    let out = TxOut { value: Amount::from_sat(1_000), script_pubkey: spk.clone() };
    let mut buf = Vec::new();
    out.consensus_encode(&mut buf).unwrap();
    buf.len()
}

/// Witness-populated clone used only to measure realistic vsize. A P2WPKH spend
/// witness is [signature, pubkey]; we use the maximum-length DER signature (72
/// bytes incl. sighash byte) so the figure is an upper bound, matching the way
/// wallets estimate.
fn with_dummy_witnesses(tx: &Transaction) -> Transaction {
    let mut tx = tx.clone();
    for input in tx.input.iter_mut() {
        input.witness = Witness::from_slice(&[vec![0u8; 72], vec![0u8; 33]]);
    }
    tx
}

fn main() {
    println!("# Experiment 01 — fee residue, weight estimation, marginal cost");
    println!("# crate under test: lightning-payjoin-kit (path dependency, unmodified)");
    println!();

    // ---------------------------------------------------------------
    // Part A. Output serialized sizes, measured.
    // ---------------------------------------------------------------
    println!("## A. Measured TxOut serialized sizes (bytes)");
    let p2wpkh_out = txout_serialized_len(&p2wpkh_spk(1));
    let p2wsh_out = txout_serialized_len(&p2wsh_spk(1));
    println!("P2WPKH TxOut = {p2wpkh_out}");
    println!("P2WSH  TxOut = {p2wsh_out}");
    println!("library constant P2WPKH_OUTPUT_VBYTES = 31 (src/psbt/builder.rs:9)");
    println!("delta charged-vs-actual for the P2WSH funding output = {}", p2wsh_out as i64 - 31);
    println!();

    // ---------------------------------------------------------------
    // Part B. R-P1 — is the residue exactly 99 x feerate?
    // ---------------------------------------------------------------
    println!("## B. R-P1 deterministic fee residue");
    println!("contrib_value  feerate  contrib_change  residue  99*feerate  residue==99r  r_recovered");
    let channel = Amount::from_sat(1_000_000);
    let funding_spk = p2wsh_spk(9);
    let mut rp1_total = 0usize;
    let mut rp1_holds = 0usize;
    let mut rp1_rejected = 0usize;
    for &feerate in &[1u64, 2, 3, 5, 8, 13, 21, 50, 100, 300] {
        for &contrib in &[20_000u64, 55_000, 199_999, 200_000, 1_234_567, 5_000_000] {
            let builder = FundingPsbtBuilder::new(channel, funding_spk.clone())
                .with_fee_rate_sat_vb(feerate);
            let initiator = vec![utxo(1, 0, 1_100_000)];
            let fallback = builder.build_fallback(&initiator, p2wpkh_spk(2)).unwrap();
            let proposal = match builder.build_privacy_input_proposal(
                &fallback.psbt,
                utxo(3, 0, contrib),
                p2wpkh_spk(4),
                Amount::from_sat(u64::MAX / 2),
            ) {
                Ok(p) => p,
                Err(e) => {
                    rp1_total += 1;
                    rp1_rejected += 1;
                    println!(
                        "{contrib:>13}  {feerate:>7}  {:>14}  {:>7}  {:>10}  {:>12}  {:>11}   REJECTED: {e}",
                        "-", "-", 99 * feerate, "n/a", "-"
                    );
                    continue;
                }
            };
            let change = proposal.psbt.unsigned_tx.output
                [proposal.counterparty_change_output_index]
                .value
                .to_sat();
            let residue = contrib - change;
            let expected = 99 * feerate;
            let holds = residue == expected;
            rp1_total += 1;
            if holds { rp1_holds += 1; }
            // An observer recovers the fee rate from the transaction itself, then
            // divides the residue by it.
            let r_recovered = residue / 99;
            println!(
                "{contrib:>13}  {feerate:>7}  {change:>14}  {residue:>7}  {expected:>10}  {holds:>12}  {r_recovered:>11}"
            );
        }
    }
    println!();
    println!("R-P1 residue == 99*feerate in {rp1_holds}/{rp1_total} sampled configurations");
    println!("({rp1_rejected}/{rp1_total} configurations were rejected by library policy before a transaction existed)");
    let constructed = rp1_total - rp1_rejected;
    println!("of the {constructed} configurations that produced a transaction, the residue held in {rp1_holds}");
    println!();

    // ---------------------------------------------------------------
    // Part C. R-P4 — effective fee rate against target.
    // ---------------------------------------------------------------
    println!("## C. R-P4 fee accuracy (V4 target: within 5% of requested rate)");
    println!("feerate  tx_vsize  fee_paid  effective_rate  pct_of_target  within_5pct");
    let mut v4_pass = 0usize;
    let mut v4_total = 0usize;
    for &feerate in &[1u64, 2, 5, 10, 25, 50, 100] {
        let builder = FundingPsbtBuilder::new(channel, funding_spk.clone())
            .with_fee_rate_sat_vb(feerate);
        let initiator = vec![utxo(1, 0, 1_100_000)];
        let fallback = builder.build_fallback(&initiator, p2wpkh_spk(2)).unwrap();
        let proposal = builder
            .build_privacy_input_proposal(
                &fallback.psbt,
                utxo(3, 0, 200_000),
                p2wpkh_spk(4),
                Amount::from_sat(u64::MAX / 2),
            )
            .unwrap();
        let tx = &proposal.psbt.unsigned_tx;
        let inputs: u64 = proposal
            .psbt
            .inputs
            .iter()
            .map(|i| i.witness_utxo.as_ref().unwrap().value.to_sat())
            .sum();
        let outputs: u64 = tx.output.iter().map(|o| o.value.to_sat()).sum();
        let fee = inputs - outputs;
        let vsize = with_dummy_witnesses(tx).vsize() as u64;
        let effective = fee as f64 / vsize as f64;
        let pct = effective / feerate as f64 * 100.0;
        let within = (pct - 100.0).abs() <= 5.0;
        v4_total += 1;
        if within { v4_pass += 1; }
        println!(
            "{feerate:>7}  {vsize:>8}  {fee:>8}  {effective:>14.4}  {pct:>12.2}%  {within:>11}"
        );
    }
    println!();
    println!("V4 (within 5% of target) holds in {v4_pass}/{v4_total} sampled rates");
    println!();

    // ---------------------------------------------------------------
    // Part D. Marginal cost of the collaborative construction.
    // ---------------------------------------------------------------
    println!("## D. Marginal on-chain cost vs the single-funder fallback (doc claim 7)");
    let builder = FundingPsbtBuilder::new(channel, funding_spk.clone()).with_fee_rate_sat_vb(1);
    let initiator = vec![utxo(1, 0, 1_100_000)];
    let fallback = builder.build_fallback(&initiator, p2wpkh_spk(2)).unwrap();
    let proposal = builder
        .build_privacy_input_proposal(
            &fallback.psbt,
            utxo(3, 0, 200_000),
            p2wpkh_spk(4),
            Amount::from_sat(u64::MAX / 2),
        )
        .unwrap();
    let base_vsize = with_dummy_witnesses(&fallback.psbt.unsigned_tx).vsize();
    let coll_vsize = with_dummy_witnesses(&proposal.psbt.unsigned_tx).vsize();
    println!("single-funder fallback vsize   = {base_vsize}");
    println!("collaborative proposal vsize   = {coll_vsize}");
    println!("measured marginal vsize        = {}", coll_vsize as i64 - base_vsize as i64);
    println!("library charges the contributor  = 99 vbytes x feerate");
    println!(
        "under-charge per collaborative tx = {} vbytes x feerate",
        (coll_vsize as i64 - base_vsize as i64) - 99
    );
    println!();

    // ---------------------------------------------------------------
    // Part E. Structural constants of the produced transaction (R-P2, R-P3).
    // ---------------------------------------------------------------
    println!("## E. Structural fields as produced (R-P2, R-P3)");
    let tx = &proposal.psbt.unsigned_tx;
    println!("version    = {}", tx.version.0);
    println!("lock_time  = {}", tx.lock_time.to_consensus_u32());
    println!("n_inputs   = {}   n_outputs = {}", tx.input.len(), tx.output.len());
    for (i, txin) in tx.input.iter().enumerate() {
        println!("input[{i}].sequence = 0x{:08x}", txin.sequence.0);
    }
    for (i, txout) in tx.output.iter().enumerate() {
        let kind = if txout.script_pubkey.is_p2wsh() { "P2WSH (funding)" } else { "P2WPKH (change)" };
        println!("output[{i}] = {:>10} sat  {kind}", txout.value.to_sat());
    }
    println!("funding_output_index (fallback) = {}", fallback.funding_output_index);
    println!("contributor_input_index         = {}", proposal.counterparty_input_index);
    println!("contributor_change_output_index = {}", proposal.counterparty_change_output_index);
    println!("all sequences == Sequence::MAX  = {}", tx.input.iter().all(|i| i.sequence == Sequence::MAX));
}
