use crate::scheduler::Scheduler;
use crate::state::{Action, AppState};

#[derive(Debug, Clone)]
pub(crate) struct ProfileValidationLoadRuntime {
    next_generation: u64,
}

impl ProfileValidationLoadRuntime {
    pub(crate) const fn new() -> Self {
        Self { next_generation: 1 }
    }

    fn next_generation(&mut self) -> u64 {
        let generation = self.next_generation;
        self.next_generation = self.next_generation.saturating_add(1);
        generation
    }
}

pub(crate) fn request_profile_validation_load(
    scheduler: &Scheduler,
    state: &mut AppState,
    runtime: &mut ProfileValidationLoadRuntime,
) {
    let Some(request) = prepare_profile_validation_request(state, runtime) else {
        return;
    };

    if let Err(error) = scheduler.request_profile_validation(request.generation, request.profile) {
        state.reduce(Action::SchedulerFailed(error.to_string()));
    }
}

fn prepare_profile_validation_request(
    state: &mut AppState,
    runtime: &mut ProfileValidationLoadRuntime,
) -> Option<ProfileValidationRequest> {
    if state.profile_validation_loading()
        || state.scheduler_error.is_some()
        || state.has_current_profile_validation()
    {
        return None;
    }
    let profile = state.trading_profile.clone()?;
    let generation = runtime.next_generation();
    state.reduce(Action::ProfileValidationStarted {
        generation,
        profile: profile.clone(),
    });
    Some(ProfileValidationRequest {
        generation,
        profile,
    })
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ProfileValidationRequest {
    generation: u64,
    profile: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{TradingConfig, TuiConfig};
    use crate::profile_snapshot::ProfileValidationState;
    use crate::{command::ActionId, profile_snapshot::ProfileValidationSnapshot};
    use agent_finance_core::DiagnosticCheck;
    use std::path::PathBuf;

    #[test]
    fn profile_validation_load_treats_failure_as_terminal_until_profile_changes() {
        let mut state = AppState::from_config(TuiConfig {
            trading: TradingConfig {
                default_profile: Some("missing".to_string()),
            },
            ..TuiConfig::default()
        });
        let mut runtime = ProfileValidationLoadRuntime::new();

        let first = prepare_profile_validation_request(&mut state, &mut runtime)
            .expect("first validation request");
        assert_eq!(first.profile, "missing");
        state.reduce(Action::ProfileValidationFailed {
            generation: first.generation,
            profile: "missing".to_string(),
            error: "profile not found".to_string(),
        });

        let second = prepare_profile_validation_request(&mut state, &mut runtime);

        assert!(second.is_none());
        assert!(matches!(
            &state.profile_validation,
            ProfileValidationState::Failed { profile, .. } if profile == "missing"
        ));
    }

    #[test]
    fn profile_validation_load_requeues_after_explicit_revalidation() {
        let mut state = AppState::from_config(TuiConfig {
            trading: TradingConfig {
                default_profile: Some("mainnet".to_string()),
            },
            ..TuiConfig::default()
        });
        let mut runtime = ProfileValidationLoadRuntime::new();
        let first = prepare_profile_validation_request(&mut state, &mut runtime)
            .expect("first validation request");
        state.reduce(Action::ProfileValidationLoaded {
            generation: first.generation,
            snapshot: ProfileValidationSnapshot {
                profile: "mainnet".to_string(),
                path: PathBuf::from("/tmp/mainnet.toml"),
                checks: vec![DiagnosticCheck::new("env", true, true, "ok")],
            },
        });
        assert!(state.has_current_profile_validation());

        state.reduce(Action::Execute(ActionId::RevalidateTradingProfile));
        let second = prepare_profile_validation_request(&mut state, &mut runtime)
            .expect("revalidation request");

        assert_eq!(second.profile, "mainnet");
        assert!(second.generation > first.generation);
        assert!(matches!(
            &state.profile_validation,
            ProfileValidationState::Loading { profile } if profile == "mainnet"
        ));
    }
}
