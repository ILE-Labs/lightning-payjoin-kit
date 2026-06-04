use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::directory::DirectoryClient;
use crate::error::{Error, Result};
use crate::payjoin::{Mailbox, PayjoinPayload, PayjoinPayloadKind, PayjoinSession, SessionId};

#[derive(Debug, Clone, Default)]
pub struct MockDirectory {
    inner: Rc<RefCell<MockDirectoryState>>,
}

#[derive(Debug, Default)]
struct MockDirectoryState {
    next_id: u64,
    latest_payloads: BTreeMap<SessionId, PayjoinPayload>,
    payloads_by_kind: BTreeMap<(SessionId, PayjoinPayloadKind), PayjoinPayload>,
}

impl DirectoryClient for MockDirectory {
    fn create_session(&mut self) -> Result<PayjoinSession> {
        let mut inner = self.inner.borrow_mut();
        inner.next_id += 1;
        let id = SessionId::new(format!("mock-session-{}", inner.next_id));
        Ok(PayjoinSession::new(
            id,
            Mailbox::new(format!("sender-{}", inner.next_id)),
            Mailbox::new(format!("receiver-{}", inner.next_id)),
        ))
    }

    fn post_payload(&mut self, session_id: &SessionId, payload: PayjoinPayload) -> Result<()> {
        if session_id.as_str().is_empty() {
            return Err(Error::Directory("empty session id".to_owned()));
        }
        let mut inner = self.inner.borrow_mut();
        inner
            .payloads_by_kind
            .insert((session_id.clone(), payload.kind), payload.clone());
        inner.latest_payloads.insert(session_id.clone(), payload);
        Ok(())
    }

    fn get_payload(&self, session_id: &SessionId) -> Result<Option<PayjoinPayload>> {
        Ok(self.inner.borrow().latest_payloads.get(session_id).cloned())
    }

    fn get_payload_by_kind(
        &self,
        session_id: &SessionId,
        kind: PayjoinPayloadKind,
    ) -> Result<Option<PayjoinPayload>> {
        Ok(self
            .inner
            .borrow()
            .payloads_by_kind
            .get(&(session_id.clone(), kind))
            .cloned())
    }
}
