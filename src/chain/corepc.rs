use std::str::FromStr;

use bitcoin::{Address, Amount, BlockHash, Denomination, Transaction, Txid};
use corepc_client::client_sync::{self, v29::Client};
use serde_json::{json, Value};

use crate::chain::Broadcaster;
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorepcAuth {
    None,
    UserPass { user: String, password: String },
    CookieFile(String),
}

#[derive(Debug)]
pub struct CorepcRegtestClient {
    client: Client,
}

impl CorepcRegtestClient {
    pub fn new(url: &str, auth: CorepcAuth) -> Result<Self> {
        let client = match auth {
            CorepcAuth::None => Client::new(url),
            CorepcAuth::UserPass { user, password } => {
                Client::new_with_auth(url, client_sync::Auth::UserPass(user, password))
                    .map_err(map_corepc_error)?
            }
            CorepcAuth::CookieFile(path) => {
                Client::new_with_auth(url, client_sync::Auth::CookieFile(path.into()))
                    .map_err(map_corepc_error)?
            }
        };

        Ok(Self { client })
    }

    pub fn inner(&self) -> &Client {
        &self.client
    }

    pub fn get_block_count(&self) -> Result<u64> {
        self.client
            .get_block_count()
            .map(|count| count.0)
            .map_err(map_corepc_error)
    }

    pub fn new_address(&self) -> Result<Address> {
        self.client.new_address().map_err(map_corepc_error)
    }

    pub fn generate_to_address(&self, blocks: usize, address: &Address) -> Result<Vec<BlockHash>> {
        self.client
            .generate_to_address(blocks, address)
            .and_then(|blocks| blocks.into_model().map(|model| model.0).map_err(Into::into))
            .map_err(map_corepc_error)
    }

    pub fn create_wallet(&self, wallet_name: &str) -> Result<Value> {
        self.client
            .call("createwallet", &[wallet_name.into()])
            .map_err(map_corepc_error)
    }

    pub fn load_wallet(&self, wallet_name: &str) -> Result<Value> {
        self.client
            .call("loadwallet", &[wallet_name.into()])
            .map_err(map_corepc_error)
    }

    pub fn list_unspent(&self) -> Result<corepc_client::types::model::ListUnspent> {
        let unspent = self.client.list_unspent().map_err(map_corepc_error)?;
        unspent
            .into_model()
            .map_err(|err| Error::CoreRpc(err.to_string()))
    }

    pub fn send_to_address(&self, address: &Address, amount: Amount) -> Result<Txid> {
        let value = self
            .client
            .call(
                "sendtoaddress",
                &[
                    json!(address.to_string()),
                    json!(amount.to_string_in(Denomination::Bitcoin)),
                ],
            )
            .map_err(map_corepc_error)?;
        parse_txid_value(value)
    }

    pub fn get_wallet_transaction(&self, txid: Txid) -> Result<Value> {
        self.client
            .call("gettransaction", &[json!(txid.to_string()), json!(false), json!(true)])
            .map_err(map_corepc_error)
    }

    pub fn broadcast(&self, transaction: &Transaction) -> Result<Txid> {
        self.client
            .send_raw_transaction(transaction)
            .and_then(|txid| txid.into_model().map(|model| model.0).map_err(Into::into))
            .map_err(map_corepc_error)
    }
}

impl Broadcaster for CorepcRegtestClient {
    fn broadcast_transaction(&mut self, transaction: &Transaction) -> Result<Txid> {
        self.broadcast(transaction)
    }
}

fn map_corepc_error(error: corepc_client::client_sync::Error) -> Error {
    Error::CoreRpc(error.to_string())
}

fn parse_txid_value(value: Value) -> Result<Txid> {
    let txid = value
        .as_str()
        .ok_or_else(|| Error::CoreRpc("RPC response did not contain a txid string".to_owned()))?;
    Txid::from_str(txid).map_err(|err| Error::CoreRpc(err.to_string()))
}
