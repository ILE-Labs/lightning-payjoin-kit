use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("wallet error: {0}")]
    Wallet(String),
    #[error("directory error: {0}")]
    Directory(String),
    #[error("invalid PSBT: {0}")]
    InvalidPsbt(String),
    #[error("invalid funding proposal: {0}")]
    InvalidProposal(String),
    #[error("policy violation: {0}")]
    Policy(String),
    #[error("coordination timed out")]
    Timeout,
    #[error("collaborative funding is unsupported by the counterparty")]
    Unsupported,
}
