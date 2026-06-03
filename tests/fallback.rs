use std::time::Duration;

use bitcoin::ScriptBuf;
use lightning_payjoin_kit::directory::MockDirectory;
use lightning_payjoin_kit::wallet::MemoryWallet;
use lightning_payjoin_kit::{
    FundingCoordinator, FundingMode, FundingPolicy, FundingRequest, FundingState,
};

#[test]
fn coordinator_moves_to_original_prepared() {
    let wallet = MemoryWallet::default();
    let directory = MockDirectory::default();
    let mut coordinator = FundingCoordinator::new(wallet, directory, FundingPolicy::default());

    let result = coordinator
        .prepare_funding(FundingRequest {
            channel_value_sats: 1_000_000,
            funding_script: ScriptBuf::new(),
            mode: FundingMode::PrivacyInput,
            fee_rate_sat_vb: 1.0,
            deadline: Duration::from_secs(30),
        })
        .expect("prepare funding");

    assert!(result.is_none());
    assert_eq!(coordinator.state(), FundingState::OriginalPrepared);
}
