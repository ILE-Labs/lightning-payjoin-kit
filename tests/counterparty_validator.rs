use std::str::FromStr;

use bitcoin::{Amount, OutPoint, ScriptBuf, Txid};
use lightning_payjoin_kit::error::Error;
use lightning_payjoin_kit::payjoin::{CounterpartyOriginalValidator, ProposalValidator};
use lightning_payjoin_kit::psbt::FundingPsbtBuilder;
use lightning_payjoin_kit::wallet::Utxo;

fn test_utxo(value_sats: u64, txid: &str, vout: u32) -> Utxo {
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

fn original_psbt() -> bitcoin::Psbt {
    FundingPsbtBuilder::new(Amount::from_sat(1_000_000), ScriptBuf::new())
        .with_fee_rate_sat_vb(2)
        .build_fallback(
            &[test_utxo(
                1_100_000,
                "1111111111111111111111111111111111111111111111111111111111111111",
                0,
            )],
            ScriptBuf::new(),
        )
        .expect("fallback")
        .psbt
}

#[test]
fn counterparty_validator_accepts_safe_original() {
    let original = original_psbt();
    let counterparty_utxo = test_utxo(
        200_000,
        "2222222222222222222222222222222222222222222222222222222222222222",
        0,
    );

    let validation = CounterpartyOriginalValidator::new(
        &original,
        &counterparty_utxo,
        Amount::from_sat(1_000_000),
        Amount::from_sat(1_000),
        Amount::from_sat(198),
    )
    .validate()
    .expect("valid original");

    assert!(validation.accepted);
    assert_eq!(validation.added_inputs, 0);
    assert_eq!(validation.added_outputs, 0);
    assert!(validation.added_fee > Amount::ZERO);
}

#[test]
fn counterparty_validator_rejects_wrong_channel_value() {
    let original = original_psbt();
    let counterparty_utxo = test_utxo(
        200_000,
        "3333333333333333333333333333333333333333333333333333333333333333",
        0,
    );

    let error = CounterpartyOriginalValidator::new(
        &original,
        &counterparty_utxo,
        Amount::from_sat(999_999),
        Amount::from_sat(1_000),
        Amount::from_sat(198),
    )
    .validate()
    .expect_err("wrong channel value");

    assert!(matches!(error, Error::InvalidProposal(_)));
}

#[test]
fn counterparty_validator_rejects_reused_counterparty_input() {
    let original = original_psbt();
    let reused_utxo = test_utxo(
        1_100_000,
        "1111111111111111111111111111111111111111111111111111111111111111",
        0,
    );

    let error = CounterpartyOriginalValidator::new(
        &original,
        &reused_utxo,
        Amount::from_sat(1_000_000),
        Amount::from_sat(1_000),
        Amount::from_sat(198),
    )
    .validate()
    .expect_err("reused input");

    assert!(matches!(error, Error::InvalidProposal(_)));
}

#[test]
fn counterparty_validator_rejects_fee_contribution_above_policy() {
    let original = original_psbt();
    let counterparty_utxo = test_utxo(
        200_000,
        "4444444444444444444444444444444444444444444444444444444444444444",
        0,
    );

    let error = CounterpartyOriginalValidator::new(
        &original,
        &counterparty_utxo,
        Amount::from_sat(1_000_000),
        Amount::from_sat(197),
        Amount::from_sat(198),
    )
    .validate()
    .expect_err("fee policy");

    assert!(matches!(error, Error::Policy(_)));
}

#[test]
fn counterparty_validator_rejects_dust_change() {
    let original = original_psbt();
    let counterparty_utxo = test_utxo(
        700,
        "5555555555555555555555555555555555555555555555555555555555555555",
        0,
    );

    let error = CounterpartyOriginalValidator::new(
        &original,
        &counterparty_utxo,
        Amount::from_sat(1_000_000),
        Amount::from_sat(198),
        Amount::from_sat(198),
    )
    .validate()
    .expect_err("dust change");

    assert!(matches!(error, Error::Policy(_)));
}
