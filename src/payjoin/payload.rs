use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
}
