#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FundingState {
    Idle,
    OriginalPrepared,
    ProposalRequested,
    ProposalReceived,
    ProposalValidated,
    FinalSigned,
    BroadcastReady,
    FallbackReady,
    Failed,
}
