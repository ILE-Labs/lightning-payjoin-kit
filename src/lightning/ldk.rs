use bitcoin::{OutPoint, ScriptBuf, Transaction};
use lightning::chain::transaction::OutPoint as LdkOutPoint;

use crate::error::{Error, Result};
use crate::funding::{FundingMode, FundingResult};

use super::{ChannelBalance, ChannelFundingHandoff, CommitmentSafety, PayjoinChannelFunder};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LdkFundingReference {
    pub funding_txo: LdkOutPoint,
    pub funding_transaction: Transaction,
    pub funding_script_pubkey: ScriptBuf,
    pub channel_value_sats: u64,
    pub mode: FundingMode,
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
