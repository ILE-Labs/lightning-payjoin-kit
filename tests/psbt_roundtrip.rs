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

fn test_utxo_with_txid(value_sats: u64, txid: &str, vout: u32) -> Utxo {
    Utxo {
        outpoint: OutPoint {
            txid: Txid::from_str(txid).expect("txid"),
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

#[test]
fn builds_privacy_input_proposal_without_changing_funding_output() {
    let builder = FundingPsbtBuilder::new(Amount::from_sat(1_000_000), ScriptBuf::new())
        .with_fee_rate_sat_vb(2);
    let fallback = builder
        .build_fallback(&[test_utxo(1_100_000, 0)], ScriptBuf::new())
        .expect("fallback funding");
    let counterparty_utxo = test_utxo_with_txid(
        200_000,
        "2222222222222222222222222222222222222222222222222222222222222222",
        0,
    );

    let proposal = builder
        .build_privacy_input_proposal(
            &fallback.psbt,
            counterparty_utxo,
            ScriptBuf::new(),
            Amount::from_sat(1_000),
        )
        .expect("privacy input proposal");

    let original_tx = &fallback.psbt.unsigned_tx;
    let proposal_tx = &proposal.psbt.unsigned_tx;
    assert_eq!(proposal_tx.input.len(), original_tx.input.len() + 1);
    assert_eq!(proposal_tx.output.len(), original_tx.output.len() + 1);
    assert_eq!(proposal_tx.output[0], original_tx.output[0]);
    assert_eq!(proposal.counterparty_input_index, original_tx.input.len());
    assert_eq!(
        proposal.counterparty_change_output_index,
        original_tx.output.len()
    );
    assert_eq!(
        proposal.counterparty_fee_contribution,
        Amount::from_sat(198)
    );
    assert_eq!(
        proposal_tx.output[proposal.counterparty_change_output_index].value,
        Amount::from_sat(199_802)
    );
}

#[test]
fn privacy_input_proposal_preserves_value_accounting() {
    let builder = FundingPsbtBuilder::new(Amount::from_sat(1_000_000), ScriptBuf::new());
    let fallback = builder
        .build_fallback(&[test_utxo(1_100_000, 0)], ScriptBuf::new())
        .expect("fallback funding");
    let counterparty_utxo = test_utxo_with_txid(
        200_000,
        "3333333333333333333333333333333333333333333333333333333333333333",
        0,
    );

    let proposal = builder
        .build_privacy_input_proposal(
            &fallback.psbt,
            counterparty_utxo,
            ScriptBuf::new(),
            Amount::from_sat(1_000),
        )
        .expect("privacy input proposal");

    let input_value = proposal
        .psbt
        .inputs
        .iter()
        .map(|input| input.witness_utxo.as_ref().unwrap().value.to_sat())
        .sum::<u64>();
    let output_value = proposal
        .psbt
        .unsigned_tx
        .output
        .iter()
        .map(|output| output.value.to_sat())
        .sum::<u64>();
    assert_eq!(
        input_value,
        output_value + fallback.fee.to_sat() + proposal.counterparty_fee_contribution.to_sat()
    );
}

#[test]
fn rejects_privacy_input_when_fee_contribution_is_too_low() {
    let builder = FundingPsbtBuilder::new(Amount::from_sat(1_000_000), ScriptBuf::new())
        .with_fee_rate_sat_vb(2);
    let fallback = builder
        .build_fallback(&[test_utxo(1_100_000, 0)], ScriptBuf::new())
        .expect("fallback funding");
    let counterparty_utxo = test_utxo_with_txid(
        200_000,
        "4444444444444444444444444444444444444444444444444444444444444444",
        0,
    );

    let error = builder
        .build_privacy_input_proposal(
            &fallback.psbt,
            counterparty_utxo,
            ScriptBuf::new(),
            Amount::from_sat(197),
        )
        .expect_err("fee contribution too low");

    assert!(matches!(error, Error::Policy(_)));
}

#[test]
fn rejects_privacy_input_when_channel_output_was_modified() {
    let builder = FundingPsbtBuilder::new(Amount::from_sat(1_000_000), ScriptBuf::new());
    let fallback = builder
        .build_fallback(&[test_utxo(1_100_000, 0)], ScriptBuf::new())
        .expect("fallback funding");
    let mut tampered = fallback.psbt;
    tampered.unsigned_tx.output[0].value = Amount::from_sat(999_999);
    let counterparty_utxo = test_utxo_with_txid(
        200_000,
        "5555555555555555555555555555555555555555555555555555555555555555",
        0,
    );

    let error = builder
        .build_privacy_input_proposal(
            &tampered,
            counterparty_utxo,
            ScriptBuf::new(),
            Amount::from_sat(1_000),
        )
        .expect_err("modified funding output");

    assert!(matches!(error, Error::InvalidProposal(_)));
}
