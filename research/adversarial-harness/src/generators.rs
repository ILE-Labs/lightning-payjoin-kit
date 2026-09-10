//! Two constructions, generated under the same UTXO draws so that the only
//! difference between the samples is the construction itself.

use crate::rng::Rng;
use crate::tx::{Input, Output, Owner, ScriptKind, Tx};

/// Weight constants the current construction uses, from src/psbt/builder.rs.
const P2WPKH_INPUT_VBYTES: u64 = 68;
const P2WPKH_OUTPUT_VBYTES: u64 = 31;
const TX_OVERHEAD_VBYTES: u64 = 10;
pub const CONTRIBUTOR_VBYTES: u64 = P2WPKH_INPUT_VBYTES + P2WPKH_OUTPUT_VBYTES; // 99

fn estimated_fee(inputs: usize, outputs: usize, feerate: u64) -> u64 {
    (TX_OVERHEAD_VBYTES
        + P2WPKH_INPUT_VBYTES * inputs as u64
        + P2WPKH_OUTPUT_VBYTES * outputs as u64)
        * feerate
}

/// Construction A — the library as it stands today.
///
/// Fee apportionment is fixed at 99 x feerate; output order is fixed as
/// [funding, initiator change, contributor change]; nLockTime is 0 and every
/// nSequence is 0xffffffff.
pub fn construct_current(
    initiator_utxo: u64,
    contributor_utxo: u64,
    channel: u64,
    feerate: u64,
) -> Option<Tx> {
    let base_fee = estimated_fee(1, 2, feerate);
    let init_change = initiator_utxo.checked_sub(channel + base_fee)?;
    if init_change < 546 {
        return None;
    }
    let contrib_fee = CONTRIBUTOR_VBYTES * feerate;
    let contrib_change = contributor_utxo.checked_sub(contrib_fee)?;
    if contrib_change < 546 {
        return None;
    }
    Some(Tx {
        inputs: vec![
            Input { value: initiator_utxo, owner: Owner::Initiator, sequence: 0xffff_ffff },
            Input { value: contributor_utxo, owner: Owner::Contributor, sequence: 0xffff_ffff },
        ],
        outputs: vec![
            Output { value: channel, kind: ScriptKind::P2wsh, owner: None },
            Output { value: init_change, kind: ScriptKind::P2wpkh, owner: Some(Owner::Initiator) },
            Output {
                value: contrib_change,
                kind: ScriptKind::P2wpkh,
                owner: Some(Owner::Contributor),
            },
        ],
        lock_time: 0,
        funding_index: 0,
        target_feerate: feerate,
    })
}

/// Construction B — the remediated construction the MVP build scope describes:
/// B2 randomised fee apportionment, B3 randomised output ordering, B4
/// anti-fee-sniping locktime and RBF-signalling sequences, B5 correct P2WSH
/// weight.
///
/// Fee apportionment is randomised by drawing the contributor's share uniformly
/// from a window around the true marginal cost, so the difference between their
/// input and their change carries no fixed arithmetic relationship. The
/// initiator absorbs the remainder.
pub fn construct_remediated(
    initiator_utxo: u64,
    contributor_utxo: u64,
    channel: u64,
    feerate: u64,
    tip_height: u32,
    rng: &mut Rng,
    jitter_pct: u64,
) -> Option<Tx> {
    // B5: charge the funding output its real 43 bytes, and account for the
    // segwit marker and flag.
    let true_vsize = {
        let nonwit = TX_OVERHEAD_VBYTES + 41 * 2 + 43 + 31 + 31;
        let wit = 2 + 108 * 2;
        (nonwit * 4 + wit).div_ceil(4)
    };
    let total_fee = true_vsize * feerate;

    // B2: the contributor's share of the fee is drawn at random rather than
    // being a fixed multiple of the feerate. The window is centred on their
    // marginal cost and is wide enough that the residue reveals no exact rate.
    let nominal = CONTRIBUTOR_VBYTES * feerate;
    let span = nominal * jitter_pct / 100;
    let contrib_fee = if span == 0 {
        nominal
    } else {
        rng.range(nominal.saturating_sub(span), nominal + span)
    };
    let contrib_fee = contrib_fee.min(total_fee);
    let init_fee = total_fee - contrib_fee;

    let init_change = initiator_utxo.checked_sub(channel + init_fee)?;
    if init_change < 546 {
        return None;
    }
    let contrib_change = contributor_utxo.checked_sub(contrib_fee)?;
    if contrib_change < 546 {
        return None;
    }

    // B3: shuffle the outputs. BOLT 2 achieves the same by sorting on a random
    // serial_id; the effect is identical and the funding output lands anywhere.
    let mut outputs = vec![
        Output { value: channel, kind: ScriptKind::P2wsh, owner: None },
        Output { value: init_change, kind: ScriptKind::P2wpkh, owner: Some(Owner::Initiator) },
        Output { value: contrib_change, kind: ScriptKind::P2wpkh, owner: Some(Owner::Contributor) },
    ];
    rng.shuffle(&mut outputs);
    let funding_index = outputs.iter().position(|o| o.owner.is_none()).unwrap();

    let mut inputs = vec![
        // B4: 0xfffffffd, the value BOLT 2 requires for interactive construction
        // and the value Bitcoin Core uses when signalling RBF.
        Input { value: initiator_utxo, owner: Owner::Initiator, sequence: 0xffff_fffd },
        Input { value: contributor_utxo, owner: Owner::Contributor, sequence: 0xffff_fffd },
    ];
    rng.shuffle(&mut inputs);

    // B4: anti-fee-sniping, following Bitcoin Core's DiscourageFeeSniping —
    // nLockTime at the tip, and one time in ten a uniform step back up to 100.
    let lock_time = if rng.below(10) == 0 {
        tip_height.saturating_sub(rng.below(100) as u32)
    } else {
        tip_height
    };

    Some(Tx { inputs, outputs, lock_time, funding_index, target_feerate: feerate })
}

/// UTXO value draw. Log-uniform over the stated range: this is an **assumption**,
/// not a measurement, and the range is a parameter so that a reviewer can
/// substitute a distribution they can defend.
pub fn draw_utxo(rng: &mut Rng, min_sat: u64, max_sat: u64) -> u64 {
    let lo = (min_sat as f64).ln();
    let hi = (max_sat as f64).ln();
    let u = rng.next_u64() as f64 / u64::MAX as f64;
    (lo + u * (hi - lo)).exp() as u64
}

/// Construction C — the pivot named in trigger T1: the contributed value is
/// absorbed into the funding output instead of being returned as change.
///
/// The contributor genuinely funds part of the channel, as BOLT 2 dual funding
/// does, and takes change only on the remainder of their UTXO. There is no longer
/// an output that mirrors their input, so the near-equality that construction A
/// and B both leave behind does not exist.
///
/// Generated here so that the pivot can be evaluated before it is needed rather
/// than after.
pub fn construct_absorbed(
    initiator_utxo: u64,
    contributor_utxo: u64,
    channel: u64,
    feerate: u64,
    tip_height: u32,
    rng: &mut Rng,
) -> Option<Tx> {
    let true_vsize = {
        let nonwit = TX_OVERHEAD_VBYTES + 41 * 2 + 43 + 31 + 31;
        let wit = 2 + 108 * 2;
        (nonwit * 4 + wit).div_ceil(4)
    };
    let total_fee = true_vsize * feerate;

    // The contributor puts a random share of the channel value in, between 20%
    // and 80%, capped by what their UTXO can afford.
    let want = channel * rng.range(20, 80) / 100;
    let affordable = contributor_utxo.saturating_sub(total_fee / 2 + 546);
    let contrib_funding = want.min(affordable);
    if contrib_funding == 0 {
        return None;
    }
    let contrib_fee = total_fee / 2;
    let init_funding = channel - contrib_funding;

    let contrib_change = contributor_utxo.checked_sub(contrib_funding + contrib_fee)?;
    let init_change = initiator_utxo.checked_sub(init_funding + (total_fee - contrib_fee))?;
    if contrib_change < 546 || init_change < 546 {
        return None;
    }

    let mut outputs = vec![
        Output { value: channel, kind: ScriptKind::P2wsh, owner: None },
        Output { value: init_change, kind: ScriptKind::P2wpkh, owner: Some(Owner::Initiator) },
        Output { value: contrib_change, kind: ScriptKind::P2wpkh, owner: Some(Owner::Contributor) },
    ];
    rng.shuffle(&mut outputs);
    let funding_index = outputs.iter().position(|o| o.owner.is_none()).unwrap();

    let mut inputs = vec![
        Input { value: initiator_utxo, owner: Owner::Initiator, sequence: 0xffff_fffd },
        Input { value: contributor_utxo, owner: Owner::Contributor, sequence: 0xffff_fffd },
    ];
    rng.shuffle(&mut inputs);

    let lock_time = if rng.below(10) == 0 {
        tip_height.saturating_sub(rng.below(100) as u32)
    } else {
        tip_height
    };

    Some(Tx { inputs, outputs, lock_time, funding_index, target_feerate: feerate })
}
