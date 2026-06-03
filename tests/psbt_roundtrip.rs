use std::str::FromStr;

use bitcoin::{Amount, OutPoint, ScriptBuf, Txid};
use lightning_payjoin_kit::error::Error;
use lightning_payjoin_kit::psbt::FundingPsbtBuilder;
use lightning_payjoin_kit::wallet::Utxo;

fn test_utxo(value_sats: u64, vout: u32) -> Utxo {
    Utxo {
        outpoint: OutPoint {
            txid: Txid::from_str(
                "1111111111111111111111111111111111111111111111111111111111111111",
            )
            .expect("txid"),
            vout,
        },
        value: Amount::from_sat(value_sats),
        script_pubkey: ScriptBuf::new(),
        confirmed: true,
    }
}

#[test]
fn builds_empty_fallback_psbt_with_channel_output() {
    let builder = FundingPsbtBuilder::new(Amount::from_sat(1_000_000), ScriptBuf::new());

    let psbt = builder.build_empty_fallback().expect("fallback PSBT");

    assert_eq!(psbt.unsigned_tx.output.len(), 1);
    assert_eq!(
        psbt.unsigned_tx.output[0].value,
        Amount::from_sat(1_000_000)
    );
}

#[test]
fn builds_fallback_psbt_from_wallet_utxos() {
    let builder = FundingPsbtBuilder::new(Amount::from_sat(1_000_000), ScriptBuf::new())
        .with_fee_rate_sat_vb(2);

    let funding = builder
        .build_fallback(&[test_utxo(1_100_000, 0)], ScriptBuf::new())
        .expect("fallback funding");

    let tx = &funding.psbt.unsigned_tx;
    assert_eq!(tx.input.len(), 1);
    assert_eq!(tx.output.len(), 2);
    assert_eq!(tx.output[0].value, Amount::from_sat(1_000_000));
    assert_eq!(funding.change_output_index, Some(1));
    assert_eq!(funding.funding_output_index, 0);
    assert_eq!(
        funding.psbt.inputs[0].witness_utxo.as_ref().unwrap().value,
        Amount::from_sat(1_100_000)
    );

    let input_value = funding
        .selected_utxos
        .iter()
        .map(|utxo| utxo.value.to_sat())
        .sum::<u64>();
    let output_value = tx
        .output
        .iter()
        .map(|output| output.value.to_sat())
        .sum::<u64>();
    assert_eq!(input_value, output_value + funding.fee.to_sat());
}

#[test]
fn omits_dust_change_and_adds_it_to_fee() {
    let builder = FundingPsbtBuilder::new(Amount::from_sat(1_000_000), ScriptBuf::new());

    let funding = builder
        .build_fallback(&[test_utxo(1_000_200, 0)], ScriptBuf::new())
        .expect("fallback funding");

    assert_eq!(funding.psbt.unsigned_tx.output.len(), 1);
    assert_eq!(funding.change_output_index, None);
    assert_eq!(funding.fee, Amount::from_sat(200));
}

#[test]
fn rejects_insufficient_funds() {
    let builder = FundingPsbtBuilder::new(Amount::from_sat(1_000_000), ScriptBuf::new());

    let error = builder
        .build_fallback(&[test_utxo(999_000, 0)], ScriptBuf::new())
        .expect_err("insufficient funds");

    assert!(matches!(error, Error::InsufficientFunds { .. }));
}
