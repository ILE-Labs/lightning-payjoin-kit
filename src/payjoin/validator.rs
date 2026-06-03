use bitcoin::{Amount, Psbt};

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalValidation {
    pub accepted: bool,
    pub added_inputs: usize,
    pub added_outputs: usize,
    pub added_fee: Amount,
}

pub trait ProposalValidator {
    fn validate(&self) -> Result<ProposalValidation>;
}

#[derive(Debug)]
pub struct InitiatorProposalValidator<'a> {
    original: &'a Psbt,
    proposal: &'a Psbt,
    funding_output_index: usize,
    max_added_fee: Amount,
}

impl<'a> InitiatorProposalValidator<'a> {
    pub fn new(
        original: &'a Psbt,
        proposal: &'a Psbt,
        funding_output_index: usize,
        max_added_fee: Amount,
    ) -> Self {
        Self {
            original,
            proposal,
            funding_output_index,
            max_added_fee,
        }
    }

    fn original_input_value(&self) -> Result<u64> {
        sum_input_values(self.original)
    }

    fn proposal_input_value(&self) -> Result<u64> {
        sum_input_values(self.proposal)
    }

    fn original_output_value(&self) -> u64 {
        sum_output_values(self.original)
    }

    fn proposal_output_value(&self) -> u64 {
        sum_output_values(self.proposal)
    }
}

impl ProposalValidator for InitiatorProposalValidator<'_> {
    fn validate(&self) -> Result<ProposalValidation> {
        if self.original.unsigned_tx.version != self.proposal.unsigned_tx.version {
            return Err(Error::InvalidProposal(
                "transaction version changed".to_owned(),
            ));
        }

        if self.original.unsigned_tx.lock_time != self.proposal.unsigned_tx.lock_time {
            return Err(Error::InvalidProposal("locktime changed".to_owned()));
        }

        let original_inputs = &self.original.unsigned_tx.input;
        let proposal_inputs = &self.proposal.unsigned_tx.input;
        if proposal_inputs.len() <= original_inputs.len() {
            return Err(Error::InvalidProposal(
                "proposal did not add a counterparty input".to_owned(),
            ));
        }

        if proposal_inputs[..original_inputs.len()] != *original_inputs {
            return Err(Error::InvalidProposal(
                "original inputs were modified".to_owned(),
            ));
        }

        let original_outputs = &self.original.unsigned_tx.output;
        let proposal_outputs = &self.proposal.unsigned_tx.output;
        let original_funding_output = original_outputs
            .get(self.funding_output_index)
            .ok_or_else(|| Error::InvalidProposal("missing original funding output".to_owned()))?;
        let proposal_funding_output = proposal_outputs
            .get(self.funding_output_index)
            .ok_or_else(|| Error::InvalidProposal("missing proposal funding output".to_owned()))?;

        if proposal_funding_output != original_funding_output {
            return Err(Error::InvalidProposal(
                "channel funding output changed".to_owned(),
            ));
        }

        if proposal_outputs.len() <= original_outputs.len() {
            return Err(Error::InvalidProposal(
                "proposal did not add a counterparty change output".to_owned(),
            ));
        }

        let original_input_value = self.original_input_value()?;
        let proposal_input_value = self.proposal_input_value()?;
        let original_output_value = self.original_output_value();
        let proposal_output_value = self.proposal_output_value();
        let original_fee = original_input_value
            .checked_sub(original_output_value)
            .ok_or_else(|| Error::InvalidProposal("original outputs exceed inputs".to_owned()))?;
        let proposal_fee = proposal_input_value
            .checked_sub(proposal_output_value)
            .ok_or_else(|| Error::InvalidProposal("proposal outputs exceed inputs".to_owned()))?;
        let added_fee = proposal_fee.checked_sub(original_fee).ok_or_else(|| {
            Error::InvalidProposal("proposal reduced absolute transaction fee".to_owned())
        })?;

        if added_fee > self.max_added_fee.to_sat() {
            return Err(Error::Policy(format!(
                "proposal added fee above policy: {added_fee} sats > {} sats",
                self.max_added_fee.to_sat()
            )));
        }

        Ok(ProposalValidation {
            accepted: true,
            added_inputs: proposal_inputs.len() - original_inputs.len(),
            added_outputs: proposal_outputs.len() - original_outputs.len(),
            added_fee: Amount::from_sat(added_fee),
        })
    }
}

fn sum_input_values(psbt: &Psbt) -> Result<u64> {
    psbt.inputs
        .iter()
        .map(|input| {
            input
                .witness_utxo
                .as_ref()
                .map(|utxo| utxo.value.to_sat())
                .ok_or_else(|| Error::InvalidPsbt("input missing witness_utxo".to_owned()))
        })
        .sum()
}

fn sum_output_values(psbt: &Psbt) -> u64 {
    psbt.unsigned_tx
        .output
        .iter()
        .map(|output| output.value.to_sat())
        .sum()
}
