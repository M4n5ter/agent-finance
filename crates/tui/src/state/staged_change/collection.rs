use agent_finance_core::submit::SubmitMode;

use super::subject::{StagedChangeRequest, StagedSubmitRequest};
use super::workflow::{
    StagedChange, StagedChangeEvent, StagedChangeStage, StagedChangeState, StagedChangeView,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StagedChanges {
    changes: Vec<StagedChange>,
}

impl StagedChanges {
    pub(crate) fn views(&self) -> Vec<StagedChangeView> {
        self.changes.iter().map(StagedChangeView::from).collect()
    }

    pub(crate) fn queue_next_submit(&mut self) -> QueueSubmitResult {
        let Some((change, request)) = self.changes.iter_mut().find_map(|change| {
            if change.state.stage() != StagedChangeStage::Ready {
                return None;
            }
            let request = change
                .subject
                .submit_request(change.id.clone(), change.state.mode(change.default_mode))?;
            Some((change, request))
        }) else {
            return QueueSubmitResult::Missing;
        };
        if change.apply(StagedChangeEvent::SubmitQueued) {
            QueueSubmitResult::Queued(request)
        } else {
            QueueSubmitResult::Rejected {
                current: format!("{:?}", change.state),
            }
        }
    }

    pub(crate) fn open(
        &mut self,
        request: StagedChangeRequest,
        mode: SubmitMode,
    ) -> OpenStagedChangeResult {
        self.open_with(request, mode, StagedChangeState::Draft)
    }

    pub(crate) fn open_ready(
        &mut self,
        request: StagedChangeRequest,
        mode: SubmitMode,
    ) -> OpenStagedChangeResult {
        self.open_with(request, mode, StagedChangeState::Ready)
    }

    fn open_with(
        &mut self,
        request: StagedChangeRequest,
        mode: SubmitMode,
        state: StagedChangeState,
    ) -> OpenStagedChangeResult {
        if self
            .changes
            .iter()
            .any(|change| change.id == request.id && !change.state.accepts_replacement())
        {
            return OpenStagedChangeResult::Rejected;
        }

        self.changes
            .retain(|change| change.id != request.id || !change.state.accepts_replacement());
        self.changes.push(StagedChange {
            id: request.id,
            default_mode: mode,
            state,
            subject: request.subject,
        });
        OpenStagedChangeResult::Opened
    }

    pub(crate) fn apply(&mut self, id: &str, event: StagedChangeEvent) -> TransitionResult {
        let Some(change) = self.changes.iter_mut().find(|change| change.id == id) else {
            return TransitionResult::Missing;
        };
        let previous = format!("{:?}", change.state);
        if change.apply(event.clone()) {
            TransitionResult::Applied
        } else {
            TransitionResult::Rejected {
                current: previous,
                event,
            }
        }
    }

    pub(crate) fn disable_live(&mut self) -> usize {
        let mut disabled = 0;
        for change in &mut self.changes {
            if change.disable_live() {
                disabled += 1;
            }
        }
        disabled
    }

    pub(crate) fn close(&mut self, id: &str) -> CloseStagedChangeResult {
        let Some(index) = self.changes.iter().position(|change| change.id == id) else {
            return CloseStagedChangeResult::Missing;
        };
        let current = &self.changes[index].state;
        if current.blocks_close() {
            return CloseStagedChangeResult::Rejected {
                current: format!("{:?}", current),
            };
        }

        self.changes.remove(index);
        CloseStagedChangeResult::Closed
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum OpenStagedChangeResult {
    Opened,
    Rejected,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum CloseStagedChangeResult {
    Closed,
    Missing,
    Rejected { current: String },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum QueueSubmitResult {
    Queued(StagedSubmitRequest),
    Missing,
    Rejected { current: String },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum TransitionResult {
    Applied,
    Missing,
    Rejected {
        current: String,
        event: StagedChangeEvent,
    },
}
