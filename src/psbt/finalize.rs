use bitcoin::{OutPoint, Transaction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalizedFunding {
    pub transaction: Transaction,
    pub funding_outpoint: OutPoint,
}
