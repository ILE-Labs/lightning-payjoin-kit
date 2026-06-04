use std::str::FromStr;

use bitcoin::{Amount, OutPoint, ScriptBuf, Txid};
use lightning_payjoin_kit::error::Error;
use lightning_payjoin_kit::payjoin::{InitiatorProposalValidator, ProposalValidator};
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

fn original_and_proposal() -> (bitcoin::Psbt, bitcoin::Psbt) {
    let builder = FundingPsbtBuilder::new(Amount::from_sat(1_000_000), ScriptBuf::new())
        .with_fee_rate_sat_vb(2);
    let fallback = builder
        .build_fallback(
            &[test_utxo(
                1_100_000,
                "1111111111111111111111111111111111111111111111111111111111111111",
                0,
            )],
            ScriptBuf::new(),
        )
        .expect("fallback");
    let proposal = builder
        .build_privacy_input_proposal(
            &fallback.psbt,
            test_utxo(
                200_000,
                "2222222222222222222222222222222222222222222222222222222222222222",
                0,
            ),
            ScriptBuf::new(),
            Amount::from_sat(1_000),
        )
        .expect("proposal");

    (fallback.psbt, proposal.psbt)
}

#[test]
fn initiator_validator_accepts_well_formed_privacy_input_proposal() {
    let (original, proposal) = original_and_proposal();

    let validation =
        InitiatorProposalValidator::new(&original, &proposal, 0, Amount::from_sat(1_000))
            .validate()
            .expect("valid proposal");

    assert!(validation.accepted);
    assert_eq!(validation.added_inputs, 1);
    assert_eq!(validation.added_outputs, 1);
    assert_eq!(validation.added_fee, Amount::from_sat(198));
}

#[test]
fn initiator_validator_rejects_modified_original_input() {
    let (original, mut proposal) = original_and_proposal();
    proposal.unsigned_tx.input[0].previous_output.vout = 99;

    let error = InitiatorProposalValidator::new(&original, &proposal, 0, Amount::from_sat(1_000))
        .validate()
        .expect_err("modified input");

    assert!(matches!(error, Error::InvalidProposal(_)));
}

#[test]
fn initiator_validator_rejects_changed_funding_output() {
    let (original, mut proposal) = original_and_proposal();
    proposal.unsigned_tx.output[0].value = Amount::from_sat(999_999);

    let error = InitiatorProposalValidator::new(&original, &proposal, 0, Amount::from_sat(1_000))
        .validate()
        .expect_err("changed funding output");

    assert!(matches!(error, Error::InvalidProposal(_)));
}

#[test]
fn initiator_validator_rejects_fee_above_policy() {
    let (original, proposal) = original_and_proposal();

    let error = InitiatorProposalValidator::new(&original, &proposal, 0, Amount::from_sat(197))
        .validate()
        .expect_err("fee above policy");

    assert!(matches!(error, Error::Policy(_)));
}
