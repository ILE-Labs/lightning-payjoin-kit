use std::time::Duration;

use bitcoin::{Amount, OutPoint, ScriptBuf, Txid};
use lightning_payjoin_kit::directory::MockDirectory;
use lightning_payjoin_kit::wallet::{MemoryWallet, Utxo};
use lightning_payjoin_kit::{
    FundingCoordinator, FundingMode, FundingPolicy, FundingRequest, FundingState,
};
use std::str::FromStr;

#[test]
fn coordinator_moves_to_original_prepared() {
    let wallet = MemoryWallet::new(
        vec![Utxo {
            outpoint: OutPoint {
                txid: Txid::from_str(
                    "2222222222222222222222222222222222222222222222222222222222222222",
                )
                .expect("txid"),
                vout: 0,
            },
            value: Amount::from_sat(1_100_000),
            script_pubkey: ScriptBuf::new(),
            confirmed: true,
        }],
        vec![ScriptBuf::new()],
    );
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

    let result = result.expect("funding result");
    assert!(result.fallback_used);
    assert_eq!(
        result.transaction.output[0].value,
        Amount::from_sat(1_000_000)
    );
    assert_eq!(coordinator.state(), FundingState::OriginalPrepared);
}
