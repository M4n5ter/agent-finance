mod collection;
mod subject;
mod workflow;

#[cfg(test)]
mod tests;

pub(super) use collection::{
    CloseStagedChangeResult, OpenStagedChangeResult, QueueSubmitResult, StagedChanges,
    TransitionResult,
};
pub use subject::{
    CancelReview, OrderTicketReview, StagedChangeRequest, StagedChangeSubject, StagedSubmitRequest,
    TransferReview,
};
#[cfg(test)]
pub use workflow::StagedChangeStage;
pub use workflow::{StagedChangeEvent, StagedChangeView};

#[cfg(test)]
pub(crate) use workflow::{StagedChange, StagedChangeState};
