use agent_finance_core::{DiagnosticCheck, ProfilePermission, RiskPolicy};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::model::Panel;
use crate::profile_snapshot::ProfileValidationState;
use crate::state::{AppState, StagedChangeQueueStatus};
use crate::task_log::{TaskLogEntry, TaskStatus};

use super::widgets::{compact_text, panel_block};

pub(super) fn render_risk_audit(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "trading gate",
                state.theme.accent_style().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "  live:{} / effective:{}",
                if state.live_writes_enabled {
                    "on"
                } else {
                    "off"
                },
                state.effective_submit_mode()
            )),
        ]),
        profile_validation_line(state),
    ];
    lines.extend(profile_validation_failures(state));
    lines.extend(risk_policy_lines(state));
    lines.extend(staged_queue_lines(state));
    lines.extend(recent_event_lines(state));

    frame.render_widget(
        Paragraph::new(lines)
            .block(panel_block(Panel::RiskAudit, state))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn profile_validation_line(state: &AppState) -> Line<'static> {
    match &state.profile_validation {
        ProfileValidationState::Idle => match state.trading_profile.as_deref() {
            Some(profile) => Line::from(format!("profile validation: {profile} pending")),
            None => Line::from(Span::styled(
                "profile validation: no profile",
                state.theme.warning_style(),
            )),
        },
        ProfileValidationState::Loading { profile } => {
            Line::from(format!("profile validation: {profile} loading"))
        }
        ProfileValidationState::Ready {
            profile,
            path,
            checks,
            ..
        } => {
            let failures = required_failure_count(checks);
            if failures == 0 {
                Line::from(format!(
                    "profile validation: {profile} ok  path={}",
                    path.display()
                ))
            } else {
                Line::from(Span::styled(
                    format!(
                        "profile validation: {profile} {failures} required failure(s)  path={}",
                        path.display()
                    ),
                    state.theme.warning_style(),
                ))
            }
        }
        ProfileValidationState::Failed { profile, error } => Line::from(Span::styled(
            format!("profile validation: {profile} failed  {error}"),
            state.theme.warning_style(),
        )),
    }
}

fn profile_validation_failures(state: &AppState) -> Vec<Line<'static>> {
    let ProfileValidationState::Ready { checks, .. } = &state.profile_validation else {
        return Vec::new();
    };

    checks
        .iter()
        .filter(|check| check.required && !check.ok)
        .take(2)
        .map(|check| {
            Line::from(Span::styled(
                format!("required failure: {}", compact_text(&check.message, 70)),
                state.theme.warning_style(),
            ))
        })
        .collect()
}

fn risk_policy_lines(state: &AppState) -> Vec<Line<'static>> {
    let ProfileValidationState::Ready { profile_config, .. } = &state.profile_validation else {
        return vec![Line::from(
            "risk policy: unavailable until profile validation completes",
        )];
    };
    let risk = &profile_config.risk;
    let mut lines = vec![
        Line::from(format!(
            "risk policy: live:{}  daily order cap:{}",
            if risk.allow_live {
                "allowed"
            } else {
                "blocked"
            },
            risk.max_daily_order_notional_usdt
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "none".to_string())
        )),
        Line::from(format!(
            "required permissions: {}",
            permission_list_or_none(risk.required_profile_permissions().iter())
        )),
    ];
    lines.push(Line::from(format!(
        "symbols: {}",
        risk_symbol_summary(risk)
    )));
    lines.push(Line::from(format!(
        "transfers:{}  futures-state:{}",
        risk.allowed_transfers.len(),
        risk.allowed_futures_state_changes.len()
    )));
    lines
}

fn staged_queue_lines(state: &AppState) -> Vec<Line<'static>> {
    let changes = state.staged_change_views();
    let mut lines = vec![Line::from("")];
    if changes.is_empty() {
        lines.push(Line::from("staged queue: empty"));
    } else {
        lines.push(Line::from(format!(
            "staged queue: total:{}  {}",
            changes.len(),
            queue_status_summary(&changes)
        )));
    }
    if let Some(request) = state.pending_staged_confirmation() {
        lines.push(Line::from(Span::styled(
            format!(
                "confirmation pending: {} {}",
                request.kind_label(),
                compact_text(&request.summary(), 54)
            ),
            state.theme.warning_style(),
        )));
    }
    lines
}

fn recent_event_lines(state: &AppState) -> Vec<Line<'static>> {
    let events = state.task_log.iter().rev().take(4).collect::<Vec<_>>();
    let mut lines = vec![Line::from(""), Line::from("recent events")];
    if events.is_empty() {
        lines.push(Line::from("no runtime events yet"));
        return lines;
    }
    lines.extend(events.into_iter().map(task_log_line));
    lines
}

fn task_log_line(entry: &TaskLogEntry) -> Line<'static> {
    Line::from(format!(
        "{} {}",
        task_status_label(entry.status),
        compact_text(&entry.message, 72)
    ))
}

fn task_status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Info => "info",
        TaskStatus::Running => "running",
        TaskStatus::Succeeded => "ok",
        TaskStatus::Warning => "warn",
        TaskStatus::Failed => "fail",
    }
}

fn required_failure_count(checks: &[DiagnosticCheck]) -> usize {
    checks
        .iter()
        .filter(|check| check.required && !check.ok)
        .count()
}

fn permission_list_or_none(values: impl Iterator<Item = ProfilePermission>) -> String {
    let labels = values.map(|value| value.to_string()).collect::<Vec<_>>();
    if labels.is_empty() {
        "none".to_string()
    } else {
        labels.join(",")
    }
}

fn risk_symbol_summary(risk: &RiskPolicy) -> String {
    if risk.allowed_symbols.is_empty() {
        "none".to_string()
    } else {
        let first = risk
            .allowed_symbols
            .iter()
            .take(3)
            .map(|(symbol, policy)| format!("{symbol} <= {}", policy.max_order_notional_usdt))
            .collect::<Vec<_>>()
            .join("; ");
        let hidden = risk.allowed_symbols.len().saturating_sub(3);
        if hidden == 0 {
            first
        } else {
            format!("{first}; +{hidden} more")
        }
    }
}

fn queue_status_summary(changes: &[crate::state::StagedChangeView]) -> String {
    let counts = [
        (StagedChangeQueueStatus::Draft, "draft"),
        (StagedChangeQueueStatus::Ready, "ready"),
        (StagedChangeQueueStatus::Running, "running"),
        (StagedChangeQueueStatus::Done, "done"),
        (StagedChangeQueueStatus::Failed, "failed"),
        (StagedChangeQueueStatus::Closed, "closed"),
    ]
    .into_iter()
    .filter_map(|(status, label)| {
        let count = changes
            .iter()
            .filter(|change| change.stage.queue_status() == status)
            .count();
        (count > 0).then(|| format!("{label}:{count}"))
    })
    .collect::<Vec<_>>();
    counts.join(" ")
}
