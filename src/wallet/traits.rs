use bitcoin::{Amount, OutPoint, ScriptBuf, Transaction};

use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utxo {
    pub outpoint: OutPoint,
    pub value: Amount,
    pub script_pubkey: ScriptBuf,
    pub confirmed: bool,
}

pub trait Wallet {
    fn list_spendable_utxos(&self) -> Result<Vec<Utxo>>;
    fn next_change_script(&mut self) -> Result<ScriptBuf>;
    fn sign_owned_inputs(&self, transaction: &mut Transaction) -> Result<()>;
}
