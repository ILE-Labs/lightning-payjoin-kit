use std::path::PathBuf;
use std::process::Command;

use bitcoin::{consensus, Transaction, Txid};

use crate::error::{Error, Result};

pub trait Broadcaster {
    fn broadcast_transaction(&mut self, transaction: &Transaction) -> Result<Txid>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitcoinCliBroadcaster {
    cli_path: PathBuf,
    network_arg: String,
}

impl BitcoinCliBroadcaster {
    pub fn regtest() -> Self {
        Self {
            cli_path: PathBuf::from("bitcoin-cli"),
            network_arg: "-regtest".to_owned(),
        }
    }

    pub fn new(cli_path: impl Into<PathBuf>, network_arg: impl Into<String>) -> Self {
        Self {
            cli_path: cli_path.into(),
            network_arg: network_arg.into(),
        }
    }
}

impl Broadcaster for BitcoinCliBroadcaster {
    fn broadcast_transaction(&mut self, transaction: &Transaction) -> Result<Txid> {
        let raw_tx = consensus::encode::serialize_hex(transaction);
        let output = Command::new(&self.cli_path)
            .arg(&self.network_arg)
            .arg("sendrawtransaction")
            .arg(raw_tx)
            .output()
            .map_err(|err| Error::Broadcast(err.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Broadcast(stderr.trim().to_owned()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .trim()
            .parse()
            .map_err(|err| Error::Broadcast(format!("invalid txid from bitcoin-cli: {err}")))
    }
}
