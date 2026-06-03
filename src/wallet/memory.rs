use bitcoin::{ScriptBuf, Transaction};

use crate::error::Result;
use crate::wallet::{Utxo, Wallet};

#[derive(Debug, Clone, Default)]
pub struct MemoryWallet {
    utxos: Vec<Utxo>,
    change_scripts: Vec<ScriptBuf>,
}

impl MemoryWallet {
    pub fn new(utxos: Vec<Utxo>, change_scripts: Vec<ScriptBuf>) -> Self {
        Self {
            utxos,
            change_scripts,
        }
    }
}

impl Wallet for MemoryWallet {
    fn list_spendable_utxos(&self) -> Result<Vec<Utxo>> {
        Ok(self.utxos.clone())
    }

    fn next_change_script(&mut self) -> Result<ScriptBuf> {
        Ok(self.change_scripts.pop().unwrap_or_default())
    }

    fn sign_owned_inputs(&self, _transaction: &mut Transaction) -> Result<()> {
        Ok(())
    }
}
