use bitcoin::{OutPoint, Psbt, Transaction};

use crate::error::{Error, Result};
use crate::wallet::SigningSummary;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalizedFunding {
    pub transaction: Transaction,
    pub funding_outpoint: OutPoint,
    pub signing_summary: SigningSummary,
}

impl FinalizedFunding {
    pub fn extract(
        psbt: Psbt,
        funding_output_index: u32,
        signing_summary: SigningSummary,
    ) -> Result<Self> {
        let transaction = psbt
            .extract_tx()
            .map_err(|err| Error::InvalidPsbt(err.to_string()))?;
        let funding_outpoint = OutPoint {
            txid: transaction.compute_txid(),
            vout: funding_output_index,
        };

        Ok(Self {
            transaction,
            funding_outpoint,
            signing_summary,
        })
    }
}
