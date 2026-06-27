use super::*;
use agent_finance_core::{
    intent::IntentStatus,
    submit::{SubmitIntentKind, SubmitMode},
};

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
            StagedChangeEvent::SubmitQueued,
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
            StagedChangeEvent::SubmitQueued,
            StagedChangeEvent::IntentCreated {
                intent_id: "intent-1".to_string(),
            },
        ],
    );

    assert!(!change.apply(StagedChangeEvent::LiveIntentClaimed {
        intent_id: "intent-2".to_string(),
    }));
    assert!(change.apply(StagedChangeEvent::LiveIntentClaimed {
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
            StagedChangeEvent::SubmitQueued,
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
    assert!(!change.apply(StagedChangeEvent::LiveIntentClaimed {
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
            StagedChangeEvent::SubmitQueued,
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
    assert!(!change.apply(StagedChangeEvent::LiveIntentClaimed {
        intent_id: "intent-1".to_string(),
    }));
}

#[test]
fn only_live_mode_changes_can_claim_live_intent() {
    for mode in [SubmitMode::DryRun, SubmitMode::Test] {
        let mut change = StagedChange::from_request(request("change-1"), mode);
        apply_all(
            &mut change,
            [
                StagedChangeEvent::ValidationStarted,
                StagedChangeEvent::ValidationReady,
                StagedChangeEvent::SubmitQueued,
                StagedChangeEvent::IntentCreated {
                    intent_id: "intent-1".to_string(),
                },
            ],
        );

        assert!(!change.apply(StagedChangeEvent::LiveIntentClaimed {
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
            StagedChangeEvent::SubmitQueued,
            StagedChangeEvent::IntentCreated {
                intent_id: "intent-1".to_string(),
            },
        ],
    );

    assert!(live.apply(StagedChangeEvent::LiveIntentClaimed {
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
            StagedChangeEvent::SubmitQueued,
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
    assert!(change.apply(StagedChangeEvent::LiveIntentClaimed {
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
        StagedChangeEvent::SubmitQueued,
        StagedChangeEvent::IntentCreated {
            intent_id: "intent-1".to_string(),
        },
        StagedChangeEvent::LiveIntentClaimed {
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
            StagedChangeEvent::SubmitQueued,
            StagedChangeEvent::ReturnedToReady,
        ],
    );

    assert_eq!(change.state(), &StagedChangeState::Ready);
    assert!(change.apply(StagedChangeEvent::SubmitQueued));
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
fn disabling_live_abandons_pending_live_changes_but_keeps_claimed_changes() {
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
        StagedChangeEvent::SubmitQueued,
        StagedChangeEvent::IntentCreated {
            intent_id: "intent-1".to_string(),
        },
        StagedChangeEvent::LiveIntentClaimed {
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
    assert_eq!(submitting.stage, StagedChangeStage::LiveIntentClaimed);
    assert_eq!(submitting.mode, SubmitMode::Live);
}
