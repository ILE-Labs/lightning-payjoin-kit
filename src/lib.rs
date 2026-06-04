//! Research-oriented primitives for collaborative Lightning channel funding.
//!
//! The crate is intentionally scaffolded around protocol boundaries first:
//! funding orchestration, Payjoin-style session exchange, PSBT construction,
//! wallet access, directory transport, and Lightning integration hooks.

pub mod chain;
pub mod directory;
pub mod error;
pub mod funding;
pub mod lightning;
pub mod payjoin;
pub mod psbt;
pub mod wallet;

pub use crate::error::{Error, Result};
pub use crate::funding::{
    FundingCoordinator, FundingMode, FundingPolicy, FundingRequest, FundingResult, FundingState,
};
