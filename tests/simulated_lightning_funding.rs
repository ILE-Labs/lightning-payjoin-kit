use std::str::FromStr;
use std::time::Duration;

use bitcoin::{key::PrivateKey, secp256k1, Amount, NetworkKind, OutPoint, ScriptBuf, Txid};
use lightning_payjoin_kit::directory::MockDirectory;
use lightning_payjoin_kit::lightning::{ChannelBalance, FundingScript, SimulatedChannel};
use lightning_payjoin_kit::wallet::MemoryWallet;
use lightning_payjoin_kit::{
    FundingCoordinator, FundingMode, FundingPolicy, FundingRequest, FundingState,
};

fn private_key(secret_byte: u8) -> PrivateKey {
    PrivateKey::new(
        secp256k1::SecretKey::from_slice(&[secret_byte; 32]).expect("secret key"),
        NetworkKind::Test,
    )
}

#[test]
fn privacy_input_flow_funds_simulated_2of2_lightning_channel() {
    let secp = secp256k1::Secp256k1::new();
    let initiator_funding_key = private_key(11).public_key(&secp);
    let counterparty_funding_key = private_key(12).public_key(&secp);
    let funding_script = FundingScript::new_2of2(initiator_funding_key, counterparty_funding_key);
    assert!(funding_script.script_pubkey.is_p2wsh());

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
    let request = FundingRequest {
        channel_value_sats: 1_000_000,
        funding_script: funding_script.script_pubkey.clone(),
        mode: FundingMode::PrivacyInput,
        fee_rate_sat_vb: 2.0,
        deadline: Duration::from_secs(30),
    };

    let original = initiator.prepare_original(&request).expect("original");
    let proposal = counterparty
        .propose_privacy_input(&original.psbt, &request)
        .expect("proposal");
    let result = initiator
        .finalize_validated_proposal(&original.psbt, proposal.psbt)
        .expect("finalized result");
    let channel = SimulatedChannel::from_funding_result(
        &result,
        ChannelBalance {
            initiator_sats: 1_000_000,
            counterparty_sats: 0,
        },
        funding_script.script_pubkey.clone(),
    )
    .expect("simulated channel");

    assert_eq!(initiator.state(), FundingState::BroadcastReady);
    assert_eq!(channel.balance.initiator_sats, 1_000_000);
    assert_eq!(channel.balance.counterparty_sats, 0);
    assert_eq!(
        result.transaction.output[result.funding_outpoint.vout as usize].script_pubkey,
        funding_script.script_pubkey
    );
}
