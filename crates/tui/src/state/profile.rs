use agent_finance_core::submit::SubmitMode;

use crate::model::{FloatingKind, Panel};
use crate::profile_snapshot::ProfileValidationState;
use crate::state::{
    AppState, LocalConfigEdit, OpenStagedChangeResult, ProfileRiskReview, StagedChangeRequest,
    StagedChangeSubject,
};
use crate::task_failure::TaskFailureSource;

impl AppState {
    pub(super) fn accept_trading_profile(&mut self) {
        let next = self.profile_editor.profile();
        if self.trading_profile == next {
            self.task_log.info("trading profile unchanged".to_string());
            self.close_floating(FloatingKind::TradingProfile);
            return;
        }

        self.edit_local_config(|state| {
            state.trading_profile = next;
            state.trading_profile_edited = true;
            state.invalidate_account_snapshot_for_profile_change();
            Some(LocalConfigEdit::new("trading", ()))
        });
        match self.trading_profile.as_deref() {
            Some(profile) => self
                .task_log
                .info(format!("trading profile set to {profile}")),
            None => self.task_log.info("trading profile cleared".to_string()),
        }
        self.close_floating(FloatingKind::TradingProfile);
    }

    pub(super) fn revalidate_trading_profile(&mut self) {
        let Some(profile) = self.trading_profile.as_deref() else {
            self.task_log
                .warning_event("no trading profile selected for validation".to_string());
            return;
        };

        if self.profile_validation_request.loading() {
            self.task_log
                .warning_event(format!("{profile} profile validation is already loading"));
            return;
        }

        self.profile_validation = ProfileValidationState::idle();
        self.task_log
            .info(format!("{profile} profile validation queued"));
    }

    pub(super) fn stage_profile_live_toggle(&mut self) {
        let Some(selected_profile) = self.trading_profile.as_deref() else {
            self.task_log
                .warning_event("no trading profile selected for profile risk change".to_string());
            return;
        };
        let ProfileValidationState::Ready {
            profile,
            path,
            profile_config,
            source_content_hash,
            ..
        } = &self.profile_validation
        else {
            self.task_log.warning_event(format!(
                "{selected_profile} profile must be validated before staging a risk change"
            ));
            return;
        };
        if profile != selected_profile {
            self.task_log.warning_event(format!(
                "{selected_profile} profile validation is not current"
            ));
            return;
        }

        let review = ProfileRiskReview::allow_live_toggle(
            profile,
            path.clone(),
            source_content_hash.clone(),
            profile_config,
        );
        let request = StagedChangeRequest {
            id: profile_risk_staged_change_id(profile, "allow-live", review.target_value()),
            subject: StagedChangeSubject::ProfileRisk(review),
        };
        let change_id = request.id.clone();
        self.focus_panel(Panel::IntentReview);
        match self.staged_changes.open_ready(request, SubmitMode::DryRun) {
            OpenStagedChangeResult::Opened => {
                self.task_log
                    .info(format!("staged profile risk change {change_id}"));
            }
            OpenStagedChangeResult::Rejected => {
                self.task_log.warning_event(
                    "profile risk change cannot replace an active staged change".to_string(),
                );
            }
        }
    }

    pub(super) fn invalidate_account_snapshot_for_profile_change(&mut self) {
        if let Some(active) = self.account.cancel() {
            self.task_log.warning_event(format!(
                "cancelled {} account snapshot loading after profile change",
                active.key
            ));
        }
        if let Some(active) = self.profile_validation_request.cancel() {
            self.task_log.warning_event(format!(
                "cancelled {} profile validation after profile change",
                active.key
            ));
        }
        self.account_snapshot = None;
        self.profile_validation = ProfileValidationState::idle();
        self.selected_open_order = 0;
        self.task_failures.clear_source(TaskFailureSource::Account);
    }
}

fn profile_risk_staged_change_id(profile: &str, field: &str, value: bool) -> String {
    format!("profile-risk-{profile}-{field}-{value}")
}
