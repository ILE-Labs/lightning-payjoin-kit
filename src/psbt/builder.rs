use bitcoin::{Amount, OutPoint, Psbt, ScriptBuf, Transaction, TxOut};

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct FundingPsbtBuilder {
    channel_value: Amount,
    funding_script: ScriptBuf,
}

impl FundingPsbtBuilder {
    pub fn new(channel_value: Amount, funding_script: ScriptBuf) -> Self {
        Self {
            channel_value,
            funding_script,
        }
    }

    pub fn build_empty_fallback(&self) -> Result<Psbt> {
        let unsigned_tx = Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: Vec::new(),
            output: vec![TxOut {
                value: self.channel_value,
                script_pubkey: self.funding_script.clone(),
            }],
        };

        Psbt::from_unsigned_tx(unsigned_tx)
            .map_err(|err| crate::error::Error::InvalidPsbt(err.to_string()))
    }

    pub fn funding_outpoint(txid: bitcoin::Txid, vout: u32) -> OutPoint {
        OutPoint { txid, vout }
    }
}
