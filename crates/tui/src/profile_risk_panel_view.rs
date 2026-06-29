use ratatui::style::Modifier;
use ratatui::text::{Line, Span};

use crate::model::Panel;
use crate::mouse_target::MouseTarget;
use crate::profile_risk_controls::{PROFILE_RISK_ACTIONS, ProfileRiskActionSpec};
use crate::profile_snapshot::{ProfileValidationState, TradingProfileSnapshot};
use crate::state::AppState;

use crate::render::profile_policy::{
    ProfilePolicyFormat, profile_policy_heading, profile_policy_lines,
};
use crate::render::widgets::compact_text;

pub(crate) struct ProfileRiskPanelRow {
    pub line: Line<'static>,
    pub action: Option<ProfileRiskActionSpec>,
}

impl ProfileRiskPanelRow {
    fn text(text: impl Into<String>) -> Self {
        Self {
            line: Line::from(text.into()),
            action: None,
        }
    }

    fn line(line: Line<'static>) -> Self {
        Self { line, action: None }
    }

    fn action(
        state: &AppState,
        mouse_target: Option<MouseTarget>,
        action: ProfileRiskActionSpec,
    ) -> Self {
        let hovered = mouse_target
            .is_some_and(|target| target.panel_action_hovered(Panel::ProfileRisk, action.action));
        let style = if hovered {
            state.theme.selected_style().add_modifier(Modifier::BOLD)
        } else {
            state.theme.accent_style()
        };
        Self {
            line: Line::from(Span::styled(action.label, style)),
            action: Some(action),
        }
    }
}

pub(crate) fn rows(
    state: &AppState,
    mouse_target: Option<MouseTarget>,
) -> Vec<ProfileRiskPanelRow> {
    let mut rows = vec![
        ProfileRiskPanelRow::line(profile_policy_heading(&state.theme)),
        ProfileRiskPanelRow::text(format!(
            "selected profile: {}",
            state.trading_profile.as_deref().unwrap_or("-")
        )),
        ProfileRiskPanelRow::line(validation_summary_line(state)),
    ];

    match &state.profile_validation {
        ProfileValidationState::Ready {
            profile_config,
            checks,
            path,
            ..
        } => {
            rows.push(ProfileRiskPanelRow::text(compact_text(
                &format!("path: {}", path.display()),
                96,
            )));
            let profile = TradingProfileSnapshot::from(profile_config.as_ref());
            rows.extend(
                profile_policy_lines(&state.theme, &profile, ProfilePolicyFormat::ProfileRisk)
                    .into_iter()
                    .map(compact_line)
                    .map(ProfileRiskPanelRow::line),
            );
            rows.extend(required_failure_lines(state, checks));
        }
        ProfileValidationState::Failed { error, .. } => {
            rows.push(ProfileRiskPanelRow::line(Line::from(Span::styled(
                compact_text(error, 96),
                state.theme.warning_style(),
            ))));
        }
        ProfileValidationState::Loading { .. } | ProfileValidationState::Idle => {}
    }

    rows.push(ProfileRiskPanelRow::text(""));
    rows.extend(
        PROFILE_RISK_ACTIONS.map(|action| ProfileRiskPanelRow::action(state, mouse_target, action)),
    );
    rows
}

pub(crate) fn action_at_content_row(
    state: &AppState,
    content_row: usize,
) -> Option<ProfileRiskActionSpec> {
    rows(state, None).get(content_row)?.action
}

fn validation_summary_line(state: &AppState) -> Line<'static> {
    match &state.profile_validation {
        ProfileValidationState::Idle if state.trading_profile.is_some() => {
            Line::from("validation: pending")
        }
        ProfileValidationState::Idle => Line::from("validation: no profile selected"),
        ProfileValidationState::Loading { profile } => {
            Line::from(format!("validation: {profile} loading"))
        }
        ProfileValidationState::Ready { checks, .. } => {
            let required_failures = checks
                .iter()
                .filter(|check| check.required && !check.ok)
                .count();
            if required_failures == 0 {
                Line::from(Span::styled("validation: ok", state.theme.success_style()))
            } else {
                Line::from(Span::styled(
                    format!("validation: {required_failures} required failure(s)"),
                    state.theme.warning_style(),
                ))
            }
        }
        ProfileValidationState::Failed { profile, .. } => Line::from(Span::styled(
            format!("validation: {profile} failed"),
            state.theme.warning_style(),
        )),
    }
}

fn required_failure_lines(
    state: &AppState,
    checks: &[agent_finance_core::DiagnosticCheck],
) -> Vec<ProfileRiskPanelRow> {
    checks
        .iter()
        .filter(|check| check.required && !check.ok)
        .take(3)
        .map(|check| {
            ProfileRiskPanelRow::line(Line::from(Span::styled(
                compact_text(&format!("failure: {}", check.message), 96),
                state.theme.warning_style(),
            )))
        })
        .collect()
}

fn compact_line(line: Line<'static>) -> Line<'static> {
    let text = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();
    if text.chars().count() <= 96 {
        return line;
    }
    Line::from(Span::styled(compact_text(&text, 96), line.style))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::ActionId;

    #[test]
    fn rows_mark_profile_risk_actions_as_clickable_metadata() {
        let state = AppState::from_config(crate::config::TuiConfig::default());

        let actions = rows(&state, None)
            .into_iter()
            .filter_map(|row| row.action.map(|action| action.action))
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![
                ActionId::OpenFloating(crate::model::FloatingKind::TradingProfile),
                ActionId::RevalidateTradingProfile,
                ActionId::StageProfileLiveToggle,
            ]
        );
    }
}
