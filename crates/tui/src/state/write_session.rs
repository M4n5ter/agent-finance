use agent_finance_core::intent::IntentStatus;
use agent_finance_core::submit::{SubmitIntentKind, SubmitMode};
use serde::Serialize;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WriteSession {
    id: String,
    intent_kind: SubmitIntentKind,
    default_mode: SubmitMode,
    state: WriteSessionState,
    summary: String,
}

impl WriteSession {
    fn from_request(request: WriteSessionRequest, default_mode: SubmitMode) -> Self {
        Self {
            id: request.id,
            intent_kind: request.intent_kind,
            default_mode,
            state: WriteSessionState::Draft,
            summary: request.summary,
        }
    }

    #[cfg(test)]
    fn state(&self) -> &WriteSessionState {
        &self.state
    }

    pub fn apply(&mut self, event: WriteSessionEvent) -> bool {
        if matches!(event, WriteSessionEvent::LiveSubmitStarted { .. })
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
        self.state = WriteSessionState::Abandoned;
        true
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WriteSessionRequest {
    pub id: String,
    pub intent_kind: SubmitIntentKind,
    pub summary: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "write session events are consumed when write panels bind the session workflow"
    )
)]
pub enum WriteSessionEvent {
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
enum WriteSessionState {
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

impl WriteSessionState {
    fn next(&self, event: WriteSessionEvent) -> Option<Self> {
        match (self, event) {
            (Self::Draft, WriteSessionEvent::ValidationStarted) => Some(Self::Validating),
            (Self::Draft, WriteSessionEvent::Abandoned) => Some(Self::Abandoned),
            (Self::Validating, WriteSessionEvent::ValidationReady) => Some(Self::Ready),
            (Self::Validating, WriteSessionEvent::FailedBeforeIntent) => {
                Some(Self::FailedBeforeIntent)
            }
            (Self::Validating, WriteSessionEvent::Abandoned) => Some(Self::Abandoned),
            (Self::Ready, WriteSessionEvent::ConfirmationRequested) => Some(Self::Confirming),
            (Self::Ready, WriteSessionEvent::FailedBeforeIntent) => Some(Self::FailedBeforeIntent),
            (Self::Ready, WriteSessionEvent::Abandoned) => Some(Self::Abandoned),
            (Self::Confirming, WriteSessionEvent::ReturnedToReady) => Some(Self::Ready),
            (Self::Confirming, WriteSessionEvent::FailedBeforeIntent) => {
                Some(Self::FailedBeforeIntent)
            }
            (Self::Confirming, WriteSessionEvent::Abandoned) => Some(Self::Abandoned),
            (Self::Confirming, WriteSessionEvent::IntentCreated { intent_id }) => {
                Some(Self::IntentCreated { intent_id })
            }
            (
                Self::IntentCreated { intent_id },
                WriteSessionEvent::NonConsumingFinished {
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
                WriteSessionEvent::PreflightFailed {
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
                WriteSessionEvent::LiveSubmitStarted { intent_id: next_id },
            ) if intent_id == &next_id => Some(Self::LiveSubmitting { intent_id: next_id }),
            (
                Self::NonConsumingCompleted { .. } | Self::PreflightFailed { .. },
                WriteSessionEvent::Abandoned,
            ) => Some(Self::Abandoned),
            (
                Self::LiveSubmitting { intent_id },
                WriteSessionEvent::LiveSubmitSucceeded { intent_id: next_id },
            ) if intent_id == &next_id => Some(Self::LiveSubmitted { intent_id: next_id }),
            (
                Self::LiveSubmitting { intent_id },
                WriteSessionEvent::LiveSubmitFailed { intent_id: next_id },
            ) if intent_id == &next_id => Some(Self::IntentFailed { intent_id: next_id }),
            _ => None,
        }
    }

    fn stage(&self) -> WriteSessionStage {
        match self {
            Self::Draft => WriteSessionStage::Draft,
            Self::Validating => WriteSessionStage::Validating,
            Self::Ready => WriteSessionStage::Ready,
            Self::Confirming => WriteSessionStage::Confirming,
            Self::IntentCreated { .. } => WriteSessionStage::IntentCreated,
            Self::NonConsumingCompleted {
                mode: NonConsumingMode::DryRun,
                ..
            } => WriteSessionStage::DryRunCompleted,
            Self::NonConsumingCompleted {
                mode: NonConsumingMode::Test,
                ..
            } => WriteSessionStage::TestCompleted,
            Self::PreflightFailed {
                attempted_mode: SubmitMode::DryRun,
                ..
            } => WriteSessionStage::DryRunFailed,
            Self::PreflightFailed {
                attempted_mode: SubmitMode::Test,
                ..
            } => WriteSessionStage::TestFailed,
            Self::PreflightFailed {
                attempted_mode: SubmitMode::Live,
                ..
            } => WriteSessionStage::LivePreflightFailed,
            Self::LiveSubmitting { .. } => WriteSessionStage::LiveSubmitting,
            Self::LiveSubmitted { .. } => WriteSessionStage::LiveSubmitted,
            Self::FailedBeforeIntent => WriteSessionStage::FailedBeforeIntent,
            Self::IntentFailed { .. } => WriteSessionStage::IntentFailed,
            Self::Abandoned => WriteSessionStage::Abandoned,
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
pub struct WriteSessions {
    sessions: Vec<WriteSession>,
}

impl WriteSessions {
    pub(super) fn views(&self) -> Vec<WriteSessionView> {
        self.sessions.iter().map(WriteSessionView::from).collect()
    }

    pub(super) fn open(
        &mut self,
        request: WriteSessionRequest,
        mode: SubmitMode,
    ) -> OpenSessionResult {
        if self
            .sessions
            .iter()
            .any(|session| session.id == request.id && !session.state.accepts_replacement())
        {
            return OpenSessionResult::Rejected;
        }

        self.sessions
            .retain(|session| session.id != request.id || !session.state.accepts_replacement());
        self.sessions
            .push(WriteSession::from_request(request, mode));
        OpenSessionResult::Opened
    }

    pub(super) fn apply(&mut self, id: &str, event: WriteSessionEvent) -> TransitionResult {
        let Some(session) = self.sessions.iter_mut().find(|session| session.id == id) else {
            return TransitionResult::Missing;
        };
        let previous = format!("{:?}", session.state);
        if session.apply(event.clone()) {
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
        for session in &mut self.sessions {
            if session.disable_live() {
                disabled += 1;
            }
        }
        disabled
    }

    pub(super) fn close(&mut self, id: &str) -> CloseSessionResult {
        let Some(index) = self.sessions.iter().position(|session| session.id == id) else {
            return CloseSessionResult::Missing;
        };
        let current = &self.sessions[index].state;
        if current.blocks_close() {
            return CloseSessionResult::Rejected {
                current: format!("{:?}", current),
            };
        }

        self.sessions.remove(index);
        CloseSessionResult::Closed
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) enum OpenSessionResult {
    Opened,
    Rejected,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) enum CloseSessionResult {
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
        event: WriteSessionEvent,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WriteSessionStage {
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

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct WriteSessionView {
    pub id: String,
    pub intent_kind: SubmitIntentKind,
    pub stage: WriteSessionStage,
    pub mode: SubmitMode,
    pub intent_id: Option<String>,
    pub intent_status: Option<IntentStatus>,
    pub summary: String,
}

impl From<&WriteSession> for WriteSessionView {
    fn from(session: &WriteSession) -> Self {
        Self {
            id: session.id.clone(),
            intent_kind: session.intent_kind,
            stage: session.state.stage(),
            mode: session.state.mode(session.default_mode),
            intent_id: session.state.intent_id().map(ToString::to_string),
            intent_status: session.state.intent_status(),
            summary: session.summary.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: &str) -> WriteSessionRequest {
        WriteSessionRequest {
            id: id.to_string(),
            intent_kind: SubmitIntentKind::Order,
            summary: "Buy BTCUSDT".to_string(),
        }
    }

    fn apply_all(session: &mut WriteSession, events: impl IntoIterator<Item = WriteSessionEvent>) {
        for event in events {
            assert!(session.apply(event));
        }
    }

    #[test]
    fn write_session_events_allow_forward_workflow_and_reject_unsafe_jumps() {
        let mut session = WriteSession::from_request(request("session-1"), SubmitMode::DryRun);

        assert!(!session.apply(WriteSessionEvent::LiveSubmitSucceeded {
            intent_id: "intent-1".to_string(),
        }));
        assert_eq!(session.state(), &WriteSessionState::Draft);

        apply_all(
            &mut session,
            [
                WriteSessionEvent::ValidationStarted,
                WriteSessionEvent::ValidationReady,
                WriteSessionEvent::ConfirmationRequested,
                WriteSessionEvent::IntentCreated {
                    intent_id: "intent-1".to_string(),
                },
            ],
        );
        assert_eq!(
            session.state(),
            &WriteSessionState::IntentCreated {
                intent_id: "intent-1".to_string()
            }
        );
    }

    #[test]
    fn write_session_events_reject_intent_id_mismatches() {
        let mut session = WriteSession::from_request(request("session-1"), SubmitMode::Live);
        apply_all(
            &mut session,
            [
                WriteSessionEvent::ValidationStarted,
                WriteSessionEvent::ValidationReady,
                WriteSessionEvent::ConfirmationRequested,
                WriteSessionEvent::IntentCreated {
                    intent_id: "intent-1".to_string(),
                },
            ],
        );

        assert!(!session.apply(WriteSessionEvent::LiveSubmitStarted {
            intent_id: "intent-2".to_string(),
        }));
        assert!(session.apply(WriteSessionEvent::LiveSubmitStarted {
            intent_id: "intent-1".to_string(),
        }));
        assert!(!session.apply(WriteSessionEvent::LiveSubmitFailed {
            intent_id: "intent-2".to_string(),
        }));
        assert!(session.apply(WriteSessionEvent::LiveSubmitFailed {
            intent_id: "intent-1".to_string(),
        }));
    }

    #[test]
    fn non_consuming_completion_does_not_claim_core_submission_status() {
        let mut session = WriteSession::from_request(request("session-1"), SubmitMode::DryRun);
        apply_all(
            &mut session,
            [
                WriteSessionEvent::ValidationStarted,
                WriteSessionEvent::ValidationReady,
                WriteSessionEvent::ConfirmationRequested,
                WriteSessionEvent::IntentCreated {
                    intent_id: "intent-1".to_string(),
                },
                WriteSessionEvent::NonConsumingFinished {
                    intent_id: "intent-1".to_string(),
                    mode: SubmitMode::DryRun,
                },
            ],
        );

        let view = WriteSessionView::from(&session);
        assert_eq!(view.intent_id.as_deref(), Some("intent-1"));
        assert_eq!(view.intent_status, None);
        assert_eq!(view.mode, SubmitMode::DryRun);
        assert!(!session.apply(WriteSessionEvent::LiveSubmitStarted {
            intent_id: "intent-1".to_string(),
        }));
    }

    #[test]
    fn test_completion_can_continue_to_live_without_claiming_core_submission() {
        let mut session = WriteSession::from_request(request("session-1"), SubmitMode::Test);
        apply_all(
            &mut session,
            [
                WriteSessionEvent::ValidationStarted,
                WriteSessionEvent::ValidationReady,
                WriteSessionEvent::ConfirmationRequested,
                WriteSessionEvent::IntentCreated {
                    intent_id: "intent-1".to_string(),
                },
                WriteSessionEvent::NonConsumingFinished {
                    intent_id: "intent-1".to_string(),
                    mode: SubmitMode::Test,
                },
            ],
        );

        let view = WriteSessionView::from(&session);
        assert_eq!(view.intent_status, None);
        assert_eq!(view.mode, SubmitMode::Test);
        assert!(!session.apply(WriteSessionEvent::LiveSubmitStarted {
            intent_id: "intent-1".to_string(),
        }));
    }

    #[test]
    fn only_live_mode_sessions_can_start_live_submission() {
        for mode in [SubmitMode::DryRun, SubmitMode::Test] {
            let mut session = WriteSession::from_request(request("session-1"), mode);
            apply_all(
                &mut session,
                [
                    WriteSessionEvent::ValidationStarted,
                    WriteSessionEvent::ValidationReady,
                    WriteSessionEvent::ConfirmationRequested,
                    WriteSessionEvent::IntentCreated {
                        intent_id: "intent-1".to_string(),
                    },
                ],
            );

            assert!(!session.apply(WriteSessionEvent::LiveSubmitStarted {
                intent_id: "intent-1".to_string(),
            }));
            assert_eq!(WriteSessionView::from(&session).mode, mode);
        }

        let mut live = WriteSession::from_request(request("session-1"), SubmitMode::Live);
        apply_all(
            &mut live,
            [
                WriteSessionEvent::ValidationStarted,
                WriteSessionEvent::ValidationReady,
                WriteSessionEvent::ConfirmationRequested,
                WriteSessionEvent::IntentCreated {
                    intent_id: "intent-1".to_string(),
                },
            ],
        );

        assert!(live.apply(WriteSessionEvent::LiveSubmitStarted {
            intent_id: "intent-1".to_string(),
        }));
    }

    #[test]
    fn live_preflight_failures_keep_core_intent_status_empty() {
        let mut session = WriteSession::from_request(request("session-1"), SubmitMode::Live);
        apply_all(
            &mut session,
            [
                WriteSessionEvent::ValidationStarted,
                WriteSessionEvent::ValidationReady,
                WriteSessionEvent::ConfirmationRequested,
                WriteSessionEvent::IntentCreated {
                    intent_id: "intent-1".to_string(),
                },
                WriteSessionEvent::PreflightFailed {
                    intent_id: "intent-1".to_string(),
                    attempted_mode: SubmitMode::Live,
                },
            ],
        );

        let view = WriteSessionView::from(&session);
        assert_eq!(view.stage, WriteSessionStage::LivePreflightFailed);
        assert_eq!(view.intent_status, None);
        assert_eq!(view.mode, SubmitMode::Live);
        assert!(session.apply(WriteSessionEvent::LiveSubmitStarted {
            intent_id: "intent-1".to_string(),
        }));
    }

    #[test]
    fn validation_failures_before_intent_do_not_claim_core_intent_status() {
        let mut session = WriteSession::from_request(request("session-1"), SubmitMode::DryRun);

        assert!(session.apply(WriteSessionEvent::ValidationStarted));
        assert!(session.apply(WriteSessionEvent::FailedBeforeIntent));

        let view = WriteSessionView::from(&session);
        assert_eq!(view.intent_id, None);
        assert_eq!(view.intent_status, None);
    }

    #[test]
    fn live_submission_lifecycle_is_the_only_core_submitted_status_source() {
        let mut session = WriteSession::from_request(request("session-1"), SubmitMode::Live);
        for event in [
            WriteSessionEvent::ValidationStarted,
            WriteSessionEvent::ValidationReady,
            WriteSessionEvent::ConfirmationRequested,
            WriteSessionEvent::IntentCreated {
                intent_id: "intent-1".to_string(),
            },
            WriteSessionEvent::LiveSubmitStarted {
                intent_id: "intent-1".to_string(),
            },
        ] {
            assert!(session.apply(event));
            assert_eq!(WriteSessionView::from(&session).intent_status, None);
        }

        assert!(session.apply(WriteSessionEvent::LiveSubmitSucceeded {
            intent_id: "intent-1".to_string(),
        }));
        assert_eq!(
            WriteSessionView::from(&session).intent_status,
            Some(IntentStatus::Submitted)
        );
    }

    #[test]
    fn write_session_events_allow_abandoning_before_intent_creation() {
        let mut session = WriteSession::from_request(request("session-1"), SubmitMode::DryRun);

        assert!(session.apply(WriteSessionEvent::ValidationStarted));
        assert!(session.apply(WriteSessionEvent::Abandoned));
        assert!(!session.apply(WriteSessionEvent::ValidationReady));
    }

    #[test]
    fn confirmation_can_return_to_ready_before_intent_creation() {
        let mut session = WriteSession::from_request(request("session-1"), SubmitMode::DryRun);
        apply_all(
            &mut session,
            [
                WriteSessionEvent::ValidationStarted,
                WriteSessionEvent::ValidationReady,
                WriteSessionEvent::ConfirmationRequested,
                WriteSessionEvent::ReturnedToReady,
            ],
        );

        assert_eq!(session.state(), &WriteSessionState::Ready);
        assert!(session.apply(WriteSessionEvent::ConfirmationRequested));
        assert!(session.apply(WriteSessionEvent::IntentCreated {
            intent_id: "intent-1".to_string(),
        }));
    }

    #[test]
    fn write_sessions_do_not_replace_active_sessions() {
        let mut sessions = WriteSessions::default();

        assert_eq!(
            sessions.open(request("session-1"), SubmitMode::DryRun),
            OpenSessionResult::Opened
        );
        assert_eq!(
            sessions.apply("session-1", WriteSessionEvent::ValidationStarted),
            TransitionResult::Applied
        );
        assert_eq!(
            sessions.open(request("session-1"), SubmitMode::Live),
            OpenSessionResult::Rejected
        );

        let view = sessions.views().pop().unwrap();
        assert_eq!(view.mode, SubmitMode::DryRun);
        assert_eq!(view.stage, WriteSessionStage::Validating);
    }

    #[test]
    fn draft_sessions_can_be_replaced_before_validation_starts() {
        let mut sessions = WriteSessions::default();

        assert_eq!(
            sessions.open(request("session-1"), SubmitMode::DryRun),
            OpenSessionResult::Opened
        );
        assert_eq!(
            sessions.open(request("session-1"), SubmitMode::Live),
            OpenSessionResult::Opened
        );

        let view = sessions.views().pop().unwrap();
        assert_eq!(view.mode, SubmitMode::Live);
        assert_eq!(view.stage, WriteSessionStage::Draft);
    }

    #[test]
    fn disabling_live_abandons_pending_live_sessions_but_keeps_submitting_sessions() {
        let mut sessions = WriteSessions::default();
        assert_eq!(
            sessions.open(request("pending"), SubmitMode::Live),
            OpenSessionResult::Opened
        );
        assert_eq!(
            sessions.open(request("submitting"), SubmitMode::Live),
            OpenSessionResult::Opened
        );
        apply_all(
            sessions
                .sessions
                .iter_mut()
                .find(|session| session.id == "submitting")
                .expect("submitting session"),
            [
                WriteSessionEvent::ValidationStarted,
                WriteSessionEvent::ValidationReady,
                WriteSessionEvent::ConfirmationRequested,
                WriteSessionEvent::IntentCreated {
                    intent_id: "intent-1".to_string(),
                },
                WriteSessionEvent::LiveSubmitStarted {
                    intent_id: "intent-1".to_string(),
                },
            ],
        );

        assert_eq!(sessions.disable_live(), 1);
        let views = sessions.views();
        let pending = views
            .iter()
            .find(|view| view.id == "pending")
            .expect("pending view");
        let submitting = views
            .iter()
            .find(|view| view.id == "submitting")
            .expect("submitting view");

        assert_eq!(pending.stage, WriteSessionStage::Abandoned);
        assert_eq!(pending.mode, SubmitMode::DryRun);
        assert_eq!(submitting.stage, WriteSessionStage::LiveSubmitting);
        assert_eq!(submitting.mode, SubmitMode::Live);
    }
}
