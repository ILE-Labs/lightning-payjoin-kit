use super::{FundingPolicy, FundingRequest, FundingResult, FundingState};
use crate::directory::DirectoryClient;
use crate::error::Result;
use crate::wallet::Wallet;

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

    pub fn prepare_funding(&mut self, _request: FundingRequest) -> Result<Option<FundingResult>> {
        self.state = FundingState::OriginalPrepared;
        Ok(None)
    }
}
