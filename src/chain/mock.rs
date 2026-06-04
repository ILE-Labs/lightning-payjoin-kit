use bitcoin::{Transaction, Txid};

use crate::chain::Broadcaster;
use crate::error::Result;

#[derive(Debug, Clone, Default)]
pub struct MockBroadcaster {
    transactions: Vec<Transaction>,
}

impl MockBroadcaster {
    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }
}

impl Broadcaster for MockBroadcaster {
    fn broadcast_transaction(&mut self, transaction: &Transaction) -> Result<Txid> {
        let txid = transaction.compute_txid();
        self.transactions.push(transaction.clone());
        Ok(txid)
    }
}
