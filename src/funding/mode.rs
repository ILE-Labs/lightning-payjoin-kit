use bitcoin::{OutPoint, ScriptBuf, Transaction};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FundingMode {
    TrueDualFunding,
    PrivacyInput,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FundingPolicy {
    pub max_counterparty_fee_contribution_sats: u64,
    pub min_fee_rate_sat_vb: f32,
    pub require_confirmed_inputs: bool,
}

impl Default for FundingPolicy {
    fn default() -> Self {
        Self {
            max_counterparty_fee_contribution_sats: 1_000,
            min_fee_rate_sat_vb: 1.0,
            require_confirmed_inputs: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FundingRequest {
    pub channel_value_sats: u64,
    pub funding_script: ScriptBuf,
    pub mode: FundingMode,
    pub fee_rate_sat_vb: f32,
    pub deadline: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FundingResult {
    pub transaction: Transaction,
    pub funding_outpoint: OutPoint,
    pub fallback_used: bool,
}
