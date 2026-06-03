use std::str::FromStr;
use std::time::Duration;

use bitcoin::{Amount, OutPoint, ScriptBuf, Txid};
use lightning_payjoin_kit::chain::MockBroadcaster;
use lightning_payjoin_kit::directory::MockDirectory;
use lightning_payjoin_kit::wallet::MemoryWallet;
use lightning_payjoin_kit::{
    FundingCoordinator, FundingMode, FundingPolicy, FundingRequest, FundingState,
};

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
fn coordinator_broadcasts_finalized_funding_result_with_mock_broadcaster() {
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
    let result = initiator
        .finalize_validated_proposal(&original.psbt, proposal.psbt)
        .expect("finalized result");
    let mut broadcaster = MockBroadcaster::default();

    let txid = initiator
        .broadcast_funding(&result, &mut broadcaster)
        .expect("broadcast");

    assert_eq!(txid, result.transaction.compute_txid());
    assert_eq!(initiator.state(), FundingState::Broadcasted);
    assert_eq!(broadcaster.transactions().len(), 1);
    assert_eq!(broadcaster.transactions()[0], result.transaction);
}
