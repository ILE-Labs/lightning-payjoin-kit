use lightning_payjoin_kit::{FundingMode, FundingPolicy};

#[test]
fn privacy_input_mode_is_distinct_from_true_dual_funding() {
    assert_ne!(FundingMode::PrivacyInput, FundingMode::TrueDualFunding);
    assert!(FundingPolicy::default().max_counterparty_fee_contribution_sats > 0);
}
