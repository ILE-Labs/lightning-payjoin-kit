#![cfg(feature = "ldk-test-utils")]

use std::time::Duration;

use bitcoin::{Amount, OutPoint, ScriptBuf, Txid};
use lightning::events::Event;
use lightning::ln::functional_test_utils::{
    check_added_monitors, create_chan_between_nodes_with_value_b,
    create_chan_between_nodes_with_value_confirm, create_chanmon_cfgs, create_network,
    create_node_cfgs, create_node_chanmgrs,
};
use lightning::ln::msgs::{BaseMessageHandler, ChannelMessageHandler, MessageSendEvent};
use lightning_payjoin_kit::directory::MockDirectory;
use lightning_payjoin_kit::lightning::{
    commitment_safe_handoff, ChannelBalance, LdkFundingAdapter, LdkFundingGeneration,
    LdkFundingSession, PayjoinChannelFunder,
};
use lightning_payjoin_kit::wallet::MemoryWallet;
use lightning_payjoin_kit::{FundingCoordinator, FundingMode, FundingPolicy};

#[test]
#[ignore = "prototype harness for real LDK ChannelManager manual-funding flow"]
fn two_ldk_nodes_accept_collaborative_manual_funding_outpoint() {
    let chanmon_cfgs = create_chanmon_cfgs(2);
    let node_cfgs = create_node_cfgs(2, &chanmon_cfgs);
    let node_chanmgrs = create_node_chanmgrs(2, &node_cfgs, &[None, None]);
    let nodes = create_network(2, &node_cfgs, &node_chanmgrs);
    let initiator = &nodes[0];
    let counterparty = &nodes[1];
    let channel_value_sats = 1_000_000;
    let user_channel_id = 42;

    initiator
        .node
        .create_channel(
            counterparty.node.get_our_node_id(),
            channel_value_sats,
            0,
            user_channel_id,
            None,
            None,
        )
        .expect("create channel");
    let open_channel = expect_open_channel(initiator, counterparty.node.get_our_node_id());
    counterparty
        .node
        .handle_open_channel(initiator.node.get_our_node_id(), &open_channel);
    let accept_channel = expect_accept_channel(counterparty, initiator.node.get_our_node_id());
    initiator
        .node
        .handle_accept_channel(counterparty.node.get_our_node_id(), &accept_channel);

    let event = initiator
        .node
        .get_and_clear_pending_events()
        .into_iter()
        .next()
        .expect("funding generation event");
    let generation = LdkFundingGeneration::from_event(
        &event,
        FundingMode::PrivacyInput,
        2.0,
        Duration::from_secs(30),
    )
    .expect("LDK funding generation");
    let mut session = LdkFundingSession::new(generation);

    let funding_result = collaborative_funding_for_ldk_request(session.request());
    let mut adapter = LdkFundingAdapter::new();
    let reference = adapter
        .accept_funding(commitment_safe_handoff(
            funding_result,
            session.request().funding_script.clone(),
            ChannelBalance {
                initiator_sats: channel_value_sats,
                counterparty_sats: 0,
            },
            FundingMode::PrivacyInput,
        ))
        .expect("LDK funding reference");

    let manual = session
        .attach_reference(reference)
        .expect("manual funding payload");
    assert_eq!(manual.funding_txo.index, 0);
    session
        .apply_manual(initiator.node)
        .expect("apply manual funding to ChannelManager");

    let funding_created = expect_funding_created(initiator, counterparty.node.get_our_node_id());
    counterparty
        .node
        .handle_funding_created(initiator.node.get_our_node_id(), &funding_created);
    check_added_monitors(counterparty, 1);
    let _counterparty_pending = counterparty.node.get_and_clear_pending_events();

    let funding_signed = expect_funding_signed(counterparty, initiator.node.get_our_node_id());
    initiator
        .node
        .handle_funding_signed(counterparty.node.get_our_node_id(), &funding_signed);
    check_added_monitors(initiator, 1);
    let initiator_events = initiator.node.get_and_clear_pending_events();
    let broadcast_safe_event = initiator_events
        .iter()
        .find(|event| matches!(event, Event::FundingTxBroadcastSafe { .. }))
        .expect("funding broadcast safe event");
    let handoff = session
        .observe_broadcast_safe_event(
            broadcast_safe_event,
            ChannelBalance {
                initiator_sats: channel_value_sats,
                counterparty_sats: 0,
            },
        )
        .expect("broadcast safe observation")
        .expect("commitment-safe handoff");

    assert_eq!(
        handoff.result.funding_outpoint,
        funding_created.funding_txo()
    );

    let (funding_messages, _channel_id) = create_chan_between_nodes_with_value_confirm(
        initiator,
        counterparty,
        &handoff.result.transaction,
    );
    create_chan_between_nodes_with_value_b(initiator, counterparty, &funding_messages);
    assert_eq!(initiator.node.list_usable_channels().len(), 1);
    assert_eq!(counterparty.node.list_usable_channels().len(), 1);
}

fn collaborative_funding_for_ldk_request(
    request: &lightning_payjoin_kit::FundingRequest,
) -> lightning_payjoin_kit::FundingResult {
    let initiator_outpoint = OutPoint {
        txid: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .parse::<Txid>()
            .expect("txid"),
        vout: 0,
    };
    let counterparty_outpoint = OutPoint {
        txid: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            .parse::<Txid>()
            .expect("txid"),
        vout: 0,
    };
    let policy = FundingPolicy::default();
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

    let original = initiator.prepare_original(request).expect("original");
    let proposal = counterparty
        .propose_privacy_input(&original.psbt, request)
        .expect("proposal");
    initiator
        .finalize_validated_proposal(&original.psbt, proposal.psbt)
        .expect("finalized funding")
}

fn expect_open_channel<'a, 'b, 'c>(
    node: &lightning::ln::functional_test_utils::Node<'a, 'b, 'c>,
    counterparty_node_id: bitcoin::secp256k1::PublicKey,
) -> lightning::ln::msgs::OpenChannel {
    node.node
        .get_and_clear_pending_msg_events()
        .into_iter()
        .find_map(|event| match event {
            MessageSendEvent::SendOpenChannel { node_id, msg } => {
                assert_eq!(node_id, counterparty_node_id);
                Some(msg)
            }
            _ => None,
        })
        .expect("open channel message")
}

fn expect_accept_channel<'a, 'b, 'c>(
    node: &lightning::ln::functional_test_utils::Node<'a, 'b, 'c>,
    counterparty_node_id: bitcoin::secp256k1::PublicKey,
) -> lightning::ln::msgs::AcceptChannel {
    node.node
        .get_and_clear_pending_msg_events()
        .into_iter()
        .find_map(|event| match event {
            MessageSendEvent::SendAcceptChannel { node_id, msg } => {
                assert_eq!(node_id, counterparty_node_id);
                Some(msg)
            }
            _ => None,
        })
        .expect("accept channel message")
}

fn expect_funding_created<'a, 'b, 'c>(
    node: &lightning::ln::functional_test_utils::Node<'a, 'b, 'c>,
    counterparty_node_id: bitcoin::secp256k1::PublicKey,
) -> lightning::ln::msgs::FundingCreated {
    node.node
        .get_and_clear_pending_msg_events()
        .into_iter()
        .find_map(|event| match event {
            MessageSendEvent::SendFundingCreated { node_id, msg } => {
                assert_eq!(node_id, counterparty_node_id);
                Some(msg)
            }
            _ => None,
        })
        .expect("funding created message")
}

fn expect_funding_signed<'a, 'b, 'c>(
    node: &lightning::ln::functional_test_utils::Node<'a, 'b, 'c>,
    counterparty_node_id: bitcoin::secp256k1::PublicKey,
) -> lightning::ln::msgs::FundingSigned {
    node.node
        .get_and_clear_pending_msg_events()
        .into_iter()
        .find_map(|event| match event {
            MessageSendEvent::SendFundingSigned { node_id, msg } => {
                assert_eq!(node_id, counterparty_node_id);
                Some(msg)
            }
            _ => None,
        })
        .expect("funding signed message")
}

trait FundingCreatedOutpoint {
    fn funding_txo(&self) -> bitcoin::OutPoint;
}

impl FundingCreatedOutpoint for lightning::ln::msgs::FundingCreated {
    fn funding_txo(&self) -> bitcoin::OutPoint {
        bitcoin::OutPoint {
            txid: self.funding_txid,
            vout: self.funding_output_index as u32,
        }
    }
}
