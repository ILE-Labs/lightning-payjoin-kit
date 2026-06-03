use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("wallet error: {0}")]
    Wallet(String),
    #[error("directory error: {0}")]
    Directory(String),
    #[error("broadcast error: {0}")]
    Broadcast(String),
    #[error("core rpc error: {0}")]
    CoreRpc(String),
    #[error("invalid PSBT: {0}")]
    InvalidPsbt(String),
    #[error("signing error: {0}")]
    Signing(String),
    #[error("invalid funding proposal: {0}")]
    InvalidProposal(String),
    #[error("policy violation: {0}")]
    Policy(String),
    #[error("insufficient funds: need {needed_sats} sats, available {available_sats} sats")]
    InsufficientFunds {
        needed_sats: u64,
        available_sats: u64,
    },
    #[error("coordination timed out")]
    Timeout,
    #[error("collaborative funding is unsupported by the counterparty")]
    Unsupported,
}
