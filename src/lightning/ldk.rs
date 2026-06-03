use std::ops::Deref;
use std::time::Duration;

use bitcoin::{OutPoint, ScriptBuf, Transaction};
use lightning::chain::chaininterface::{BroadcasterInterface, FeeEstimator};
use lightning::chain::transaction::OutPoint as LdkOutPoint;
use lightning::chain::Watch;
use lightning::events::Event;
use lightning::ln::channelmanager::ChannelManager;
use lightning::ln::types::ChannelId;
use lightning::onion_message::messenger::MessageRouter;
use lightning::routing::router::Router;
use lightning::sign::{EntropySource, NodeSigner, SignerProvider};
use lightning::util::logger::Logger;

use crate::error::{Error, Result};
use crate::funding::{FundingMode, FundingRequest, FundingResult};

use super::{ChannelBalance, ChannelFundingHandoff, CommitmentSafety, PayjoinChannelFunder};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LdkFundingReference {
    pub funding_txo: LdkOutPoint,
    pub funding_transaction: Transaction,
    pub funding_script_pubkey: ScriptBuf,
    pub channel_value_sats: u64,
    pub mode: FundingMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LdkFundingGeneration {
    pub temporary_channel_id: ChannelId,
    pub counterparty_node_id: bitcoin::secp256k1::PublicKey,
    pub user_channel_id: u128,
    pub request: FundingRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LdkFundingSessionState {
    WaitingForFunding,
    ManualFundingReady,
    ManualFundingApplied,
    BroadcastSafe,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LdkFundingSession {
    generation: LdkFundingGeneration,
    reference: Option<LdkFundingReference>,
    manual: Option<LdkManualFunding>,
    broadcast_safe: Option<LdkBroadcastSafe>,
    state: LdkFundingSessionState,
}

impl LdkFundingGeneration {
    pub fn from_event(
        event: &Event,
        mode: FundingMode,
        fee_rate_sat_vb: f32,
        deadline: Duration,
    ) -> Option<Self> {
        match event {
            Event::FundingGenerationReady {
                temporary_channel_id,
                counterparty_node_id,
                channel_value_satoshis,
                output_script,
                user_channel_id,
            } => Some(Self {
                temporary_channel_id: *temporary_channel_id,
                counterparty_node_id: *counterparty_node_id,
                user_channel_id: *user_channel_id,
                request: FundingRequest {
                    channel_value_sats: *channel_value_satoshis,
                    funding_script: output_script.clone(),
                    mode,
                    fee_rate_sat_vb,
                    deadline,
                },
            }),
            _ => None,
        }
    }
}

impl LdkFundingSession {
    pub fn new(generation: LdkFundingGeneration) -> Self {
        Self {
            generation,
            reference: None,
            manual: None,
            broadcast_safe: None,
            state: LdkFundingSessionState::WaitingForFunding,
        }
    }

    pub fn generation(&self) -> &LdkFundingGeneration {
        &self.generation
    }

    pub fn request(&self) -> &FundingRequest {
        &self.generation.request
    }

    pub fn state(&self) -> LdkFundingSessionState {
        self.state
    }

    pub fn reference(&self) -> Option<&LdkFundingReference> {
        self.reference.as_ref()
    }

    pub fn manual(&self) -> Option<&LdkManualFunding> {
        self.manual.as_ref()
    }

    pub fn attach_reference(&mut self, reference: LdkFundingReference) -> Result<&LdkManualFunding> {
        let manual = LdkManualFunding::new(&self.generation, &reference)?;
        self.reference = Some(reference);
        self.manual = Some(manual);
        self.state = LdkFundingSessionState::ManualFundingReady;
        Ok(self
            .manual
            .as_ref()
            .expect("manual funding was just attached"))
    }

    pub fn apply_manual<C: LdkManualFundingCallback>(&mut self, callback: &C) -> Result<()> {
        let manual = self
            .manual
            .as_ref()
            .ok_or_else(|| Error::InvalidProposal("manual funding is not ready".to_owned()))?;
        manual.apply_to(callback)?;
        self.state = LdkFundingSessionState::ManualFundingApplied;
        Ok(())
    }

    pub fn observe_broadcast_safe_event(
        &mut self,
        event: &Event,
        balance: ChannelBalance,
    ) -> Result<Option<ChannelFundingHandoff>> {
        let Some(broadcast_safe) = LdkBroadcastSafe::from_event(event) else {
            return Ok(None);
        };
        self.ensure_broadcast_safe_matches_session(&broadcast_safe)?;

        let reference = self
            .reference
            .clone()
            .ok_or_else(|| Error::InvalidProposal("funding reference is not attached".to_owned()))?;
        let handoff = broadcast_safe.commitment_safe_handoff(reference, balance)?;
        self.broadcast_safe = Some(broadcast_safe);
        self.state = LdkFundingSessionState::BroadcastSafe;
        Ok(Some(handoff))
    }

    fn ensure_broadcast_safe_matches_session(&self, broadcast_safe: &LdkBroadcastSafe) -> Result<()> {
        if broadcast_safe.former_temporary_channel_id != self.generation.temporary_channel_id {
            return Err(Error::InvalidProposal(
                "broadcast-safe event belongs to a different temporary channel".to_owned(),
            ));
        }

        if broadcast_safe.counterparty_node_id != self.generation.counterparty_node_id {
            return Err(Error::InvalidProposal(
                "broadcast-safe event belongs to a different counterparty".to_owned(),
            ));
        }

        if broadcast_safe.user_channel_id != self.generation.user_channel_id {
            return Err(Error::InvalidProposal(
                "broadcast-safe event belongs to a different user channel id".to_owned(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LdkBroadcastSafe {
    pub channel_id: ChannelId,
    pub user_channel_id: u128,
    pub funding_txo: OutPoint,
    pub counterparty_node_id: bitcoin::secp256k1::PublicKey,
    pub former_temporary_channel_id: ChannelId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LdkManualFunding {
    pub temporary_channel_id: ChannelId,
    pub counterparty_node_id: bitcoin::secp256k1::PublicKey,
    pub funding_txo: LdkOutPoint,
    pub user_channel_id: u128,
}

impl LdkManualFunding {
    pub fn new(generation: &LdkFundingGeneration, reference: &LdkFundingReference) -> Result<Self> {
        if reference.channel_value_sats != generation.request.channel_value_sats {
            return Err(Error::InvalidProposal(format!(
                "LDK funding value mismatch: event requested {} sats, reference has {} sats",
                generation.request.channel_value_sats, reference.channel_value_sats
            )));
        }

        if reference.funding_script_pubkey != generation.request.funding_script {
            return Err(Error::InvalidProposal(
                "LDK funding script mismatch between event and reference".to_owned(),
            ));
        }

        if reference.mode != generation.request.mode {
            return Err(Error::InvalidProposal(
                "LDK funding mode mismatch between event and reference".to_owned(),
            ));
        }

        Ok(Self {
            temporary_channel_id: generation.temporary_channel_id,
            counterparty_node_id: generation.counterparty_node_id,
            funding_txo: reference.funding_txo,
            user_channel_id: generation.user_channel_id,
        })
    }

    pub fn apply_to<C: LdkManualFundingCallback>(&self, callback: &C) -> Result<()> {
        callback.unsafe_manual_funding_transaction_generated(
            self.temporary_channel_id,
            self.counterparty_node_id,
            self.funding_txo,
        )
    }
}

pub trait LdkManualFundingCallback {
    fn unsafe_manual_funding_transaction_generated(
        &self,
        temporary_channel_id: ChannelId,
        counterparty_node_id: bitcoin::secp256k1::PublicKey,
        funding_txo: LdkOutPoint,
    ) -> Result<()>;
}

impl<F> LdkManualFundingCallback for F
where
    F: Fn(ChannelId, bitcoin::secp256k1::PublicKey, LdkOutPoint) -> Result<()>,
{
    fn unsafe_manual_funding_transaction_generated(
        &self,
        temporary_channel_id: ChannelId,
        counterparty_node_id: bitcoin::secp256k1::PublicKey,
        funding_txo: LdkOutPoint,
    ) -> Result<()> {
        self(temporary_channel_id, counterparty_node_id, funding_txo)
    }
}

impl<M, T, ES, NS, SP, F, R, MR, L> LdkManualFundingCallback
    for ChannelManager<M, T, ES, NS, SP, F, R, MR, L>
where
    M: Deref,
    T: Deref,
    ES: Deref,
    NS: Deref,
    SP: Deref,
    F: Deref,
    R: Deref,
    MR: Deref,
    L: Deref,
    M::Target: Watch<<SP::Target as SignerProvider>::EcdsaSigner>,
    T::Target: BroadcasterInterface,
    ES::Target: EntropySource,
    NS::Target: NodeSigner,
    SP::Target: SignerProvider,
    F::Target: FeeEstimator,
    R::Target: Router,
    MR::Target: MessageRouter,
    L::Target: Logger,
{
    fn unsafe_manual_funding_transaction_generated(
        &self,
        temporary_channel_id: ChannelId,
        counterparty_node_id: bitcoin::secp256k1::PublicKey,
        funding_txo: LdkOutPoint,
    ) -> Result<()> {
        ChannelManager::unsafe_manual_funding_transaction_generated(
            self,
            temporary_channel_id,
            counterparty_node_id,
            funding_txo,
        )
        .map_err(|err| Error::InvalidProposal(format!("{err:?}")))
    }
}

impl LdkBroadcastSafe {
    pub fn from_event(event: &Event) -> Option<Self> {
        match event {
            Event::FundingTxBroadcastSafe {
                channel_id,
                user_channel_id,
                funding_txo,
                counterparty_node_id,
                former_temporary_channel_id,
            } => Some(Self {
                channel_id: *channel_id,
                user_channel_id: *user_channel_id,
                funding_txo: *funding_txo,
                counterparty_node_id: *counterparty_node_id,
                former_temporary_channel_id: *former_temporary_channel_id,
            }),
            _ => None,
        }
    }

    pub fn commitment_safety(&self) -> CommitmentSafety {
        CommitmentSafety::CommitmentsExchanged
    }

    pub fn matches_reference(&self, reference: &LdkFundingReference) -> bool {
        self.funding_txo == reference.bitcoin_outpoint()
    }

    pub fn commitment_safe_handoff(
        &self,
        reference: LdkFundingReference,
        balance: ChannelBalance,
    ) -> Result<ChannelFundingHandoff> {
        if !self.matches_reference(&reference) {
            return Err(Error::InvalidProposal(
                "LDK broadcast-safe outpoint does not match funding reference".to_owned(),
            ));
        }

        Ok(ChannelFundingHandoff::new(
            FundingResult {
                transaction: reference.funding_transaction,
                funding_outpoint: self.funding_txo,
                fallback_used: false,
            },
            reference.funding_script_pubkey,
            balance,
            reference.mode,
            self.commitment_safety(),
        ))
    }
}

impl LdkFundingReference {
    pub fn from_handoff(handoff: ChannelFundingHandoff) -> Result<Self> {
        handoff.ensure_commitment_safe()?;

        if handoff.mode == FundingMode::PrivacyInput && handoff.balance.counterparty_sats != 0 {
            return Err(Error::Policy(
                "privacy-input mode must not assign channel balance to the counterparty".to_owned(),
            ));
        }

        let funding_output = handoff
            .result
            .transaction
            .output
            .get(handoff.result.funding_outpoint.vout as usize)
            .ok_or_else(|| Error::InvalidProposal("funding output index out of bounds".to_owned()))?;
        let channel_value_sats = handoff.balance.initiator_sats + handoff.balance.counterparty_sats;

        if funding_output.value.to_sat() != channel_value_sats {
            return Err(Error::InvalidProposal(format!(
                "funding output value does not match channel balance: output {} sats, balance {channel_value_sats} sats",
                funding_output.value.to_sat()
            )));
        }

        if funding_output.script_pubkey != handoff.funding_script_pubkey {
            return Err(Error::InvalidProposal(
                "funding output script does not match handoff script".to_owned(),
            ));
        }

        if handoff.result.transaction.compute_txid() != handoff.result.funding_outpoint.txid {
            return Err(Error::InvalidProposal(
                "funding outpoint txid does not match funding transaction".to_owned(),
            ));
        }

        Ok(Self {
            funding_txo: ldk_outpoint(handoff.result.funding_outpoint)?,
            funding_transaction: handoff.result.transaction,
            funding_script_pubkey: handoff.funding_script_pubkey,
            channel_value_sats,
            mode: handoff.mode,
        })
    }

    pub fn bitcoin_outpoint(&self) -> OutPoint {
        self.funding_txo.into_bitcoin_outpoint()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LdkFundingAdapter {
    accepted: Vec<LdkFundingReference>,
}

impl LdkFundingAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accepted(&self) -> &[LdkFundingReference] {
        &self.accepted
    }
}

impl PayjoinChannelFunder for LdkFundingAdapter {
    type Channel = LdkFundingReference;

    fn accept_funding(&mut self, handoff: ChannelFundingHandoff) -> Result<Self::Channel> {
        let reference = LdkFundingReference::from_handoff(handoff)?;
        self.accepted.push(reference.clone());
        Ok(reference)
    }
}

pub fn ldk_outpoint(outpoint: OutPoint) -> Result<LdkOutPoint> {
    let index = u16::try_from(outpoint.vout).map_err(|_| {
        Error::InvalidProposal(format!(
            "funding output index {} exceeds LDK u16 outpoint limit",
            outpoint.vout
        ))
    })?;

    Ok(LdkOutPoint {
        txid: outpoint.txid,
        index,
    })
}

pub fn commitment_safe_handoff(
    result: FundingResult,
    funding_script_pubkey: ScriptBuf,
    balance: ChannelBalance,
    mode: FundingMode,
) -> ChannelFundingHandoff {
    ChannelFundingHandoff::new(
        result,
        funding_script_pubkey,
        balance,
        mode,
        CommitmentSafety::CommitmentsExchanged,
    )
}
