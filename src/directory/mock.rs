use std::collections::BTreeMap;

use crate::directory::DirectoryClient;
use crate::error::{Error, Result};
use crate::payjoin::{Mailbox, PayjoinPayload, PayjoinSession, SessionId};

#[derive(Debug, Default)]
pub struct MockDirectory {
    next_id: u64,
    payloads: BTreeMap<SessionId, PayjoinPayload>,
}

impl DirectoryClient for MockDirectory {
    fn create_session(&mut self) -> Result<PayjoinSession> {
        self.next_id += 1;
        let id = SessionId::new(format!("mock-session-{}", self.next_id));
        Ok(PayjoinSession::new(
            id,
            Mailbox::new(format!("sender-{}", self.next_id)),
            Mailbox::new(format!("receiver-{}", self.next_id)),
        ))
    }

    fn post_payload(&mut self, session_id: &SessionId, payload: PayjoinPayload) -> Result<()> {
        if session_id.as_str().is_empty() {
            return Err(Error::Directory("empty session id".to_owned()));
        }
        self.payloads.insert(session_id.clone(), payload);
        Ok(())
    }

    fn get_payload(&self, session_id: &SessionId) -> Result<Option<PayjoinPayload>> {
        Ok(self.payloads.get(session_id).cloned())
    }
}
