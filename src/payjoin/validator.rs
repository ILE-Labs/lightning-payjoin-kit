use bitcoin::{Amount, Psbt};

use crate::error::{Error, Result};
use crate::wallet::Utxo;

const DUST_CHANGE_SATS: u64 = 546;

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

#[derive(Debug)]
pub struct CounterpartyOriginalValidator<'a> {
    original: &'a Psbt,
    counterparty_utxo: &'a Utxo,
    expected_channel_value: Amount,
    max_fee_contribution: Amount,
    required_fee_contribution: Amount,
}

impl<'a> CounterpartyOriginalValidator<'a> {
    pub fn new(
        original: &'a Psbt,
        counterparty_utxo: &'a Utxo,
        expected_channel_value: Amount,
        max_fee_contribution: Amount,
        required_fee_contribution: Amount,
    ) -> Self {
        Self {
            original,
            counterparty_utxo,
            expected_channel_value,
            max_fee_contribution,
            required_fee_contribution,
        }
    }

    fn validate_counterparty_input_is_fresh(&self) -> Result<()> {
        let already_present = self
            .original
            .unsigned_tx
            .input
            .iter()
            .any(|input| input.previous_output == self.counterparty_utxo.outpoint);

        if already_present {
            return Err(Error::InvalidProposal(
                "counterparty input is already present in original PSBT".to_owned(),
            ));
        }

        Ok(())
    }

    fn validate_counterparty_change_policy(&self) -> Result<()> {
        if self.required_fee_contribution > self.max_fee_contribution {
            return Err(Error::Policy(format!(
                "required fee contribution exceeds policy: {} sats > {} sats",
                self.required_fee_contribution.to_sat(),
                self.max_fee_contribution.to_sat()
            )));
        }

        let change_value = self
            .counterparty_utxo
            .value
            .to_sat()
            .checked_sub(self.required_fee_contribution.to_sat())
            .ok_or_else(|| Error::Policy("counterparty input cannot pay fee".to_owned()))?;

        if change_value < DUST_CHANGE_SATS {
            return Err(Error::Policy(format!(
                "counterparty change would be dust: {change_value} sats"
            )));
        }

        Ok(())
    }
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

impl ProposalValidator for CounterpartyOriginalValidator<'_> {
    fn validate(&self) -> Result<ProposalValidation> {
        let funding_output = self.original.unsigned_tx.output.first().ok_or_else(|| {
            Error::InvalidProposal("original PSBT missing funding output".to_owned())
        })?;

        if funding_output.value != self.expected_channel_value {
            return Err(Error::InvalidProposal(format!(
                "unexpected channel value: expected {} sats, got {} sats",
                self.expected_channel_value.to_sat(),
                funding_output.value.to_sat()
            )));
        }

        let input_value = sum_input_values(self.original)?;
        let output_value = sum_output_values(self.original);
        let fee = input_value.checked_sub(output_value).ok_or_else(|| {
            Error::InvalidProposal("original PSBT outputs exceed inputs".to_owned())
        })?;

        self.validate_counterparty_input_is_fresh()?;
        self.validate_counterparty_change_policy()?;

        Ok(ProposalValidation {
            accepted: true,
            added_inputs: 0,
            added_outputs: 0,
            added_fee: Amount::from_sat(fee),
        })
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
