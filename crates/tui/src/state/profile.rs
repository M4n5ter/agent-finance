use crate::model::FloatingKind;
use crate::state::AppState;
use crate::task_failure::TaskFailureSource;

impl AppState {
    pub(super) fn accept_trading_profile(&mut self) {
        let next = self.profile_editor.profile();
        if self.trading_profile == next {
            self.task_log.info("trading profile unchanged".to_string());
            self.close_floating(FloatingKind::TradingProfile);
            return;
        }

        self.trading_profile = next;
        self.trading_profile_edited = true;
        self.invalidate_account_snapshot_for_profile_change();
        self.mark_config_changed("trading");
        match self.trading_profile.as_deref() {
            Some(profile) => self
                .task_log
                .info(format!("trading profile set to {profile}")),
            None => self.task_log.info("trading profile cleared".to_string()),
        }
        self.close_floating(FloatingKind::TradingProfile);
    }

    fn invalidate_account_snapshot_for_profile_change(&mut self) {
        if let Some(active) = self.account.cancel() {
            self.task_log.warning_event(format!(
                "cancelled {} account snapshot loading after profile change",
                active.key
            ));
        }
        self.account_snapshot = None;
        self.selected_open_order = 0;
        self.task_failures.clear_source(TaskFailureSource::Account);
    }
}
