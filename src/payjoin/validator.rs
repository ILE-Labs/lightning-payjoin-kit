use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalValidation {
    pub accepted: bool,
}

pub trait ProposalValidator {
    fn validate(&self) -> Result<ProposalValidation>;
}
