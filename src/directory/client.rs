use crate::error::Result;
use crate::payjoin::{PayjoinPayload, PayjoinSession, SessionId};

pub trait DirectoryClient {
    fn create_session(&mut self) -> Result<PayjoinSession>;
    fn post_payload(&mut self, session_id: &SessionId, payload: PayjoinPayload) -> Result<()>;
    fn get_payload(&self, session_id: &SessionId) -> Result<Option<PayjoinPayload>>;
}
