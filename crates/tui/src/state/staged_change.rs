use agent_finance_core::{
    Market, OrderKind, OrderSide, TimeInForce,
    intent::IntentStatus,
    submit::{SubmitIntentKind, SubmitMode},
};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StagedChange {
    id: String,
    default_mode: SubmitMode,
    state: StagedChangeState,
    subject: StagedChangeSubject,
}

impl StagedChange {
    #[cfg(test)]
    fn from_request(request: StagedChangeRequest, default_mode: SubmitMode) -> Self {
        Self {
            id: request.id,
            default_mode,
            state: StagedChangeState::Draft,
            subject: request.subject,
        }
    }

    #[cfg(test)]
    fn state(&self) -> &StagedChangeState {
        &self.state
    }

    pub fn apply(&mut self, event: StagedChangeEvent) -> bool {
        if matches!(event, StagedChangeEvent::LiveSubmitStarted { .. })
            && self.default_mode != SubmitMode::Live
        {
            return false;
        }
        let Some(next) = self.state.next(event) else {
            return false;
        };
        self.state = next;
        true
    }

    fn disable_live(&mut self) -> bool {
        if self.default_mode != SubmitMode::Live || !self.state.can_disable_live() {
            return false;
        }
        self.default_mode = SubmitMode::DryRun;
        self.state = StagedChangeState::Abandoned;
        true
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StagedChangeRequest {
    pub id: String,
    pub subject: StagedChangeSubject,
}

impl StagedChangeRequest {
    #[cfg(test)]
    pub fn text(id: &str, intent_kind: SubmitIntentKind, summary: &str) -> Self {
        Self {
            id: id.to_string(),
            subject: StagedChangeSubject::Text {
                intent_kind,
                summary: summary.to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum StagedChangeSubject {
    OrderTicket(OrderTicketReview),
    #[cfg(test)]
    Text {
        intent_kind: SubmitIntentKind,
        summary: String,
    },
}

impl StagedChangeSubject {
    fn intent_kind(&self) -> SubmitIntentKind {
        match self {
            Self::OrderTicket(_) => SubmitIntentKind::Order,
            #[cfg(test)]
            Self::Text { intent_kind, .. } => *intent_kind,
        }
    }

    fn summary(&self) -> String {
        match self {
            Self::OrderTicket(review) => review.summary(),
            #[cfg(test)]
            Self::Text { summary, .. } => summary.clone(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct OrderTicketReview {
    pub symbol: String,
    pub profile: String,
    pub market: Market,
    pub side: OrderSide,
    pub kind: OrderKind,
    pub quantity: String,
    pub price: Option<String>,
    pub time_in_force: TimeInForce,
    pub reduce_only: bool,
    pub effective_mode: SubmitMode,
}

impl OrderTicketReview {
    pub fn summary(&self) -> String {
        format!(
            "{} {} {} {} {} @ {} {}{}",
            self.side,
            self.quantity,
            self.symbol,
            self.market,
            self.kind,
            self.price.as_deref().unwrap_or("market"),
            self.time_in_force,
            if self.reduce_only { " reduce-only" } else { "" }
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "staged change events are consumed when staged change panels bind the change workflow"
    )
)]
pub enum StagedChangeEvent {
    ValidationStarted,
    ValidationReady,
    ConfirmationRequested,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "confirmation backtracking is reserved for the first write panel binding"
        )
    )]
    ReturnedToReady,
    IntentCreated {
        intent_id: String,
    },
    NonConsumingFinished {
        intent_id: String,
        mode: SubmitMode,
    },
    PreflightFailed {
        intent_id: String,
        attempted_mode: SubmitMode,
    },
    LiveSubmitStarted {
        intent_id: String,
    },
    LiveSubmitSucceeded {
        intent_id: String,
    },
    LiveSubmitFailed {
        intent_id: String,
    },
    FailedBeforeIntent,
    Abandoned,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum StagedChangeState {
    Draft,
    Validating,
    Ready,
    Confirming,
    IntentCreated {
        intent_id: String,
    },
    NonConsumingCompleted {
        intent_id: String,
        mode: NonConsumingMode,
    },
    PreflightFailed {
        intent_id: String,
        attempted_mode: SubmitMode,
    },
    LiveSubmitting {
        intent_id: String,
    },
    LiveSubmitted {
        intent_id: String,
    },
    FailedBeforeIntent,
    IntentFailed {
        intent_id: String,
    },
    Abandoned,
}

impl StagedChangeState {
    fn next(&self, event: StagedChangeEvent) -> Option<Self> {
        match (self, event) {
            (Self::Draft, StagedChangeEvent::ValidationStarted) => Some(Self::Validating),
            (Self::Draft, StagedChangeEvent::Abandoned) => Some(Self::Abandoned),
            (Self::Validating, StagedChangeEvent::ValidationReady) => Some(Self::Ready),
            (Self::Validating, StagedChangeEvent::FailedBeforeIntent) => {
                Some(Self::FailedBeforeIntent)
            }
            (Self::Validating, StagedChangeEvent::Abandoned) => Some(Self::Abandoned),
            (Self::Ready, StagedChangeEvent::ConfirmationRequested) => Some(Self::Confirming),
            (Self::Ready, StagedChangeEvent::FailedBeforeIntent) => Some(Self::FailedBeforeIntent),
            (Self::Ready, StagedChangeEvent::Abandoned) => Some(Self::Abandoned),
            (Self::Confirming, StagedChangeEvent::ReturnedToReady) => Some(Self::Ready),
            (Self::Confirming, StagedChangeEvent::FailedBeforeIntent) => {
                Some(Self::FailedBeforeIntent)
            }
            (Self::Confirming, StagedChangeEvent::Abandoned) => Some(Self::Abandoned),
            (Self::Confirming, StagedChangeEvent::IntentCreated { intent_id }) => {
                Some(Self::IntentCreated { intent_id })
            }
            (
                Self::IntentCreated { intent_id },
                StagedChangeEvent::NonConsumingFinished {
                    intent_id: next_id,
                    mode,
                },
            ) if intent_id == &next_id => {
                let mode = NonConsumingMode::from_submit_mode(mode)?;
                Some(Self::NonConsumingCompleted {
                    intent_id: next_id,
                    mode,
                })
            }
            (
                Self::IntentCreated { intent_id },
                StagedChangeEvent::PreflightFailed {
                    intent_id: next_id,
                    attempted_mode,
                },
            ) if intent_id == &next_id => Some(Self::PreflightFailed {
                intent_id: next_id,
                attempted_mode,
            }),
            (
                Self::IntentCreated { intent_id }
                | Self::NonConsumingCompleted { intent_id, .. }
                | Self::PreflightFailed { intent_id, .. },
                StagedChangeEvent::LiveSubmitStarted { intent_id: next_id },
            ) if intent_id == &next_id => Some(Self::LiveSubmitting { intent_id: next_id }),
            (
                Self::NonConsumingCompleted { .. } | Self::PreflightFailed { .. },
                StagedChangeEvent::Abandoned,
            ) => Some(Self::Abandoned),
            (
                Self::LiveSubmitting { intent_id },
                StagedChangeEvent::LiveSubmitSucceeded { intent_id: next_id },
            ) if intent_id == &next_id => Some(Self::LiveSubmitted { intent_id: next_id }),
            (
                Self::LiveSubmitting { intent_id },
                StagedChangeEvent::LiveSubmitFailed { intent_id: next_id },
            ) if intent_id == &next_id => Some(Self::IntentFailed { intent_id: next_id }),
            _ => None,
        }
    }

    fn stage(&self) -> StagedChangeStage {
        match self {
            Self::Draft => StagedChangeStage::Draft,
            Self::Validating => StagedChangeStage::Validating,
            Self::Ready => StagedChangeStage::Ready,
            Self::Confirming => StagedChangeStage::Confirming,
            Self::IntentCreated { .. } => StagedChangeStage::IntentCreated,
            Self::NonConsumingCompleted {
                mode: NonConsumingMode::DryRun,
                ..
            } => StagedChangeStage::DryRunCompleted,
            Self::NonConsumingCompleted {
                mode: NonConsumingMode::Test,
                ..
            } => StagedChangeStage::TestCompleted,
            Self::PreflightFailed {
                attempted_mode: SubmitMode::DryRun,
                ..
            } => StagedChangeStage::DryRunFailed,
            Self::PreflightFailed {
                attempted_mode: SubmitMode::Test,
                ..
            } => StagedChangeStage::TestFailed,
            Self::PreflightFailed {
                attempted_mode: SubmitMode::Live,
                ..
            } => StagedChangeStage::LivePreflightFailed,
            Self::LiveSubmitting { .. } => StagedChangeStage::LiveSubmitting,
            Self::LiveSubmitted { .. } => StagedChangeStage::LiveSubmitted,
            Self::FailedBeforeIntent => StagedChangeStage::FailedBeforeIntent,
            Self::IntentFailed { .. } => StagedChangeStage::IntentFailed,
            Self::Abandoned => StagedChangeStage::Abandoned,
        }
    }

    fn mode(&self, default_mode: SubmitMode) -> SubmitMode {
        match self {
            Self::NonConsumingCompleted { mode, .. } => mode.submit_mode(),
            Self::PreflightFailed { attempted_mode, .. } => *attempted_mode,
            Self::LiveSubmitting { .. }
            | Self::LiveSubmitted { .. }
            | Self::IntentFailed { .. } => SubmitMode::Live,
            _ => default_mode,
        }
    }

    fn intent_status(&self) -> Option<IntentStatus> {
        match self {
            Self::LiveSubmitted { .. } => Some(IntentStatus::Submitted),
            Self::IntentFailed { .. } => Some(IntentStatus::Failed),
            _ => None,
        }
    }

    fn intent_id(&self) -> Option<&str> {
        match self {
            Self::IntentCreated { intent_id }
            | Self::NonConsumingCompleted { intent_id, .. }
            | Self::PreflightFailed { intent_id, .. }
            | Self::LiveSubmitting { intent_id }
            | Self::LiveSubmitted { intent_id }
            | Self::IntentFailed { intent_id } => Some(intent_id),
            _ => None,
        }
    }

    fn accepts_replacement(&self) -> bool {
        matches!(
            self,
            Self::Draft | Self::FailedBeforeIntent | Self::IntentFailed { .. } | Self::Abandoned
        )
    }

    fn blocks_close(&self) -> bool {
        matches!(self, Self::LiveSubmitting { .. })
    }

    fn can_disable_live(&self) -> bool {
        !matches!(
            self,
            Self::LiveSubmitting { .. } | Self::LiveSubmitted { .. }
        )
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum NonConsumingMode {
    DryRun,
    Test,
}

impl NonConsumingMode {
    fn from_submit_mode(mode: SubmitMode) -> Option<Self> {
        match mode {
            SubmitMode::DryRun => Some(Self::DryRun),
            SubmitMode::Test => Some(Self::Test),
            SubmitMode::Live => None,
        }
    }

    fn submit_mode(self) -> SubmitMode {
        match self {
            Self::DryRun => SubmitMode::DryRun,
            Self::Test => SubmitMode::Test,
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct StagedChanges {
    changes: Vec<StagedChange>,
}

impl StagedChanges {
    pub(super) fn views(&self) -> Vec<StagedChangeView> {
        self.changes.iter().map(StagedChangeView::from).collect()
    }

    pub(super) fn open(
        &mut self,
        request: StagedChangeRequest,
        mode: SubmitMode,
    ) -> OpenStagedChangeResult {
        self.open_with(request, mode, StagedChangeState::Draft)
    }

    pub(super) fn open_ready(
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

    pub(super) fn apply(&mut self, id: &str, event: StagedChangeEvent) -> TransitionResult {
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

    pub(super) fn disable_live(&mut self) -> usize {
        let mut disabled = 0;
        for change in &mut self.changes {
            if change.disable_live() {
                disabled += 1;
            }
        }
        disabled
    }

    pub(super) fn close(&mut self, id: &str) -> CloseStagedChangeResult {
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
pub(super) enum OpenStagedChangeResult {
    Opened,
    Rejected,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) enum CloseStagedChangeResult {
    Closed,
    Missing,
    Rejected { current: String },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) enum TransitionResult {
    Applied,
    Missing,
    Rejected {
        current: String,
        event: StagedChangeEvent,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StagedChangeStage {
    Draft,
    Validating,
    Ready,
    Confirming,
    IntentCreated,
    DryRunCompleted,
    TestCompleted,
    DryRunFailed,
    TestFailed,
    LivePreflightFailed,
    LiveSubmitting,
    LiveSubmitted,
    FailedBeforeIntent,
    IntentFailed,
    Abandoned,
}

impl StagedChangeStage {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Validating => "validating",
            Self::Ready => "ready",
            Self::Confirming => "confirming",
            Self::IntentCreated => "intent-created",
            Self::DryRunCompleted => "dry-run-completed",
            Self::TestCompleted => "test-completed",
            Self::DryRunFailed => "dry-run-failed",
            Self::TestFailed => "test-failed",
            Self::LivePreflightFailed => "live-preflight-failed",
            Self::LiveSubmitting => "live-submitting",
            Self::LiveSubmitted => "live-submitted",
            Self::FailedBeforeIntent => "failed-before-intent",
            Self::IntentFailed => "intent-failed",
            Self::Abandoned => "abandoned",
        }
    }
}

impl fmt::Display for StagedChangeStage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct StagedChangeView {
    pub id: String,
    pub intent_kind: SubmitIntentKind,
    pub stage: StagedChangeStage,
    pub mode: SubmitMode,
    pub intent_id: Option<String>,
    pub intent_status: Option<IntentStatus>,
    pub summary: String,
    pub subject: StagedChangeSubject,
}

impl From<&StagedChange> for StagedChangeView {
    fn from(change: &StagedChange) -> Self {
        Self {
            id: change.id.clone(),
            intent_kind: change.subject.intent_kind(),
            stage: change.state.stage(),
            mode: change.state.mode(change.default_mode),
            intent_id: change.state.intent_id().map(ToString::to_string),
            intent_status: change.state.intent_status(),
            summary: change.subject.summary(),
            subject: change.subject.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: &str) -> StagedChangeRequest {
        StagedChangeRequest::text(id, SubmitIntentKind::Order, "Buy BTCUSDT")
    }

    fn apply_all(change: &mut StagedChange, events: impl IntoIterator<Item = StagedChangeEvent>) {
        for event in events {
            assert!(change.apply(event));
        }
    }

    #[test]
    fn staged_change_events_allow_forward_workflow_and_reject_unsafe_jumps() {
        let mut change = StagedChange::from_request(request("change-1"), SubmitMode::DryRun);

        assert!(!change.apply(StagedChangeEvent::LiveSubmitSucceeded {
            intent_id: "intent-1".to_string(),
        }));
        assert_eq!(change.state(), &StagedChangeState::Draft);

        apply_all(
            &mut change,
            [
                StagedChangeEvent::ValidationStarted,
                StagedChangeEvent::ValidationReady,
                StagedChangeEvent::ConfirmationRequested,
                StagedChangeEvent::IntentCreated {
                    intent_id: "intent-1".to_string(),
                },
            ],
        );
        assert_eq!(
            change.state(),
            &StagedChangeState::IntentCreated {
                intent_id: "intent-1".to_string()
            }
        );
    }

    #[test]
    fn staged_change_events_reject_intent_id_mismatches() {
        let mut change = StagedChange::from_request(request("change-1"), SubmitMode::Live);
        apply_all(
            &mut change,
            [
                StagedChangeEvent::ValidationStarted,
                StagedChangeEvent::ValidationReady,
                StagedChangeEvent::ConfirmationRequested,
                StagedChangeEvent::IntentCreated {
                    intent_id: "intent-1".to_string(),
                },
            ],
        );

        assert!(!change.apply(StagedChangeEvent::LiveSubmitStarted {
            intent_id: "intent-2".to_string(),
        }));
        assert!(change.apply(StagedChangeEvent::LiveSubmitStarted {
            intent_id: "intent-1".to_string(),
        }));
        assert!(!change.apply(StagedChangeEvent::LiveSubmitFailed {
            intent_id: "intent-2".to_string(),
        }));
        assert!(change.apply(StagedChangeEvent::LiveSubmitFailed {
            intent_id: "intent-1".to_string(),
        }));
    }

    #[test]
    fn non_consuming_completion_does_not_claim_core_submission_status() {
        let mut change = StagedChange::from_request(request("change-1"), SubmitMode::DryRun);
        apply_all(
            &mut change,
            [
                StagedChangeEvent::ValidationStarted,
                StagedChangeEvent::ValidationReady,
                StagedChangeEvent::ConfirmationRequested,
                StagedChangeEvent::IntentCreated {
                    intent_id: "intent-1".to_string(),
                },
                StagedChangeEvent::NonConsumingFinished {
                    intent_id: "intent-1".to_string(),
                    mode: SubmitMode::DryRun,
                },
            ],
        );

        let view = StagedChangeView::from(&change);
        assert_eq!(view.intent_id.as_deref(), Some("intent-1"));
        assert_eq!(view.intent_status, None);
        assert_eq!(view.mode, SubmitMode::DryRun);
        assert!(!change.apply(StagedChangeEvent::LiveSubmitStarted {
            intent_id: "intent-1".to_string(),
        }));
    }

    #[test]
    fn test_completion_can_continue_to_live_without_claiming_core_submission() {
        let mut change = StagedChange::from_request(request("change-1"), SubmitMode::Test);
        apply_all(
            &mut change,
            [
                StagedChangeEvent::ValidationStarted,
                StagedChangeEvent::ValidationReady,
                StagedChangeEvent::ConfirmationRequested,
                StagedChangeEvent::IntentCreated {
                    intent_id: "intent-1".to_string(),
                },
                StagedChangeEvent::NonConsumingFinished {
                    intent_id: "intent-1".to_string(),
                    mode: SubmitMode::Test,
                },
            ],
        );

        let view = StagedChangeView::from(&change);
        assert_eq!(view.intent_status, None);
        assert_eq!(view.mode, SubmitMode::Test);
        assert!(!change.apply(StagedChangeEvent::LiveSubmitStarted {
            intent_id: "intent-1".to_string(),
        }));
    }

    #[test]
    fn only_live_mode_changes_can_start_live_submission() {
        for mode in [SubmitMode::DryRun, SubmitMode::Test] {
            let mut change = StagedChange::from_request(request("change-1"), mode);
            apply_all(
                &mut change,
                [
                    StagedChangeEvent::ValidationStarted,
                    StagedChangeEvent::ValidationReady,
                    StagedChangeEvent::ConfirmationRequested,
                    StagedChangeEvent::IntentCreated {
                        intent_id: "intent-1".to_string(),
                    },
                ],
            );

            assert!(!change.apply(StagedChangeEvent::LiveSubmitStarted {
                intent_id: "intent-1".to_string(),
            }));
            assert_eq!(StagedChangeView::from(&change).mode, mode);
        }

        let mut live = StagedChange::from_request(request("change-1"), SubmitMode::Live);
        apply_all(
            &mut live,
            [
                StagedChangeEvent::ValidationStarted,
                StagedChangeEvent::ValidationReady,
                StagedChangeEvent::ConfirmationRequested,
                StagedChangeEvent::IntentCreated {
                    intent_id: "intent-1".to_string(),
                },
            ],
        );

        assert!(live.apply(StagedChangeEvent::LiveSubmitStarted {
            intent_id: "intent-1".to_string(),
        }));
    }

    #[test]
    fn live_preflight_failures_keep_core_intent_status_empty() {
        let mut change = StagedChange::from_request(request("change-1"), SubmitMode::Live);
        apply_all(
            &mut change,
            [
                StagedChangeEvent::ValidationStarted,
                StagedChangeEvent::ValidationReady,
                StagedChangeEvent::ConfirmationRequested,
                StagedChangeEvent::IntentCreated {
                    intent_id: "intent-1".to_string(),
                },
                StagedChangeEvent::PreflightFailed {
                    intent_id: "intent-1".to_string(),
                    attempted_mode: SubmitMode::Live,
                },
            ],
        );

        let view = StagedChangeView::from(&change);
        assert_eq!(view.stage, StagedChangeStage::LivePreflightFailed);
        assert_eq!(view.intent_status, None);
        assert_eq!(view.mode, SubmitMode::Live);
        assert!(change.apply(StagedChangeEvent::LiveSubmitStarted {
            intent_id: "intent-1".to_string(),
        }));
    }

    #[test]
    fn validation_failures_before_intent_do_not_claim_core_intent_status() {
        let mut change = StagedChange::from_request(request("change-1"), SubmitMode::DryRun);

        assert!(change.apply(StagedChangeEvent::ValidationStarted));
        assert!(change.apply(StagedChangeEvent::FailedBeforeIntent));

        let view = StagedChangeView::from(&change);
        assert_eq!(view.intent_id, None);
        assert_eq!(view.intent_status, None);
    }

    #[test]
    fn live_submission_lifecycle_is_the_only_core_submitted_status_source() {
        let mut change = StagedChange::from_request(request("change-1"), SubmitMode::Live);
        for event in [
            StagedChangeEvent::ValidationStarted,
            StagedChangeEvent::ValidationReady,
            StagedChangeEvent::ConfirmationRequested,
            StagedChangeEvent::IntentCreated {
                intent_id: "intent-1".to_string(),
            },
            StagedChangeEvent::LiveSubmitStarted {
                intent_id: "intent-1".to_string(),
            },
        ] {
            assert!(change.apply(event));
            assert_eq!(StagedChangeView::from(&change).intent_status, None);
        }

        assert!(change.apply(StagedChangeEvent::LiveSubmitSucceeded {
            intent_id: "intent-1".to_string(),
        }));
        assert_eq!(
            StagedChangeView::from(&change).intent_status,
            Some(IntentStatus::Submitted)
        );
    }

    #[test]
    fn staged_change_events_allow_abandoning_before_intent_creation() {
        let mut change = StagedChange::from_request(request("change-1"), SubmitMode::DryRun);

        assert!(change.apply(StagedChangeEvent::ValidationStarted));
        assert!(change.apply(StagedChangeEvent::Abandoned));
        assert!(!change.apply(StagedChangeEvent::ValidationReady));
    }

    #[test]
    fn confirmation_can_return_to_ready_before_intent_creation() {
        let mut change = StagedChange::from_request(request("change-1"), SubmitMode::DryRun);
        apply_all(
            &mut change,
            [
                StagedChangeEvent::ValidationStarted,
                StagedChangeEvent::ValidationReady,
                StagedChangeEvent::ConfirmationRequested,
                StagedChangeEvent::ReturnedToReady,
            ],
        );

        assert_eq!(change.state(), &StagedChangeState::Ready);
        assert!(change.apply(StagedChangeEvent::ConfirmationRequested));
        assert!(change.apply(StagedChangeEvent::IntentCreated {
            intent_id: "intent-1".to_string(),
        }));
    }

    #[test]
    fn staged_changes_do_not_replace_active_changes() {
        let mut changes = StagedChanges::default();

        assert_eq!(
            changes.open(request("change-1"), SubmitMode::DryRun),
            OpenStagedChangeResult::Opened
        );
        assert_eq!(
            changes.apply("change-1", StagedChangeEvent::ValidationStarted),
            TransitionResult::Applied
        );
        assert_eq!(
            changes.open(request("change-1"), SubmitMode::Live),
            OpenStagedChangeResult::Rejected
        );

        let view = changes.views().pop().unwrap();
        assert_eq!(view.mode, SubmitMode::DryRun);
        assert_eq!(view.stage, StagedChangeStage::Validating);
    }

    #[test]
    fn draft_changes_can_be_replaced_before_validation_starts() {
        let mut changes = StagedChanges::default();

        assert_eq!(
            changes.open(request("change-1"), SubmitMode::DryRun),
            OpenStagedChangeResult::Opened
        );
        assert_eq!(
            changes.open(request("change-1"), SubmitMode::Live),
            OpenStagedChangeResult::Opened
        );

        let view = changes.views().pop().unwrap();
        assert_eq!(view.mode, SubmitMode::Live);
        assert_eq!(view.stage, StagedChangeStage::Draft);
    }

    #[test]
    fn disabling_live_abandons_pending_live_changes_but_keeps_submitting_changes() {
        let mut changes = StagedChanges::default();
        assert_eq!(
            changes.open(request("pending"), SubmitMode::Live),
            OpenStagedChangeResult::Opened
        );
        assert_eq!(
            changes.open(request("submitting"), SubmitMode::Live),
            OpenStagedChangeResult::Opened
        );
        for event in [
            StagedChangeEvent::ValidationStarted,
            StagedChangeEvent::ValidationReady,
            StagedChangeEvent::ConfirmationRequested,
            StagedChangeEvent::IntentCreated {
                intent_id: "intent-1".to_string(),
            },
            StagedChangeEvent::LiveSubmitStarted {
                intent_id: "intent-1".to_string(),
            },
        ] {
            assert_eq!(
                changes.apply("submitting", event),
                TransitionResult::Applied
            );
        }

        assert_eq!(changes.disable_live(), 1);
        let views = changes.views();
        let pending = views
            .iter()
            .find(|view| view.id == "pending")
            .expect("pending view");
        let submitting = views
            .iter()
            .find(|view| view.id == "submitting")
            .expect("submitting view");

        assert_eq!(pending.stage, StagedChangeStage::Abandoned);
        assert_eq!(pending.mode, SubmitMode::DryRun);
        assert_eq!(submitting.stage, StagedChangeStage::LiveSubmitting);
        assert_eq!(submitting.mode, SubmitMode::Live);
    }
}
