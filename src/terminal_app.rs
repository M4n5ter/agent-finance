use anyhow::{Result, anyhow};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;

use crate::cli::{
    AccountArgs, AccountCommand, AuditArgs, AuditCommand, CapabilitiesArgs, OrderArgs,
    OrderCommand, ProfileArgs, ProfileCommand, RiskArgs, RiskCommand, TransferArgs,
    TransferCommand,
};

pub(crate) fn run_capabilities(args: CapabilitiesArgs) -> Result<()> {
    let report = agent_finance_core::CapabilityReport::new(vec![
        agent_finance_binance::provider_capability(),
    ]);
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("command model: {}", report.command_model);
        for provider in report.providers {
            println!("\nprovider: {}", provider.provider);
            for capability in provider.capabilities {
                println!(
                    "- {} [{}] markets={}",
                    capability.name,
                    capability.access,
                    capability.markets.join(",")
                );
                for note in capability.notes {
                    println!("  {note}");
                }
            }
        }
        println!("\nsafety:");
        for item in report.safety_model {
            println!("- {item}");
        }
    }
    Ok(())
}

pub(crate) async fn run_profile(args: ProfileArgs, timeout_seconds: u64) -> Result<()> {
    let store = agent_finance_core::ProfileStore::from_default_dir()?;
    match args.command {
        ProfileCommand::Path(args) => {
            let path = store.path(&args.profile);
            print_json_or_text(
                args.json,
                &json!({ "profile": args.profile, "path": path }),
                || path.display().to_string(),
            )
        }
        ProfileCommand::Template(args) => {
            let content = agent_finance_binance::profile_template(&args.profile);
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "profile": args.profile,
                        "content": content,
                    }))?
                );
            } else {
                print!("{content}");
            }
            Ok(())
        }
        ProfileCommand::Explain(args) => {
            let profile = store.load(&args.profile)?;
            print_json_or_text(args.json, &profile, || explain_profile(&profile))
        }
        ProfileCommand::Doctor(args) => {
            let profile = store.load(&args.profile)?;
            let mut checks = vec![json!({
                "name": "profile-parse",
                "ok": true,
                "message": "profile TOML parsed successfully",
            })];
            let key_ok = std::env::var(&profile.provider.api_key_env).is_ok();
            let secret_ok = std::env::var(&profile.provider.api_secret_env).is_ok();
            checks.push(json!({
                "name": "api-key-env",
                "ok": key_ok,
                "message": format!("{} {}", profile.provider.api_key_env, if key_ok { "is set" } else { "is missing" }),
            }));
            checks.push(json!({
                "name": "api-secret-env",
                "ok": secret_ok,
                "message": format!("{} {}", profile.provider.api_secret_env, if secret_ok { "is set" } else { "is missing" }),
            }));
            if key_ok && secret_ok {
                match binance_client(&profile, timeout_seconds)?
                    .account_permissions()
                    .await
                {
                    Ok(payload) => checks.push(json!({
                        "name": "binance-permissions",
                        "ok": true,
                        "message": "Binance API key permission endpoint succeeded",
                        "payload": payload,
                    })),
                    Err(error) => checks.push(json!({
                        "name": "binance-permissions",
                        "ok": false,
                        "message": format!("{error:#}"),
                    })),
                }
            }
            let report = json!({
                "profile": args.profile,
                "checks": checks,
            });
            print_json_or_text(args.json, &report, || {
                report["checks"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|check| {
                        format!(
                            "{}: {} - {}",
                            if check["ok"].as_bool().unwrap_or(false) {
                                "ok"
                            } else {
                                "fail"
                            },
                            check["name"].as_str().unwrap_or("unknown"),
                            check["message"].as_str().unwrap_or("")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        }
    }
}

pub(crate) async fn run_account(args: AccountArgs, timeout_seconds: u64) -> Result<()> {
    match args.command {
        AccountCommand::Permissions(args) => {
            let profile = load_profile(&args.profile)?;
            let payload = binance_client(&profile, timeout_seconds)?
                .account_permissions()
                .await?;
            print_json_or_text(args.json, &payload, || {
                serde_json::to_string_pretty(&payload).unwrap()
            })
        }
        AccountCommand::Balances(args) => {
            let profile = load_profile(&args.profile)?;
            let payload = binance_client(&profile, timeout_seconds)?
                .spot_account()
                .await?;
            print_json_or_text(args.json, &payload, || {
                serde_json::to_string_pretty(&payload).unwrap()
            })
        }
        AccountCommand::Positions(args) => {
            let profile = load_profile(&args.profile)?;
            let payload = binance_client(&profile, timeout_seconds)?
                .futures_account()
                .await?;
            print_json_or_text(args.json, &payload, || {
                serde_json::to_string_pretty(&payload).unwrap()
            })
        }
    }
}

pub(crate) async fn run_order(args: OrderArgs, timeout_seconds: u64) -> Result<()> {
    match args.command {
        OrderCommand::Intent(args) => {
            let profile = load_profile(&args.profile)?;
            let market = args.market.into();
            let intent = agent_finance_core::OrderIntent {
                profile: profile.name.clone(),
                provider: profile.provider.provider,
                environment: profile.provider.environment,
                market,
                symbol: args.symbol.to_ascii_uppercase(),
                side: args.side.into(),
                quantity: args.quantity.parse()?,
                spec: agent_finance_core::OrderSpec::new(
                    market,
                    args.kind.into(),
                    parse_optional_decimal(args.price)?,
                    parse_optional_decimal(args.valuation_price)?,
                    parse_optional_decimal(args.stop_price)?,
                    args.time_in_force.map(Into::into),
                )?,
                reduce_only: args.reduce_only,
                position_side: args.position_side.map(Into::into),
                client_order_id: format!("af-{}", Utc::now().timestamp_millis()),
            };
            let risk = agent_finance_core::check_order_intent(&profile, &intent, false);
            let envelope = agent_finance_core::create_order_intent(intent, args.ttl_seconds)?;
            let path = save_intent_with_audit(
                &profile,
                &envelope,
                &risk,
                format!("created order intent {}", envelope.id),
            )?;
            print_json_or_text(
                args.json,
                &json!({ "intent": envelope, "risk": risk, "path": path }),
                || {
                    format!(
                        "created order intent {}\nrisk allowed: {}\npath: {}",
                        envelope.id,
                        risk.allowed,
                        path.display()
                    )
                },
            )
        }
        OrderCommand::CancelIntent(args) => {
            let profile = load_profile(&args.profile)?;
            let intent = agent_finance_core::CancelIntent {
                profile: profile.name.clone(),
                provider: profile.provider.provider,
                environment: profile.provider.environment,
                market: args.market.into(),
                symbol: args.symbol.to_ascii_uppercase(),
                target: agent_finance_core::CancelTarget::new(args.order_id, args.client_order_id)?,
            };
            let risk = agent_finance_core::check_cancel_intent(&profile, &intent, false);
            let envelope = agent_finance_core::create_cancel_intent(intent, args.ttl_seconds)?;
            let path = save_intent_with_audit(
                &profile,
                &envelope,
                &risk,
                format!("created cancel intent {}", envelope.id),
            )?;
            print_json_or_text(
                args.json,
                &json!({ "intent": envelope, "risk": risk, "path": path }),
                || {
                    format!(
                        "created cancel intent {}\nrisk allowed: {}\npath: {}",
                        envelope.id,
                        risk.allowed,
                        path.display()
                    )
                },
            )
        }
        OrderCommand::Submit(args) => {
            let profile = load_profile(&args.profile)?;
            let mode = WriteMode::from_flags(args.live, args.test)?;
            let report = submit_intent(
                &profile,
                &args.intent_id,
                ExpectedIntentKind::OrderCommand,
                mode,
                timeout_seconds,
            )
            .await?;
            print_submit_report(args.json, &report)
        }
        OrderCommand::Open(args) => {
            let profile = load_profile(&args.profile)?;
            let response = binance_client(&profile, timeout_seconds)?
                .open_orders(args.market.into(), args.symbol.as_deref())
                .await?;
            print_json_or_text(args.json, &response, || {
                serde_json::to_string_pretty(&response).unwrap()
            })
        }
    }
}

pub(crate) async fn run_transfer(args: TransferArgs, timeout_seconds: u64) -> Result<()> {
    match args.command {
        TransferCommand::Intent(args) => {
            let profile = load_profile(&args.profile)?;
            let intent = agent_finance_core::TransferIntent {
                profile: profile.name.clone(),
                provider: profile.provider.provider,
                environment: profile.provider.environment,
                direction: args.direction.into(),
                asset: args.asset.to_ascii_uppercase(),
                amount: args.amount.parse()?,
                client_transfer_id: format!("af-{}", Utc::now().timestamp_millis()),
            };
            let risk = agent_finance_core::check_transfer_intent(&profile, &intent, false);
            let envelope = agent_finance_core::create_transfer_intent(intent, args.ttl_seconds)?;
            let path = save_intent_with_audit(
                &profile,
                &envelope,
                &risk,
                format!("created transfer intent {}", envelope.id),
            )?;
            print_json_or_text(
                args.json,
                &json!({ "intent": envelope, "risk": risk, "path": path }),
                || {
                    format!(
                        "created transfer intent {}\nrisk allowed: {}\npath: {}",
                        envelope.id,
                        risk.allowed,
                        path.display()
                    )
                },
            )
        }
        TransferCommand::Submit(args) => {
            let profile = load_profile(&args.profile)?;
            let mode = WriteMode::from_flags(args.live, false)?;
            let report = submit_intent(
                &profile,
                &args.intent_id,
                ExpectedIntentKind::TransferCommand,
                mode,
                timeout_seconds,
            )
            .await?;
            print_submit_report(args.json, &report)
        }
        TransferCommand::History(args) => {
            let profile = load_profile(&args.profile)?;
            ensure_live_sapi_profile(&profile, "transfer history")?;
            let response = binance_client(&profile, timeout_seconds)?
                .transfer_history(args.direction.into(), args.current, args.size)
                .await?;
            print_json_or_text(args.json, &response, || {
                serde_json::to_string_pretty(&response).unwrap()
            })
        }
    }
}

pub(crate) fn run_risk(args: RiskArgs) -> Result<()> {
    match args.command {
        RiskCommand::Check(args) => {
            let profile = load_profile(&args.profile)?;
            let envelope =
                agent_finance_core::IntentStore::from_default_dir()?.load(&args.intent_id)?;
            let risk = match &envelope.kind {
                agent_finance_core::IntentKind::Order(intent) => {
                    check_order_with_runtime_limits(&profile, intent, args.live)?
                }
                agent_finance_core::IntentKind::Cancel(intent) => {
                    agent_finance_core::check_cancel_intent(&profile, intent, args.live)
                }
                agent_finance_core::IntentKind::Transfer(intent) => {
                    agent_finance_core::check_transfer_intent(&profile, intent, args.live)
                }
            };
            print_json_or_text(args.json, &risk, || {
                if risk.findings.is_empty() {
                    format!("allowed: {}", risk.allowed)
                } else {
                    let findings = risk
                        .findings
                        .iter()
                        .map(|finding| format!("- {}: {}", finding.code, finding.message))
                        .collect::<Vec<_>>()
                        .join("\n");
                    format!("allowed: {}\n{findings}", risk.allowed)
                }
            })
        }
        RiskCommand::Explain(args) => {
            let profile = load_profile(&args.profile)?;
            let used = agent_finance_core::daily_live_order_notional_used_today(&profile)?;
            let report = json!({
                "profile": profile.name,
                "provider": profile.provider.provider,
                "environment": profile.provider.environment,
                "allow_live": profile.risk.allow_live,
                "max_daily_order_notional_usdt": profile.risk.max_daily_order_notional_usdt,
                "daily_order_notional_used_utc": used.to_string(),
                "allowed_symbols": profile.risk.allowed_symbols,
                "allowed_transfers": profile.risk.allowed_transfers,
            });
            print_json_or_text(args.json, &report, || {
                serde_json::to_string_pretty(&report).unwrap()
            })
        }
    }
}

pub(crate) fn run_audit(args: AuditArgs) -> Result<()> {
    match args.command {
        AuditCommand::Tail(args) => {
            let events = agent_finance_core::read_audit_events(args.limit)?;
            print_json_or_text(args.json, &events, || {
                events
                    .iter()
                    .map(|event| {
                        format!(
                            "{} {:?} {}",
                            event.timestamp_utc.to_rfc3339(),
                            event.kind,
                            event.summary
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        }
        AuditCommand::Export(args) => {
            let events = agent_finance_core::read_all_audit_events()?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&events)?);
            } else {
                for event in events {
                    println!("{}", serde_json::to_string(&event)?);
                }
            }
            Ok(())
        }
    }
}

fn load_profile(name: &str) -> Result<agent_finance_core::Profile> {
    agent_finance_core::ProfileStore::from_default_dir()?.load(name)
}

fn binance_client(
    profile: &agent_finance_core::Profile,
    timeout_seconds: u64,
) -> Result<agent_finance_binance::BinanceClient> {
    let credentials = agent_finance_binance::BinanceCredentials::from_env(
        &profile.provider.api_key_env,
        &profile.provider.api_secret_env,
    )?;
    agent_finance_binance::BinanceClient::new(
        credentials,
        binance_endpoints(profile),
        timeout_seconds,
    )
}

fn binance_endpoints(
    profile: &agent_finance_core::Profile,
) -> agent_finance_binance::BinanceEndpoints {
    agent_finance_binance::BinanceEndpoints::new(
        profile.provider.environment,
        profile.provider.spot_base_url.clone(),
        profile.provider.usds_futures_base_url.clone(),
        profile.provider.sapi_base_url.clone(),
    )
}

fn ensure_live_sapi_profile(profile: &agent_finance_core::Profile, operation: &str) -> Result<()> {
    if profile.provider.environment == agent_finance_core::Environment::Live {
        return Ok(());
    }
    Err(anyhow!(
        "{operation} uses Binance SAPI live account data; use a live profile after reviewing the request"
    ))
}

fn parse_optional_decimal(
    value: Option<String>,
) -> Result<Option<agent_finance_core::DecimalValue>> {
    value.map(|value| value.parse()).transpose()
}

#[derive(Debug, Clone, Copy)]
enum WriteMode {
    DryRun,
    Test,
    Live,
}

#[derive(Debug, Clone, Copy)]
enum ExpectedIntentKind {
    OrderCommand,
    TransferCommand,
}

impl ExpectedIntentKind {
    fn validate(self, intent: &agent_finance_core::IntentKind) -> Result<()> {
        match (self, intent) {
            (
                Self::OrderCommand,
                agent_finance_core::IntentKind::Order(_)
                | agent_finance_core::IntentKind::Cancel(_),
            )
            | (Self::TransferCommand, agent_finance_core::IntentKind::Transfer(_)) => Ok(()),
            (Self::OrderCommand, agent_finance_core::IntentKind::Transfer(_)) => {
                Err(anyhow!("order submit cannot submit a transfer intent"))
            }
            (
                Self::TransferCommand,
                agent_finance_core::IntentKind::Order(_)
                | agent_finance_core::IntentKind::Cancel(_),
            ) => Err(anyhow!("transfer submit can only submit a transfer intent")),
        }
    }
}

impl WriteMode {
    fn from_flags(live: bool, test: bool) -> Result<Self> {
        match (live, test) {
            (true, true) => Err(anyhow!("--live and --test are mutually exclusive")),
            (true, false) => Ok(Self::Live),
            (false, true) => Ok(Self::Test),
            (false, false) => Ok(Self::DryRun),
        }
    }

    fn is_live(self) -> bool {
        matches!(self, Self::Live)
    }

    fn consumes_intent(self) -> bool {
        matches!(self, Self::Live)
    }

    fn audit_kind(
        self,
        intent: &agent_finance_core::IntentKind,
    ) -> agent_finance_core::AuditEventKind {
        match (self, intent) {
            (Self::DryRun, _) => agent_finance_core::AuditEventKind::DryRun,
            (Self::Test, _) => agent_finance_core::AuditEventKind::TestSubmit,
            (Self::Live, agent_finance_core::IntentKind::Cancel(_)) => {
                agent_finance_core::AuditEventKind::Cancel
            }
            (Self::Live, agent_finance_core::IntentKind::Transfer(_)) => {
                agent_finance_core::AuditEventKind::Transfer
            }
            (Self::Live, agent_finance_core::IntentKind::Order(_)) => {
                agent_finance_core::AuditEventKind::LiveSubmit
            }
        }
    }

    fn binance_mode(self) -> Option<agent_finance_binance::BinanceRequestMode> {
        match self {
            Self::DryRun => None,
            Self::Test => Some(agent_finance_binance::BinanceRequestMode::Test),
            Self::Live => Some(agent_finance_binance::BinanceRequestMode::Live),
        }
    }
}

#[derive(Serialize)]
struct SubmitReport {
    intent_id: String,
    mode: String,
    risk: agent_finance_core::RiskDecision,
    response: serde_json::Value,
}

async fn submit_intent(
    profile: &agent_finance_core::Profile,
    intent_id: &str,
    expected_kind: ExpectedIntentKind,
    mode: WriteMode,
    timeout_seconds: u64,
) -> Result<SubmitReport> {
    let store = agent_finance_core::IntentStore::from_default_dir()?;
    let envelope = store.load_for_submit(intent_id)?;
    expected_kind.validate(&envelope.kind)?;
    if matches!(mode, WriteMode::Test)
        && !matches!(envelope.kind, agent_finance_core::IntentKind::Order(_))
    {
        return Err(anyhow!(
            "--test is only supported for order intents with Binance test endpoints"
        ));
    }
    let risk = check_intent(profile, &envelope.kind, mode.is_live())?;
    if !risk.allowed {
        let error = anyhow!("risk policy blocked intent submit");
        return Err(error);
    }
    if !mode.consumes_intent() {
        let response = execute_intent(profile, &envelope.kind, mode, timeout_seconds).await?;
        append_audit(
            profile,
            Some(envelope.id.clone()),
            mode.audit_kind(&envelope.kind),
            format!("planned intent {}", envelope.id),
            json!({ "risk": risk, "response": response }),
        )?;
        return Ok(SubmitReport {
            intent_id: envelope.id,
            mode: format!("{mode:?}"),
            risk,
            response,
        });
    }
    let envelope = store.claim_for_submit(&envelope.id)?;
    expected_kind.validate(&envelope.kind)?;
    let _audit_lock = live_order_audit_lock(profile, &envelope.kind, mode)?;
    let risk = check_intent(profile, &envelope.kind, mode.is_live())?;
    if !risk.allowed {
        let error = anyhow!("risk policy blocked claimed intent submit");
        store.mark_failed(&envelope.id)?;
        append_audit(
            profile,
            Some(envelope.id.clone()),
            agent_finance_core::AuditEventKind::Error,
            format!("blocked live intent {}", envelope.id),
            json!({ "risk": risk, "error": format!("{error:#}") }),
        )?;
        return Err(error);
    }
    let response = execute_intent(profile, &envelope.kind, mode, timeout_seconds).await;
    match response {
        Ok(response) => {
            let payload = submit_audit_payload(&envelope.kind, &risk, &response)?;
            append_audit(
                profile,
                Some(envelope.id.clone()),
                mode.audit_kind(&envelope.kind),
                format!("submitted intent {}", envelope.id),
                payload,
            )?;
            store.mark_submitted(&envelope.id)?;
            Ok(SubmitReport {
                intent_id: envelope.id,
                mode: format!("{mode:?}"),
                risk,
                response,
            })
        }
        Err(error) => {
            store.mark_failed(&envelope.id)?;
            append_audit(
                profile,
                Some(envelope.id.clone()),
                agent_finance_core::AuditEventKind::Error,
                format!("failed to submit intent {}", envelope.id),
                json!({ "risk": risk, "error": format!("{error:#}") }),
            )?;
            Err(error)
        }
    }
}

fn live_order_audit_lock(
    profile: &agent_finance_core::Profile,
    intent: &agent_finance_core::IntentKind,
    mode: WriteMode,
) -> Result<Option<agent_finance_core::AuditScopeLock>> {
    if !matches!(mode, WriteMode::Live)
        || !matches!(intent, agent_finance_core::IntentKind::Order(_))
    {
        return Ok(None);
    }
    let scope = format!(
        "daily-order-notional:{}:{}:{}:{}",
        profile.name,
        profile.provider.provider,
        profile.provider.environment,
        Utc::now().date_naive()
    );
    agent_finance_core::AuditScopeLock::acquire(&scope).map(Some)
}

fn submit_audit_payload(
    intent: &agent_finance_core::IntentKind,
    risk: &agent_finance_core::RiskDecision,
    response: &serde_json::Value,
) -> Result<serde_json::Value> {
    match intent {
        agent_finance_core::IntentKind::Order(intent) => {
            agent_finance_core::live_order_audit_payload(intent, risk, response)
        }
        _ => Ok(json!({ "risk": risk, "response": response })),
    }
}

fn check_intent(
    profile: &agent_finance_core::Profile,
    intent: &agent_finance_core::IntentKind,
    live: bool,
) -> Result<agent_finance_core::RiskDecision> {
    match intent {
        agent_finance_core::IntentKind::Order(intent) => {
            check_order_with_runtime_limits(profile, intent, live)
        }
        agent_finance_core::IntentKind::Cancel(intent) => Ok(
            agent_finance_core::check_cancel_intent(profile, intent, live),
        ),
        agent_finance_core::IntentKind::Transfer(intent) => Ok(
            agent_finance_core::check_transfer_intent(profile, intent, live),
        ),
    }
}

fn check_order_with_runtime_limits(
    profile: &agent_finance_core::Profile,
    intent: &agent_finance_core::OrderIntent,
    live: bool,
) -> Result<agent_finance_core::RiskDecision> {
    let runtime = if live {
        agent_finance_core::OrderRuntimeRisk {
            daily_order_notional_used_utc: Some(
                agent_finance_core::daily_live_order_notional_used_today(profile)?,
            ),
        }
    } else {
        agent_finance_core::OrderRuntimeRisk::default()
    };
    Ok(agent_finance_core::check_order_intent_with_runtime(
        profile, intent, live, &runtime,
    ))
}

async fn execute_intent(
    profile: &agent_finance_core::Profile,
    intent: &agent_finance_core::IntentKind,
    mode: WriteMode,
    timeout_seconds: u64,
) -> Result<serde_json::Value> {
    if matches!(mode, WriteMode::DryRun) {
        return plan_intent(profile, intent, mode);
    }
    let Some(binance_mode) = mode.binance_mode() else {
        unreachable!("dry-run returned above");
    };
    let client = binance_client(profile, timeout_seconds)?;
    match intent {
        agent_finance_core::IntentKind::Order(intent) => {
            client.submit_order(intent, binance_mode).await
        }
        agent_finance_core::IntentKind::Cancel(intent) => client.cancel_order(intent).await,
        agent_finance_core::IntentKind::Transfer(intent) => {
            client.submit_transfer(intent, binance_mode).await
        }
    }
}

fn plan_intent(
    profile: &agent_finance_core::Profile,
    intent: &agent_finance_core::IntentKind,
    mode: WriteMode,
) -> Result<serde_json::Value> {
    let planner = agent_finance_binance::BinancePlanner::new(binance_endpoints(profile));
    let request = match intent {
        agent_finance_core::IntentKind::Order(intent) => planner.order_request(intent, false)?,
        agent_finance_core::IntentKind::Cancel(intent) => planner.cancel_request(intent)?,
        agent_finance_core::IntentKind::Transfer(intent) => planner.transfer_request(intent)?,
    };
    Ok(json!({
        "dry_run": matches!(mode, WriteMode::DryRun),
        "mode": format!("{mode:?}"),
        "request": request,
        "note": "dry-run is offline and does not read Binance API credentials",
    }))
}

fn print_submit_report(json_output: bool, report: &SubmitReport) -> Result<()> {
    print_json_or_text(json_output, report, || {
        format!(
            "submitted intent {}\n{}",
            report.intent_id,
            serde_json::to_string_pretty(&report.response).unwrap()
        )
    })
}

fn save_intent_with_audit(
    profile: &agent_finance_core::Profile,
    envelope: &agent_finance_core::IntentEnvelope,
    risk: &agent_finance_core::RiskDecision,
    summary: String,
) -> Result<std::path::PathBuf> {
    let path = agent_finance_core::IntentStore::from_default_dir()?.save(envelope)?;
    append_audit(
        profile,
        Some(envelope.id.clone()),
        agent_finance_core::AuditEventKind::IntentCreated,
        summary,
        json!({ "intent": envelope, "risk": risk, "path": path }),
    )?;
    Ok(path)
}

fn append_audit(
    profile: &agent_finance_core::Profile,
    intent_id: Option<String>,
    kind: agent_finance_core::AuditEventKind,
    summary: String,
    payload: serde_json::Value,
) -> Result<()> {
    let event = agent_finance_core::AuditEvent {
        timestamp_utc: Utc::now(),
        profile: profile.name.clone(),
        provider: profile.provider.provider.to_string(),
        environment: profile.provider.environment.to_string(),
        intent_id,
        kind,
        summary,
        payload,
    };
    agent_finance_core::append_audit_event(&event)?;
    Ok(())
}

fn print_json_or_text<T, F>(json_output: bool, value: &T, text: F) -> Result<()>
where
    T: Serialize,
    F: FnOnce() -> String,
{
    if json_output {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", text());
    }
    Ok(())
}

fn explain_profile(profile: &agent_finance_core::Profile) -> String {
    let symbols = profile
        .risk
        .allowed_symbols
        .keys()
        .cloned()
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "profile: {}\nprovider: {}\nenvironment: {:?}\nallow_live: {}\nallowed_symbols: {}\nallowed_transfers: {}",
        profile.name,
        profile.provider.provider,
        profile.provider.environment,
        profile.risk.allow_live,
        symbols,
        profile
            .risk
            .allowed_transfers
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}
