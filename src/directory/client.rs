use crate::error::Result;
use crate::payjoin::{PayjoinPayload, PayjoinPayloadKind, PayjoinSession, SessionId};

pub trait DirectoryClient {
    fn create_session(&mut self) -> Result<PayjoinSession>;
    fn post_payload(&mut self, session_id: &SessionId, payload: PayjoinPayload) -> Result<()>;
    fn get_payload(&self, session_id: &SessionId) -> Result<Option<PayjoinPayload>>;
    fn get_payload_by_kind(
        &self,
        session_id: &SessionId,
        kind: PayjoinPayloadKind,
    ) -> Result<Option<PayjoinPayload>>;
}
