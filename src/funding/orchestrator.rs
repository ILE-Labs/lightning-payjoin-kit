use super::{FundingPolicy, FundingRequest, FundingResult, FundingState};
use crate::directory::DirectoryClient;
use crate::error::Result;
use crate::psbt::FundingPsbtBuilder;
use crate::wallet::Wallet;
use bitcoin::Amount;

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

    pub fn prepare_funding(&mut self, request: FundingRequest) -> Result<Option<FundingResult>> {
        let utxos = self.wallet.list_spendable_utxos()?;
        let change_script = self.wallet.next_change_script()?;
        let fee_rate_sat_vb = request.fee_rate_sat_vb.ceil().max(1.0) as u64;
        let builder = FundingPsbtBuilder::new(
            Amount::from_sat(request.channel_value_sats),
            request.funding_script,
        )
        .with_fee_rate_sat_vb(fee_rate_sat_vb);
        let fallback = builder.build_fallback(&utxos, change_script)?;
        let transaction = fallback.psbt.unsigned_tx;
        let funding_outpoint = bitcoin::OutPoint {
            txid: transaction.compute_txid(),
            vout: fallback.funding_output_index,
        };

        self.state = FundingState::OriginalPrepared;
        Ok(Some(FundingResult {
            transaction,
            funding_outpoint,
            fallback_used: true,
        }))
    }
}
