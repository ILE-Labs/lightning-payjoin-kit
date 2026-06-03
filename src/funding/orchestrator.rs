use super::{FundingPolicy, FundingRequest, FundingResult, FundingState};
use crate::directory::DirectoryClient;
use crate::error::{Error, Result};
use crate::payjoin::{
    CounterpartyOriginalValidator, InitiatorProposalValidator, ProposalValidation,
    ProposalValidator,
};
use crate::psbt::{FallbackFunding, FundingPsbtBuilder, PrivacyInputProposal};
use crate::wallet::{Utxo, Wallet};
use bitcoin::{Amount, Psbt};

#[derive(Debug)]
pub struct FundingCoordinator<W, D> {
    wallet: W,
    directory: D,
    policy: FundingPolicy,
    state: FundingState,
}

impl<W, D> FundingCoordinator<W, D>
where
    W: Wallet,
    D: DirectoryClient,
{
    pub fn new(wallet: W, directory: D, policy: FundingPolicy) -> Self {
        Self {
            wallet,
            directory,
            policy,
            state: FundingState::Idle,
        }
    }

    pub fn state(&self) -> FundingState {
        self.state
    }

    pub fn wallet(&self) -> &W {
        &self.wallet
    }

    pub fn directory(&self) -> &D {
        &self.directory
    }

    pub fn policy(&self) -> &FundingPolicy {
        &self.policy
    }

    pub fn prepare_original(&mut self, request: &FundingRequest) -> Result<FallbackFunding> {
        let utxos = self.eligible_utxos()?;
        let change_script = self.wallet.next_change_script()?;
        let builder = self.builder_for(request);
        let fallback = builder.build_fallback(&utxos, change_script)?;
        self.state = FundingState::OriginalPrepared;
        Ok(fallback)
    }

    pub fn prepare_funding(&mut self, request: FundingRequest) -> Result<Option<FundingResult>> {
        let fallback = self.prepare_original(&request)?;
        let transaction = fallback.psbt.unsigned_tx;
        let funding_outpoint = bitcoin::OutPoint {
            txid: transaction.compute_txid(),
            vout: fallback.funding_output_index,
        };

        Ok(Some(FundingResult {
            transaction,
            funding_outpoint,
            fallback_used: true,
        }))
    }

    pub fn propose_privacy_input(
        &mut self,
        original: &Psbt,
        request: &FundingRequest,
    ) -> Result<PrivacyInputProposal> {
        let builder = self.builder_for(request);
        let counterparty_utxo =
            self.eligible_utxos()?
                .into_iter()
                .next()
                .ok_or(Error::InsufficientFunds {
                    needed_sats: builder.privacy_input_fee_contribution().to_sat(),
                    available_sats: 0,
                })?;
        let change_script = self.wallet.next_change_script()?;
        let required_fee = builder.privacy_input_fee_contribution();

        CounterpartyOriginalValidator::new(
            original,
            &counterparty_utxo,
            Amount::from_sat(request.channel_value_sats),
            Amount::from_sat(self.policy.max_counterparty_fee_contribution_sats),
            required_fee,
        )
        .validate()?;

        let proposal = builder.build_privacy_input_proposal(
            original,
            counterparty_utxo,
            change_script,
            Amount::from_sat(self.policy.max_counterparty_fee_contribution_sats),
        )?;
        self.state = FundingState::ProposalReceived;
        Ok(proposal)
    }

    pub fn validate_privacy_input_proposal(
        &mut self,
        original: &Psbt,
        proposal: &Psbt,
    ) -> Result<ProposalValidation> {
        let validation = InitiatorProposalValidator::new(
            original,
            proposal,
            0,
            Amount::from_sat(self.policy.max_counterparty_fee_contribution_sats),
        )
        .validate()?;
        self.state = FundingState::ProposalValidated;
        Ok(validation)
    }

    fn builder_for(&self, request: &FundingRequest) -> FundingPsbtBuilder {
        let fee_rate_sat_vb = request
            .fee_rate_sat_vb
            .max(self.policy.min_fee_rate_sat_vb)
            .ceil()
            .max(1.0) as u64;
        FundingPsbtBuilder::new(
            Amount::from_sat(request.channel_value_sats),
            request.funding_script.clone(),
        )
        .with_fee_rate_sat_vb(fee_rate_sat_vb)
    }

    fn eligible_utxos(&self) -> Result<Vec<Utxo>> {
        let utxos = self.wallet.list_spendable_utxos()?;
        if self.policy.require_confirmed_inputs {
            Ok(utxos.into_iter().filter(|utxo| utxo.confirmed).collect())
        } else {
            Ok(utxos)
        }
    }
}
