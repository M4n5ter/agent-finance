use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use agent_finance_core::{
    CancelIntent, FuturesStateIntent, OrderIntent, Profile, SubmitMode, TransferIntent,
};
use agent_finance_trading::TradingRuntime;
use anyhow::Result;
use chrono::Utc;

use crate::config::TuiLaunch;
use crate::state::{
    CancelReview, FuturesStateReview, StagedChangeEvent, StagedChangeSubject, StagedSubmitRequest,
    TransferReview,
};

use super::{SchedulerEvent, scheduler_runtime};

#[derive(Debug)]
pub(super) enum WriteCommand {
    SubmitStaged(StagedSubmitRequest),
}

pub(super) fn spawn_write_worker(
    launch: &TuiLaunch,
    commands: Receiver<WriteCommand>,
    events: Sender<SchedulerEvent>,
) {
    let runtime = TradingRuntime::with_http_policy(
        launch.timeout_seconds,
        launch.proxy.clone(),
        launch.no_proxy,
    );
    thread::Builder::new()
        .name("agent-finance-tui-write".to_string())
        .spawn(move || {
            let Some(tokio) = scheduler_runtime("write", &events) else {
                return;
            };

            while let Ok(command) = commands.recv() {
                if !handle_write_command(&tokio, &runtime, command, &events) {
                    break;
                }
            }
        })
        .unwrap_or_else(|error| panic!("failed to spawn TUI write scheduler thread: {error}"));
}

fn handle_write_command(
    tokio: &tokio::runtime::Runtime,
    runtime: &TradingRuntime,
    command: WriteCommand,
    events: &Sender<SchedulerEvent>,
) -> bool {
    match command {
        WriteCommand::SubmitStaged(request) => {
            handle_staged_submit(tokio, runtime, request, events)
        }
    }
}

fn handle_staged_submit(
    tokio: &tokio::runtime::Runtime,
    runtime: &TradingRuntime,
    request: StagedSubmitRequest,
    events: &Sender<SchedulerEvent>,
) -> bool {
    let created = create_staged_intent(runtime, &request);
    let (profile, intent_id) = match created {
        Ok((profile, intent_id)) => {
            if !send_staged_change_progress(
                events,
                &request.id,
                StagedChangeEvent::IntentCreated {
                    intent_id: intent_id.clone(),
                },
                Some(format!(
                    "created {} intent {intent_id}",
                    request.subject.kind_label()
                )),
            ) {
                return false;
            }
            (profile, intent_id)
        }
        Err(error) => {
            return send_staged_change_progress(
                events,
                &request.id,
                StagedChangeEvent::FailedBeforeIntent,
                Some(format!("{error:#}")),
            );
        }
    };

    let result = tokio.block_on(submit_staged_intent(
        runtime,
        &profile,
        &intent_id,
        request.mode,
        &request.subject,
    ));
    match (request.mode, result) {
        (SubmitMode::DryRun | SubmitMode::Test, Ok(_)) => send_staged_change_progress(
            events,
            &request.id,
            StagedChangeEvent::NonConsumingFinished {
                intent_id,
                mode: request.mode,
            },
            Some(format!("{} submit completed", request.mode)),
        ),
        (SubmitMode::DryRun | SubmitMode::Test, Err(error)) => send_staged_change_progress(
            events,
            &request.id,
            StagedChangeEvent::PreflightFailed {
                intent_id,
                attempted_mode: request.mode,
            },
            Some(error.to_string()),
        ),
        (SubmitMode::Live, Ok(_)) => {
            send_staged_change_progress(
                events,
                &request.id,
                StagedChangeEvent::LiveIntentClaimed {
                    intent_id: intent_id.clone(),
                },
                None,
            ) && send_staged_change_progress(
                events,
                &request.id,
                StagedChangeEvent::LiveSubmitSucceeded { intent_id },
                Some("live submit completed".to_string()),
            )
        }
        (SubmitMode::Live, Err(error)) if error.exchange_was_accepted() => {
            send_staged_change_progress(
                events,
                &request.id,
                StagedChangeEvent::LiveIntentClaimed {
                    intent_id: intent_id.clone(),
                },
                None,
            ) && send_staged_change_progress(
                events,
                &request.id,
                StagedChangeEvent::LiveSubmitSucceeded { intent_id },
                Some(format!(
                    "exchange accepted the live submit, but local finalization failed: {error}"
                )),
            )
        }
        (SubmitMode::Live, Err(error)) if error.exchange_was_attempted() => {
            send_staged_change_progress(
                events,
                &request.id,
                StagedChangeEvent::LiveIntentClaimed {
                    intent_id: intent_id.clone(),
                },
                None,
            ) && send_staged_change_progress(
                events,
                &request.id,
                StagedChangeEvent::LiveSubmitFailed { intent_id },
                Some(error.to_string()),
            )
        }
        (SubmitMode::Live, Err(error)) => send_staged_change_progress(
            events,
            &request.id,
            StagedChangeEvent::PreflightFailed {
                intent_id,
                attempted_mode: SubmitMode::Live,
            },
            Some(error.to_string()),
        ),
    }
}

fn send_staged_change_progress(
    events: &Sender<SchedulerEvent>,
    id: &str,
    event: StagedChangeEvent,
    message: Option<String>,
) -> bool {
    events
        .send(SchedulerEvent::StagedChangeProgress {
            id: id.to_string(),
            event,
            message,
        })
        .is_ok()
}

fn create_staged_order_intent(
    runtime: &TradingRuntime,
    review: &crate::state::OrderTicketReview,
    mode: SubmitMode,
) -> Result<(Profile, String)> {
    let profile = runtime.load_profile(&review.profile)?;
    let intent = order_intent_from_review(&profile, review)?;
    let risk =
        runtime.check_order_with_runtime_limits(&profile, &intent, mode == SubmitMode::Live)?;
    let envelope = agent_finance_core::create_order_intent(intent, 300)?;
    runtime.save_intent_with_audit(
        &profile,
        &envelope,
        &risk,
        format!("created TUI order intent {}", envelope.id),
    )?;
    Ok((profile, envelope.id))
}

fn create_staged_intent(
    runtime: &TradingRuntime,
    request: &StagedSubmitRequest,
) -> Result<(Profile, String)> {
    match &request.subject {
        StagedChangeSubject::OrderTicket(review) => {
            create_staged_order_intent(runtime, review, request.mode)
        }
        StagedChangeSubject::Cancel(review) => {
            create_staged_cancel_intent(runtime, review, request.mode)
        }
        StagedChangeSubject::Transfer(review) => {
            create_staged_transfer_intent(runtime, review, request.mode)
        }
        StagedChangeSubject::FuturesState(review) => {
            create_staged_futures_state_intent(runtime, review, request.mode)
        }
        #[cfg(test)]
        StagedChangeSubject::Text { .. } => unreachable!("text changes are never submitted"),
    }
}

fn create_staged_cancel_intent(
    runtime: &TradingRuntime,
    review: &CancelReview,
    mode: SubmitMode,
) -> Result<(Profile, String)> {
    let profile = runtime.load_profile(&review.profile)?;
    let intent = cancel_intent_from_review(&profile, review)?;
    let risk = agent_finance_core::check_cancel_intent(&profile, &intent, mode == SubmitMode::Live);
    let envelope = agent_finance_core::create_cancel_intent(intent, 300)?;
    runtime.save_intent_with_audit(
        &profile,
        &envelope,
        &risk,
        format!("created TUI cancel intent {}", envelope.id),
    )?;
    Ok((profile, envelope.id))
}

fn create_staged_transfer_intent(
    runtime: &TradingRuntime,
    review: &TransferReview,
    mode: SubmitMode,
) -> Result<(Profile, String)> {
    let profile = runtime.load_profile(&review.profile)?;
    let intent = transfer_intent_from_review(&profile, review)?;
    let risk =
        agent_finance_core::check_transfer_intent(&profile, &intent, mode == SubmitMode::Live);
    let envelope = agent_finance_core::create_transfer_intent(intent, 300)?;
    runtime.save_intent_with_audit(
        &profile,
        &envelope,
        &risk,
        format!("created TUI transfer intent {}", envelope.id),
    )?;
    Ok((profile, envelope.id))
}

fn create_staged_futures_state_intent(
    runtime: &TradingRuntime,
    review: &FuturesStateReview,
    mode: SubmitMode,
) -> Result<(Profile, String)> {
    let profile = runtime.load_profile(&review.profile)?;
    let intent = futures_state_intent_from_review(&profile, review)?;
    let risk =
        agent_finance_core::check_futures_state_intent(&profile, &intent, mode == SubmitMode::Live);
    let envelope = agent_finance_core::create_futures_state_intent(intent, 300)?;
    runtime.save_intent_with_audit(
        &profile,
        &envelope,
        &risk,
        format!("created TUI futures state intent {}", envelope.id),
    )?;
    Ok((profile, envelope.id))
}

fn order_intent_from_review(
    profile: &Profile,
    review: &crate::state::OrderTicketReview,
) -> Result<OrderIntent> {
    Ok(OrderIntent {
        profile: profile.name.clone(),
        provider: profile.provider.provider,
        environment: profile.provider.environment,
        market: review.market,
        symbol: review.symbol.to_ascii_uppercase(),
        side: review.side,
        quantity: review.parsed_quantity.clone(),
        spec: review.order_spec.clone(),
        reduce_only: review.reduce_only,
        position_side: None,
        client_order_id: format!("af-tui-{}", Utc::now().timestamp_millis()),
    })
}

fn transfer_intent_from_review(
    profile: &Profile,
    review: &TransferReview,
) -> Result<TransferIntent> {
    Ok(TransferIntent {
        profile: profile.name.clone(),
        provider: profile.provider.provider,
        environment: profile.provider.environment,
        direction: review.direction,
        asset: review.asset.to_ascii_uppercase(),
        amount: review.parsed_amount.clone(),
        client_transfer_id: format!("af-tui-transfer-{}", Utc::now().timestamp_millis()),
    })
}

fn futures_state_intent_from_review(
    profile: &Profile,
    review: &FuturesStateReview,
) -> Result<FuturesStateIntent> {
    Ok(FuturesStateIntent {
        profile: profile.name.clone(),
        provider: profile.provider.provider,
        environment: profile.provider.environment,
        change: review.change.clone(),
    })
}

fn cancel_intent_from_review(profile: &Profile, review: &CancelReview) -> Result<CancelIntent> {
    Ok(CancelIntent {
        profile: profile.name.clone(),
        provider: profile.provider.provider,
        environment: profile.provider.environment,
        market: review.market,
        symbol: review.symbol.to_ascii_uppercase(),
        target: review.target(),
    })
}

async fn submit_staged_intent(
    runtime: &TradingRuntime,
    profile: &Profile,
    intent_id: &str,
    mode: SubmitMode,
    subject: &StagedChangeSubject,
) -> std::result::Result<agent_finance_core::SubmitSnapshot, agent_finance_trading::SubmitFailure> {
    runtime
        .submit_typed_intent_classified(profile, intent_id, subject.intent_kind(), mode)
        .await
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::str::FromStr;

    use agent_finance_core::{
        DecimalValue, Environment, FuturesStateChange, ProfilePermissions, Provider,
        ProviderConfig, RiskPolicy, SubmitMode, TransferDirection, TransferPolicy,
    };

    use super::*;

    #[test]
    fn transfer_intent_from_review_preserves_profile_and_normalizes_asset() {
        let profile = Profile {
            name: "mainnet".to_string(),
            provider: ProviderConfig {
                provider: Provider::Binance,
                environment: Environment::Live,
                api_key_env: "BINANCE_API_KEY".to_string(),
                api_secret_env: "BINANCE_PRIVATE_KEY".to_string(),
                spot_base_url: None,
                usds_futures_base_url: None,
                sapi_base_url: None,
            },
            permissions: ProfilePermissions {
                spot_trading: false,
                usds_futures: true,
                universal_transfer: true,
            },
            risk: RiskPolicy {
                allow_live: false,
                max_daily_order_notional_usdt: None,
                allowed_symbols: BTreeMap::new(),
                allowed_transfers: vec![TransferPolicy {
                    direction: TransferDirection::SpotToUsdsFutures,
                    asset: "USDT".to_string(),
                    max_amount: DecimalValue::from_str("10").unwrap(),
                }],
                allowed_futures_state_changes: Vec::new(),
            },
        };
        let review = TransferReview {
            profile: "mainnet".to_string(),
            direction: TransferDirection::SpotToUsdsFutures,
            asset: "usdt".to_string(),
            amount: "5".to_string(),
            parsed_amount: DecimalValue::from_str("5").unwrap(),
            effective_mode: SubmitMode::DryRun,
        };

        let intent = transfer_intent_from_review(&profile, &review).unwrap();

        assert_eq!(intent.profile, "mainnet");
        assert_eq!(intent.provider, Provider::Binance);
        assert_eq!(intent.environment, Environment::Live);
        assert_eq!(intent.direction, TransferDirection::SpotToUsdsFutures);
        assert_eq!(intent.asset, "USDT");
        assert_eq!(intent.amount, DecimalValue::from_str("5").unwrap());
        assert!(intent.client_transfer_id.starts_with("af-tui-transfer-"));
    }

    #[test]
    fn futures_state_intent_from_review_preserves_profile_and_change() {
        let profile = Profile {
            name: "mainnet".to_string(),
            provider: ProviderConfig {
                provider: Provider::Binance,
                environment: Environment::Live,
                api_key_env: "BINANCE_API_KEY".to_string(),
                api_secret_env: "BINANCE_PRIVATE_KEY".to_string(),
                spot_base_url: None,
                usds_futures_base_url: None,
                sapi_base_url: None,
            },
            permissions: ProfilePermissions {
                spot_trading: false,
                usds_futures: true,
                universal_transfer: false,
            },
            risk: RiskPolicy {
                allow_live: false,
                max_daily_order_notional_usdt: None,
                allowed_symbols: BTreeMap::new(),
                allowed_transfers: Vec::new(),
                allowed_futures_state_changes: Vec::new(),
            },
        };
        let review = FuturesStateReview {
            profile: "mainnet".to_string(),
            change: FuturesStateChange::Leverage {
                symbol: "ETHUSDT".to_string(),
                leverage: 2,
            },
            effective_mode: SubmitMode::DryRun,
        };

        let intent = futures_state_intent_from_review(&profile, &review).unwrap();

        assert_eq!(intent.profile, "mainnet");
        assert_eq!(intent.provider, Provider::Binance);
        assert_eq!(intent.environment, Environment::Live);
        assert_eq!(intent.change, review.change);
    }
}
