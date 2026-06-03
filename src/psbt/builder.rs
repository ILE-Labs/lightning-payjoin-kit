use bitcoin::{Amount, OutPoint, Psbt, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};

use crate::error::{Error, Result};
use crate::wallet::Utxo;

const P2WPKH_INPUT_VBYTES: u64 = 68;
const P2WPKH_OUTPUT_VBYTES: u64 = 31;
const TX_OVERHEAD_VBYTES: u64 = 10;
const DUST_CHANGE_SATS: u64 = 546;

#[derive(Debug, Clone)]
pub struct FundingPsbtBuilder {
    channel_value: Amount,
    funding_script: ScriptBuf,
    fee_rate_sat_vb: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallbackFunding {
    pub psbt: Psbt,
    pub selected_utxos: Vec<Utxo>,
    pub funding_output_index: u32,
    pub change_output_index: Option<u32>,
    pub fee: Amount,
}

impl FundingPsbtBuilder {
    pub fn new(channel_value: Amount, funding_script: ScriptBuf) -> Self {
        Self {
            channel_value,
            funding_script,
            fee_rate_sat_vb: 1,
        }
    }

    pub fn with_fee_rate_sat_vb(mut self, fee_rate_sat_vb: u64) -> Self {
        self.fee_rate_sat_vb = fee_rate_sat_vb.max(1);
        self
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

    pub fn build_fallback(
        &self,
        available_utxos: &[Utxo],
        change_script: ScriptBuf,
    ) -> Result<FallbackFunding> {
        let selected_utxos = self.select_utxos(available_utxos)?;
        let selected_value = selected_utxos
            .iter()
            .map(|utxo| utxo.value.to_sat())
            .sum::<u64>();
        let channel_value = self.channel_value.to_sat();

        let fee_without_change = self.estimated_fee(selected_utxos.len(), 1);
        let mut outputs = vec![TxOut {
            value: self.channel_value,
            script_pubkey: self.funding_script.clone(),
        }];

        let remaining_without_change = selected_value
            .checked_sub(channel_value + fee_without_change)
            .ok_or(Error::InsufficientFunds {
                needed_sats: channel_value + fee_without_change,
                available_sats: selected_value,
            })?;

        let fee = if remaining_without_change >= DUST_CHANGE_SATS {
            let fee_with_change = self.estimated_fee(selected_utxos.len(), 2);
            let change_value = selected_value
                .checked_sub(channel_value + fee_with_change)
                .ok_or(Error::InsufficientFunds {
                    needed_sats: channel_value + fee_with_change,
                    available_sats: selected_value,
                })?;

            if change_value >= DUST_CHANGE_SATS {
                outputs.push(TxOut {
                    value: Amount::from_sat(change_value),
                    script_pubkey: change_script,
                });
                fee_with_change
            } else {
                fee_without_change + remaining_without_change
            }
        } else {
            fee_without_change + remaining_without_change
        };

        let inputs = selected_utxos
            .iter()
            .map(|utxo| TxIn {
                previous_output: utxo.outpoint,
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::default(),
            })
            .collect();

        let unsigned_tx = Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: inputs,
            output: outputs,
        };

        let mut psbt = Psbt::from_unsigned_tx(unsigned_tx)
            .map_err(|err| Error::InvalidPsbt(err.to_string()))?;

        for (input, utxo) in psbt.inputs.iter_mut().zip(&selected_utxos) {
            input.witness_utxo = Some(TxOut {
                value: utxo.value,
                script_pubkey: utxo.script_pubkey.clone(),
            });
        }

        let change_output_index = if psbt.unsigned_tx.output.len() > 1 {
            Some(1)
        } else {
            None
        };

        Ok(FallbackFunding {
            psbt,
            selected_utxos,
            funding_output_index: 0,
            change_output_index,
            fee: Amount::from_sat(fee),
        })
    }

    pub fn funding_outpoint(txid: bitcoin::Txid, vout: u32) -> OutPoint {
        OutPoint { txid, vout }
    }

    fn select_utxos(&self, available_utxos: &[Utxo]) -> Result<Vec<Utxo>> {
        let mut selected = Vec::new();
        let mut selected_value = 0_u64;
        let target = self.channel_value.to_sat();

        for utxo in available_utxos {
            selected.push(utxo.clone());
            selected_value += utxo.value.to_sat();

            let fee_without_change = self.estimated_fee(selected.len(), 1);
            let needed_without_change = target + fee_without_change;

            if selected_value >= needed_without_change {
                return Ok(selected);
            }
        }

        let available_sats = available_utxos.iter().map(|utxo| utxo.value.to_sat()).sum();
        Err(Error::InsufficientFunds {
            needed_sats: target + self.estimated_fee(available_utxos.len().max(1), 1),
            available_sats,
        })
    }

    fn estimated_fee(&self, input_count: usize, output_count: usize) -> u64 {
        let vbytes = TX_OVERHEAD_VBYTES
            + P2WPKH_INPUT_VBYTES * input_count as u64
            + P2WPKH_OUTPUT_VBYTES * output_count as u64;
        vbytes * self.fee_rate_sat_vb
    }
}
