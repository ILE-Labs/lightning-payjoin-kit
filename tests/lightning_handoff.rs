use std::str::FromStr;
use std::time::Duration;

use bitcoin::{key::PrivateKey, secp256k1, Amount, NetworkKind, OutPoint, ScriptBuf, Txid};
use lightning_payjoin_kit::directory::MockDirectory;
use lightning_payjoin_kit::error::Error;
use lightning_payjoin_kit::lightning::{
    ChannelBalance, ChannelFundingHandoff, CommitmentSafety, FundingScript, PayjoinChannelFunder,
    SimulatedChannelFunder,
};
use lightning_payjoin_kit::wallet::MemoryWallet;
use lightning_payjoin_kit::{FundingCoordinator, FundingMode, FundingPolicy, FundingRequest};

#[test]
fn simulated_funder_accepts_commitment_safe_privacy_input_handoff() {
    let (result, funding_script) = finalized_privacy_input_funding();
    let handoff = ChannelFundingHandoff::new(
        result.clone(),
        funding_script.script_pubkey.clone(),
        ChannelBalance {
            initiator_sats: 1_000_000,
            counterparty_sats: 0,
        },
        FundingMode::PrivacyInput,
        CommitmentSafety::CommitmentsExchanged,
    );

    let mut funder = SimulatedChannelFunder;
    let channel = funder.accept_funding(handoff).expect("channel handoff");

    assert_eq!(channel.funding_outpoint, result.funding_outpoint);
    assert_eq!(channel.balance.counterparty_sats, 0);
    assert_eq!(channel.funding_script_pubkey, funding_script.script_pubkey);
}

#[test]
fn simulated_funder_rejects_unsafe_handoff() {
    let (result, funding_script) = finalized_privacy_input_funding();
    let handoff = ChannelFundingHandoff::new(
        result,
        funding_script.script_pubkey,
        ChannelBalance {
            initiator_sats: 1_000_000,
            counterparty_sats: 0,
        },
        FundingMode::PrivacyInput,
        CommitmentSafety::Unsafe,
    );

    let mut funder = SimulatedChannelFunder;
    let error = funder
        .accept_funding(handoff)
        .expect_err("unsafe handoff must be rejected");

    assert!(matches!(error, Error::Policy(_)));
}

#[test]
fn privacy_input_handoff_does_not_allow_counterparty_channel_balance() {
    let (result, funding_script) = finalized_privacy_input_funding();
    let handoff = ChannelFundingHandoff::new(
        result,
        funding_script.script_pubkey,
        ChannelBalance {
            initiator_sats: 900_000,
            counterparty_sats: 100_000,
        },
        FundingMode::PrivacyInput,
        CommitmentSafety::CommitmentsExchanged,
    );

    let mut funder = SimulatedChannelFunder;
    let error = funder
        .accept_funding(handoff)
        .expect_err("privacy input balance must be rejected");

    assert!(matches!(error, Error::Policy(_)));
}

fn finalized_privacy_input_funding() -> (lightning_payjoin_kit::FundingResult, FundingScript) {
    let secp = secp256k1::Secp256k1::new();
    let funding_script = FundingScript::new_2of2(
        private_key(11).public_key(&secp),
        private_key(12).public_key(&secp),
    );
    let request = FundingRequest {
        channel_value_sats: 1_000_000,
        funding_script: funding_script.script_pubkey.clone(),
        mode: FundingMode::PrivacyInput,
        fee_rate_sat_vb: 2.0,
        deadline: Duration::from_secs(30),
    };
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

    let original = initiator.prepare_original(&request).expect("original");
    let proposal = counterparty
        .propose_privacy_input(&original.psbt, &request)
        .expect("proposal");
    let result = initiator
        .finalize_validated_proposal(&original.psbt, proposal.psbt)
        .expect("funding result");

    (result, funding_script)
}

fn private_key(secret_byte: u8) -> PrivateKey {
    PrivateKey::new(
        secp256k1::SecretKey::from_slice(&[secret_byte; 32]).expect("secret key"),
        NetworkKind::Test,
    )
}
