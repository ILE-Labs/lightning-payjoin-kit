#![cfg(feature = "ldk")]

use std::str::FromStr;
use std::time::Duration;

use bitcoin::{key::PrivateKey, secp256k1, Amount, NetworkKind, OutPoint, ScriptBuf, Txid};
use lightning_payjoin_kit::directory::MockDirectory;
use lightning_payjoin_kit::error::Error;
use lightning_payjoin_kit::lightning::{
    commitment_safe_handoff, ldk_outpoint, ChannelBalance, FundingScript, LdkFundingAdapter,
    LdkFundingReference, PayjoinChannelFunder,
};
use lightning_payjoin_kit::wallet::MemoryWallet;
use lightning_payjoin_kit::{FundingCoordinator, FundingMode, FundingPolicy, FundingRequest};

#[test]
fn ldk_adapter_accepts_commitment_safe_funding_reference() {
    let (result, funding_script) = finalized_privacy_input_funding();
    let expected_outpoint = result.funding_outpoint;
    let handoff = commitment_safe_handoff(
        result,
        funding_script.script_pubkey,
        ChannelBalance {
            initiator_sats: 1_000_000,
            counterparty_sats: 0,
        },
        FundingMode::PrivacyInput,
    );
    let mut adapter = LdkFundingAdapter::new();

    let reference = adapter.accept_funding(handoff).expect("ldk reference");

    assert_eq!(reference.funding_txo.txid, expected_outpoint.txid);
    assert_eq!(reference.funding_txo.index, expected_outpoint.vout as u16);
    assert_eq!(reference.bitcoin_outpoint(), expected_outpoint);
    assert_eq!(reference.channel_value_sats, 1_000_000);
    assert_eq!(reference.mode, FundingMode::PrivacyInput);
    assert_eq!(adapter.accepted(), &[reference]);
}

#[test]
fn ldk_outpoint_rejects_output_index_above_lightning_limit() {
    let outpoint = OutPoint {
        txid: Txid::from_str("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")
            .expect("txid"),
        vout: u16::MAX as u32 + 1,
    };

    let error = ldk_outpoint(outpoint).expect_err("vout above u16 must fail");

    assert!(matches!(error, Error::InvalidProposal(_)));
}

#[test]
fn ldk_reference_rejects_mismatched_funding_script() {
    let (result, _funding_script) = finalized_privacy_input_funding();
    let handoff = commitment_safe_handoff(
        result,
        ScriptBuf::new(),
        ChannelBalance {
            initiator_sats: 1_000_000,
            counterparty_sats: 0,
        },
        FundingMode::PrivacyInput,
    );

    let error = LdkFundingReference::from_handoff(handoff).expect_err("script mismatch");

    assert!(matches!(error, Error::InvalidProposal(_)));
}

fn finalized_privacy_input_funding() -> (lightning_payjoin_kit::FundingResult, FundingScript) {
    let secp = secp256k1::Secp256k1::new();
    let funding_script =
        FundingScript::new_2of2(private_key(11).public_key(&secp), private_key(12).public_key(&secp));
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
