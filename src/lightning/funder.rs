use bitcoin::ScriptBuf;

use crate::error::{Error, Result};
use crate::funding::{FundingMode, FundingResult};

use super::{ChannelBalance, SimulatedChannel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitmentSafety {
    Unsafe,
    CommitmentsExchanged,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelFundingHandoff {
    pub result: FundingResult,
    pub funding_script_pubkey: ScriptBuf,
    pub balance: ChannelBalance,
    pub mode: FundingMode,
    pub commitment_safety: CommitmentSafety,
}

impl ChannelFundingHandoff {
    pub fn new(
        result: FundingResult,
        funding_script_pubkey: ScriptBuf,
        balance: ChannelBalance,
        mode: FundingMode,
        commitment_safety: CommitmentSafety,
    ) -> Self {
        Self {
            result,
            funding_script_pubkey,
            balance,
            mode,
            commitment_safety,
        }
    }

    pub fn ensure_commitment_safe(&self) -> Result<()> {
        if self.commitment_safety != CommitmentSafety::CommitmentsExchanged {
            return Err(Error::Policy(
                "funding transaction cannot be handed to broadcast before commitment safety"
                    .to_owned(),
            ));
        }

        Ok(())
    }
}

pub trait PayjoinChannelFunder {
    type Channel;

    fn accept_funding(&mut self, handoff: ChannelFundingHandoff) -> Result<Self::Channel>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SimulatedChannelFunder;

impl PayjoinChannelFunder for SimulatedChannelFunder {
    type Channel = SimulatedChannel;

    fn accept_funding(&mut self, handoff: ChannelFundingHandoff) -> Result<Self::Channel> {
        handoff.ensure_commitment_safe()?;

        if handoff.mode == FundingMode::PrivacyInput && handoff.balance.counterparty_sats != 0 {
            return Err(Error::Policy(
                "privacy-input mode must not assign channel balance to the counterparty".to_owned(),
            ));
        }

        SimulatedChannel::from_funding_result(
            &handoff.result,
            handoff.balance,
            handoff.funding_script_pubkey,
        )
    }
}
