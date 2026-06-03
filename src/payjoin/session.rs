use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mailbox {
    pub id: String,
}

impl Mailbox {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayjoinSession {
    pub id: SessionId,
    pub sender_mailbox: Mailbox,
    pub receiver_mailbox: Mailbox,
}

impl PayjoinSession {
    pub fn new(id: SessionId, sender_mailbox: Mailbox, receiver_mailbox: Mailbox) -> Self {
        Self {
            id,
            sender_mailbox,
            receiver_mailbox,
        }
    }
}
