#![cfg(feature = "ldk")]

use std::str::FromStr;
use std::time::Duration;

use bitcoin::{key::PrivateKey, secp256k1, Amount, NetworkKind, OutPoint, ScriptBuf, Txid};
use lightning::events::Event;
use lightning::ln::types::ChannelId;
use lightning_payjoin_kit::directory::MockDirectory;
use lightning_payjoin_kit::error::Error;
use lightning_payjoin_kit::lightning::{
    commitment_safe_handoff, ldk_outpoint, ChannelBalance, CommitmentSafety, FundingScript,
    LdkBroadcastSafe, LdkFundingAdapter, LdkFundingGeneration, LdkFundingReference,
    LdkManualFunding, PayjoinChannelFunder,
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

#[test]
fn ldk_funding_generation_event_maps_to_funding_request() {
    let output_script = ScriptBuf::new();
    let event = Event::FundingGenerationReady {
        temporary_channel_id: ChannelId([3; 32]),
        counterparty_node_id: public_key(44),
        channel_value_satoshis: 1_250_000,
        output_script: output_script.clone(),
        user_channel_id: 99,
    };

    let generation = LdkFundingGeneration::from_event(
        &event,
        FundingMode::PrivacyInput,
        2.5,
        Duration::from_secs(90),
    )
    .expect("funding generation event");

    assert_eq!(generation.temporary_channel_id, ChannelId([3; 32]));
    assert_eq!(generation.counterparty_node_id, public_key(44));
    assert_eq!(generation.user_channel_id, 99);
    assert_eq!(generation.request.channel_value_sats, 1_250_000);
    assert_eq!(generation.request.funding_script, output_script);
    assert_eq!(generation.request.mode, FundingMode::PrivacyInput);
    assert_eq!(generation.request.fee_rate_sat_vb, 2.5);
    assert_eq!(generation.request.deadline, Duration::from_secs(90));
}

#[test]
fn ldk_broadcast_safe_event_marks_commitment_safe() {
    let (result, funding_script) = finalized_privacy_input_funding();
    let handoff = commitment_safe_handoff(
        result,
        funding_script.script_pubkey,
        ChannelBalance {
            initiator_sats: 1_000_000,
            counterparty_sats: 0,
        },
        FundingMode::PrivacyInput,
    );
    let reference = LdkFundingReference::from_handoff(handoff).expect("reference");
    let event = Event::FundingTxBroadcastSafe {
        channel_id: ChannelId([9; 32]),
        user_channel_id: 77,
        funding_txo: reference.bitcoin_outpoint(),
        counterparty_node_id: public_key(55),
        former_temporary_channel_id: ChannelId([8; 32]),
    };

    let broadcast_safe = LdkBroadcastSafe::from_event(&event).expect("broadcast safe event");

    assert_eq!(
        broadcast_safe.commitment_safety(),
        CommitmentSafety::CommitmentsExchanged
    );
    assert!(broadcast_safe.matches_reference(&reference));
    assert_eq!(broadcast_safe.channel_id, ChannelId([9; 32]));
    assert_eq!(broadcast_safe.user_channel_id, 77);
    assert_eq!(broadcast_safe.counterparty_node_id, public_key(55));
}

#[test]
fn ldk_manual_funding_payload_matches_channel_manager_callback_shape() {
    let (result, funding_script) = finalized_privacy_input_funding();
    let reference = LdkFundingReference::from_handoff(commitment_safe_handoff(
        result,
        funding_script.script_pubkey.clone(),
        ChannelBalance {
            initiator_sats: 1_000_000,
            counterparty_sats: 0,
        },
        FundingMode::PrivacyInput,
    ))
    .expect("reference");
    let generation = LdkFundingGeneration {
        temporary_channel_id: ChannelId([7; 32]),
        counterparty_node_id: public_key(77),
        user_channel_id: 7_777,
        request: FundingRequest {
            channel_value_sats: reference.channel_value_sats,
            funding_script: funding_script.script_pubkey,
            mode: reference.mode,
            fee_rate_sat_vb: 2.0,
            deadline: Duration::from_secs(30),
        },
    };

    let manual = LdkManualFunding::new(&generation, &reference).expect("manual funding");

    assert_eq!(manual.temporary_channel_id, generation.temporary_channel_id);
    assert_eq!(manual.counterparty_node_id, generation.counterparty_node_id);
    assert_eq!(manual.funding_txo, reference.funding_txo);
    assert_eq!(manual.user_channel_id, generation.user_channel_id);
}

#[test]
fn ldk_manual_funding_rejects_event_reference_mismatch() {
    let (result, funding_script) = finalized_privacy_input_funding();
    let reference = LdkFundingReference::from_handoff(commitment_safe_handoff(
        result,
        funding_script.script_pubkey,
        ChannelBalance {
            initiator_sats: 1_000_000,
            counterparty_sats: 0,
        },
        FundingMode::PrivacyInput,
    ))
    .expect("reference");
    let generation = LdkFundingGeneration {
        temporary_channel_id: ChannelId([7; 32]),
        counterparty_node_id: public_key(77),
        user_channel_id: 7_777,
        request: FundingRequest {
            channel_value_sats: reference.channel_value_sats,
            funding_script: ScriptBuf::new(),
            mode: reference.mode,
            fee_rate_sat_vb: 2.0,
            deadline: Duration::from_secs(30),
        },
    };

    let error = LdkManualFunding::new(&generation, &reference).expect_err("script mismatch");

    assert!(matches!(error, Error::InvalidProposal(_)));
}

#[test]
fn ldk_broadcast_safe_event_builds_commitment_safe_handoff() {
    let (result, funding_script) = finalized_privacy_input_funding();
    let reference = LdkFundingReference::from_handoff(commitment_safe_handoff(
        result,
        funding_script.script_pubkey,
        ChannelBalance {
            initiator_sats: 1_000_000,
            counterparty_sats: 0,
        },
        FundingMode::PrivacyInput,
    ))
    .expect("reference");
    let event = Event::FundingTxBroadcastSafe {
        channel_id: ChannelId([9; 32]),
        user_channel_id: 77,
        funding_txo: reference.bitcoin_outpoint(),
        counterparty_node_id: public_key(55),
        former_temporary_channel_id: ChannelId([8; 32]),
    };
    let broadcast_safe = LdkBroadcastSafe::from_event(&event).expect("broadcast safe event");

    let handoff = broadcast_safe
        .commitment_safe_handoff(
            reference,
            ChannelBalance {
                initiator_sats: 1_000_000,
                counterparty_sats: 0,
            },
        )
        .expect("commitment-safe handoff");

    assert_eq!(handoff.commitment_safety, CommitmentSafety::CommitmentsExchanged);
    assert_eq!(handoff.mode, FundingMode::PrivacyInput);
    assert_eq!(handoff.balance.counterparty_sats, 0);
}

#[test]
fn ldk_event_helpers_ignore_unrelated_events() {
    let event = Event::ChannelPending {
        channel_id: ChannelId([1; 32]),
        user_channel_id: 11,
        former_temporary_channel_id: None,
        counterparty_node_id: public_key(66),
        funding_txo: OutPoint {
            txid: Txid::from_str("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd")
                .expect("txid"),
            vout: 0,
        },
        channel_type: None,
        funding_redeem_script: None,
    };

    assert!(LdkFundingGeneration::from_event(
        &event,
        FundingMode::PrivacyInput,
        1.0,
        Duration::from_secs(30)
    )
    .is_none());
    assert!(LdkBroadcastSafe::from_event(&event).is_none());
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

fn public_key(secret_byte: u8) -> secp256k1::PublicKey {
    let secp = secp256k1::Secp256k1::new();
    secp256k1::PublicKey::from_secret_key(&secp, &private_key(secret_byte).inner)
}

fn private_key(secret_byte: u8) -> PrivateKey {
    PrivateKey::new(
        secp256k1::SecretKey::from_slice(&[secret_byte; 32]).expect("secret key"),
        NetworkKind::Test,
    )
}
