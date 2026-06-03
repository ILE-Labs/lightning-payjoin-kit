use std::str::FromStr;
use std::time::Duration;

use bitcoin::{Amount, OutPoint, ScriptBuf, Txid};
use lightning_payjoin_kit::directory::MockDirectory;
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
