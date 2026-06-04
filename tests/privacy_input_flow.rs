use std::str::FromStr;
use std::time::Duration;

use bitcoin::{Amount, OutPoint, ScriptBuf, Txid};
use lightning_payjoin_kit::directory::MockDirectory;
use lightning_payjoin_kit::error::Error;
use lightning_payjoin_kit::wallet::{MemoryWallet, Utxo};
use lightning_payjoin_kit::{
    FundingCoordinator, FundingMode, FundingPolicy, FundingRequest, FundingState,
};

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

fn request() -> FundingRequest {
    FundingRequest {
        channel_value_sats: 1_000_000,
        funding_script: ScriptBuf::new(),
        mode: FundingMode::PrivacyInput,
        fee_rate_sat_vb: 2.0,
        deadline: Duration::from_secs(30),
    }
}

#[test]
fn coordinator_runs_privacy_input_flow_in_memory() {
    let policy = FundingPolicy::default();
    let mut initiator = FundingCoordinator::new(
        MemoryWallet::new(
            vec![test_utxo(
                1_100_000,
                "1111111111111111111111111111111111111111111111111111111111111111",
                0,
            )],
            vec![ScriptBuf::new()],
        ),
        MockDirectory::default(),
        policy.clone(),
    );
    let mut counterparty = FundingCoordinator::new(
        MemoryWallet::new(
            vec![test_utxo(
                200_000,
                "2222222222222222222222222222222222222222222222222222222222222222",
                0,
            )],
            vec![ScriptBuf::new()],
        ),
        MockDirectory::default(),
        policy,
    );
    let request = request();

    let original = initiator.prepare_original(&request).expect("original");
    assert_eq!(initiator.state(), FundingState::OriginalPrepared);

    let proposal = counterparty
        .propose_privacy_input(&original.psbt, &request)
        .expect("proposal");
    assert_eq!(counterparty.state(), FundingState::ProposalReceived);

    let validation = initiator
        .validate_privacy_input_proposal(&original.psbt, &proposal.psbt)
        .expect("valid proposal");
    assert_eq!(initiator.state(), FundingState::ProposalValidated);
    assert_eq!(validation.added_inputs, 1);
    assert_eq!(validation.added_outputs, 1);
    assert_eq!(validation.added_fee, Amount::from_sat(198));

    assert_eq!(
        proposal.psbt.unsigned_tx.output[original.funding_output_index as usize],
        original.psbt.unsigned_tx.output[original.funding_output_index as usize]
    );
}

#[test]
fn coordinator_finalizes_validated_privacy_input_proposal() {
    let policy = FundingPolicy::default();
    let mut initiator = FundingCoordinator::new(
        MemoryWallet::new(
            vec![test_utxo(
                1_100_000,
                "5555555555555555555555555555555555555555555555555555555555555555",
                0,
            )],
            vec![ScriptBuf::new()],
        ),
        MockDirectory::default(),
        policy.clone(),
    );
    let mut counterparty = FundingCoordinator::new(
        MemoryWallet::new(
            vec![test_utxo(
                200_000,
                "6666666666666666666666666666666666666666666666666666666666666666",
                0,
            )],
            vec![ScriptBuf::new()],
        ),
        MockDirectory::default(),
        policy,
    );
    let request = request();
    let original = initiator.prepare_original(&request).expect("original");
    let proposal = counterparty
        .propose_privacy_input(&original.psbt, &request)
        .expect("proposal");

    let result = initiator
        .finalize_validated_proposal(&original.psbt, proposal.psbt)
        .expect("funding result");

    assert_eq!(initiator.state(), FundingState::BroadcastReady);
    assert!(!result.fallback_used);
    assert_eq!(result.transaction.input.len(), 2);
    assert_eq!(
        result.transaction.output[0].value,
        Amount::from_sat(1_000_000)
    );
    assert_eq!(result.funding_outpoint.vout, 0);
    assert_eq!(
        result.funding_outpoint.txid,
        result.transaction.compute_txid()
    );
}

#[test]
fn coordinator_refuses_to_finalize_when_wallet_signs_no_inputs() {
    let policy = FundingPolicy::default();
    let mut initiator = FundingCoordinator::new(
        MemoryWallet::new(
            vec![test_utxo(
                1_100_000,
                "7777777777777777777777777777777777777777777777777777777777777777",
                0,
            )],
            vec![ScriptBuf::new()],
        ),
        MockDirectory::default(),
        policy.clone(),
    );
    let mut wrong_wallet_initiator = FundingCoordinator::new(
        MemoryWallet::new(
            vec![test_utxo(
                1_100_000,
                "8888888888888888888888888888888888888888888888888888888888888888",
                0,
            )],
            vec![ScriptBuf::new()],
        ),
        MockDirectory::default(),
        policy.clone(),
    );
    let mut counterparty = FundingCoordinator::new(
        MemoryWallet::new(
            vec![test_utxo(
                200_000,
                "9999999999999999999999999999999999999999999999999999999999999999",
                0,
            )],
            vec![ScriptBuf::new()],
        ),
        MockDirectory::default(),
        policy,
    );
    let request = request();
    let original = initiator.prepare_original(&request).expect("original");
    let proposal = counterparty
        .propose_privacy_input(&original.psbt, &request)
        .expect("proposal");

    let error = wrong_wallet_initiator
        .finalize_validated_proposal(&original.psbt, proposal.psbt)
        .expect_err("no owned inputs signed");

    assert!(matches!(error, Error::Signing(_)));
    assert_ne!(wrong_wallet_initiator.state(), FundingState::BroadcastReady);
}

#[test]
fn coordinator_finalizes_with_real_p2wpkh_witnesses_for_both_peers() {
    let policy = FundingPolicy::default();
    let initiator_outpoint = OutPoint {
        txid: Txid::from_str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .expect("txid"),
        vout: 0,
    };
    let counterparty_outpoint = OutPoint {
        txid: Txid::from_str("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
            .expect("txid"),
        vout: 0,
    };
    let mut initiator = FundingCoordinator::new(
        MemoryWallet::deterministic_p2wpkh(
            initiator_outpoint,
            Amount::from_sat(1_100_000),
            1,
            vec![ScriptBuf::new()],
        )
        .expect("initiator wallet"),
        MockDirectory::default(),
        policy.clone(),
    );
    let mut counterparty = FundingCoordinator::new(
        MemoryWallet::deterministic_p2wpkh(
            counterparty_outpoint,
            Amount::from_sat(200_000),
            2,
            vec![ScriptBuf::new()],
        )
        .expect("counterparty wallet"),
        MockDirectory::default(),
        policy,
    );
    let request = request();
    let original = initiator.prepare_original(&request).expect("original");
    let proposal = counterparty
        .propose_privacy_input(&original.psbt, &request)
        .expect("proposal");

    assert_eq!(
        proposal.psbt.inputs[proposal.counterparty_input_index]
            .final_script_witness
            .as_ref()
            .map(|witness| witness.len()),
        Some(2)
    );

    let result = initiator
        .finalize_validated_proposal(&original.psbt, proposal.psbt)
        .expect("finalized signed result");

    assert_eq!(result.transaction.input.len(), 2);
    assert_eq!(result.transaction.input[0].witness.len(), 2);
    assert_eq!(result.transaction.input[1].witness.len(), 2);
    assert_eq!(
        result.funding_outpoint.txid,
        result.transaction.compute_txid()
    );
}

#[test]
fn coordinator_rejects_unconfirmed_counterparty_utxo_when_policy_requires_confirmed() {
    let policy = FundingPolicy::default();
    let original = FundingCoordinator::new(
        MemoryWallet::new(
            vec![test_utxo(
                1_100_000,
                "3333333333333333333333333333333333333333333333333333333333333333",
                0,
            )],
            vec![ScriptBuf::new()],
        ),
        MockDirectory::default(),
        policy.clone(),
    )
    .prepare_original(&request())
    .expect("original");
    let mut unconfirmed = test_utxo(
        200_000,
        "4444444444444444444444444444444444444444444444444444444444444444",
        0,
    );
    unconfirmed.confirmed = false;
    let mut counterparty = FundingCoordinator::new(
        MemoryWallet::new(vec![unconfirmed], vec![ScriptBuf::new()]),
        MockDirectory::default(),
        policy,
    );

    let error = counterparty
        .propose_privacy_input(&original.psbt, &request())
        .expect_err("no eligible confirmed utxo");

    assert!(matches!(
        error,
        lightning_payjoin_kit::Error::InsufficientFunds { .. }
    ));
}
