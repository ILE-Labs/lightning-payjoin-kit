use bitcoin::Psbt;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PayjoinPayloadKind {
    Original,
    Proposal,
    Final,
    Abort,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayjoinPayload {
    pub kind: PayjoinPayloadKind,
    pub bytes: Vec<u8>,
}

impl PayjoinPayload {
    pub fn new(kind: PayjoinPayloadKind, bytes: Vec<u8>) -> Self {
        Self { kind, bytes }
    }

    pub fn from_psbt(kind: PayjoinPayloadKind, psbt: &Psbt) -> Result<Self> {
        Ok(Self {
            kind,
            bytes: psbt.serialize(),
        })
    }

    pub fn into_psbt(self, expected_kind: PayjoinPayloadKind) -> Result<Psbt> {
        if self.kind != expected_kind {
            return Err(Error::InvalidProposal(format!(
                "unexpected payload kind: expected {expected_kind:?}, got {:?}",
                self.kind
            )));
        }

        Psbt::deserialize(&self.bytes).map_err(|err| Error::InvalidPsbt(err.to_string()))
    }
}
