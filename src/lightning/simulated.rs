use bitcoin::{OutPoint, ScriptBuf, Transaction};

use crate::error::{Error, Result};
use crate::funding::FundingResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelBalance {
    pub initiator_sats: u64,
    pub counterparty_sats: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedChannel {
    pub funding_outpoint: OutPoint,
    pub balance: ChannelBalance,
    pub funding_script_pubkey: ScriptBuf,
}

impl SimulatedChannel {
    pub fn new(
        funding_outpoint: OutPoint,
        balance: ChannelBalance,
        funding_script_pubkey: ScriptBuf,
    ) -> Self {
        Self {
            funding_outpoint,
            balance,
            funding_script_pubkey,
        }
    }

    pub fn from_funding_result(
        result: &FundingResult,
        balance: ChannelBalance,
        funding_script_pubkey: ScriptBuf,
    ) -> Result<Self> {
        validate_funding_transaction(
            &result.transaction,
            result.funding_outpoint,
            balance,
            &funding_script_pubkey,
        )?;

        Ok(Self::new(
            result.funding_outpoint,
            balance,
            funding_script_pubkey,
        ))
    }
}

pub fn validate_funding_transaction(
    transaction: &Transaction,
    funding_outpoint: OutPoint,
    balance: ChannelBalance,
    funding_script_pubkey: &ScriptBuf,
) -> Result<()> {
    let funding_output = transaction
        .output
        .get(funding_outpoint.vout as usize)
        .ok_or_else(|| Error::InvalidProposal("funding output index out of bounds".to_owned()))?;
    let expected_value = balance.initiator_sats + balance.counterparty_sats;

    if funding_output.value.to_sat() != expected_value {
        return Err(Error::InvalidProposal(format!(
            "funding output value does not match channel balance: output {} sats, balance {expected_value} sats",
            funding_output.value.to_sat()
        )));
    }

    if funding_output.script_pubkey != *funding_script_pubkey {
        return Err(Error::InvalidProposal(
            "funding output script does not match simulated channel script".to_owned(),
        ));
    }

    if transaction.compute_txid() != funding_outpoint.txid {
        return Err(Error::InvalidProposal(
            "funding outpoint txid does not match funding transaction".to_owned(),
        ));
    }

    Ok(())
}
