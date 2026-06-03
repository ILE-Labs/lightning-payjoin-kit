use std::str::FromStr;
use std::time::Duration;

use bitcoin::{Amount, OutPoint, ScriptBuf, Txid};
use lightning_payjoin_kit::directory::MockDirectory;
use lightning_payjoin_kit::payjoin::{PayjoinPayload, PayjoinPayloadKind};
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
fn payjoin_payload_roundtrips_psbt() {
    let mut coordinator = FundingCoordinator::new(
        MemoryWallet::new(
            vec![test_utxo(
                1_100_000,
                "1111111111111111111111111111111111111111111111111111111111111111",
                0,
            )],
            vec![ScriptBuf::new()],
        ),
        MockDirectory::default(),
        FundingPolicy::default(),
    );
    let original = coordinator.prepare_original(&request()).expect("original");

    let payload =
        PayjoinPayload::from_psbt(PayjoinPayloadKind::Original, &original.psbt).expect("payload");
    let decoded = payload
        .into_psbt(PayjoinPayloadKind::Original)
        .expect("decoded psbt");

    assert_eq!(decoded.unsigned_tx, original.psbt.unsigned_tx);
}

#[test]
fn coordinators_exchange_original_and_proposal_through_mock_directory() {
    let directory = MockDirectory::default();
    let policy = FundingPolicy::default();
    let mut initiator = FundingCoordinator::new(
        MemoryWallet::new(
            vec![test_utxo(
                1_100_000,
                "2222222222222222222222222222222222222222222222222222222222222222",
                0,
            )],
            vec![ScriptBuf::new()],
        ),
        directory.clone(),
        policy.clone(),
    );
    let mut counterparty = FundingCoordinator::new(
        MemoryWallet::new(
            vec![test_utxo(
                200_000,
                "3333333333333333333333333333333333333333333333333333333333333333",
                0,
            )],
            vec![ScriptBuf::new()],
        ),
        directory,
        policy,
    );
    let request = request();

    let (session, original) = initiator
        .post_original_to_directory(&request)
        .expect("posted original");
    assert_eq!(initiator.state(), FundingState::ProposalRequested);

    counterparty
        .propose_from_directory(&session.id, &request)
        .expect("posted proposal");
    assert_eq!(counterparty.state(), FundingState::ProposalReceived);

    let validation = initiator
        .validate_proposal_from_directory(&session.id, &original.psbt)
        .expect("validated proposal");

    assert_eq!(initiator.state(), FundingState::ProposalValidated);
    assert_eq!(validation.added_inputs, 1);
    assert_eq!(validation.added_outputs, 1);
    assert_eq!(validation.added_fee, Amount::from_sat(198));
}
