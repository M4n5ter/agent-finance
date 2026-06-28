use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::model::Panel;
use crate::profile_snapshot::ProfileValidationState;
use crate::settings_editor::SettingRow;
use crate::state::AppState;

use super::widgets::panel_block;

pub(super) fn render_settings(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let dirty = if state.config_changes.is_empty() {
        "clean".to_string()
    } else {
        state.config_changes.join(", ")
    };
    let profile = state.trading_profile.as_deref().unwrap_or("-");
    let mut lines = vec![
        Line::from(Span::styled(
            "configuration cockpit",
            state.theme.accent_style().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("workspace: {}", state.workspace)),
        Line::from(format!("dirty config: {dirty}")),
        Line::from(format!(
            "watchlist: {} symbols  selected={}",
            state.watchlist.len(),
            state.selected_symbol().unwrap_or("-")
        )),
        Line::from(format!(
            "trading profile: {profile}  live writes={}",
            if state.live_writes_enabled {
                "on"
            } else {
                "off"
            }
        )),
        profile_validation_line(state),
        Line::from(format!(
            "default submit mode: {}  effective={}",
            state.default_submit_mode,
            state.effective_submit_mode()
        )),
        Line::from(format!(
            "provider preferences: equity={}  crypto={}",
            state.providers.equity, state.providers.crypto
        )),
        Line::from(format!(
            "theme: accent={}  selection={}/{}",
            state.theme.accent, state.theme.selection_background, state.theme.selection_foreground
        )),
        Line::from(format!(
            "provider capability profiles: {}",
            state.provider_profiles.len()
        )),
        Line::from(format!(
            "normal key bindings: {}",
            state.keymap.normal_len()
        )),
        Line::from(""),
        Line::from("settings editor"),
    ];
    lines.extend(profile_validation_failures(state));
    lines.extend(setting_rows(state));
    lines.extend([
        Line::from(""),
        Line::from(crate::settings_controls::settings_panel_hint()),
        Line::from(""),
        Line::from("available editors"),
        Line::from(
            ": command palette  a add symbols  d delete symbol  watchlist left/right reorder",
        ),
        Line::from(crate::settings_controls::settings_profile_risk_hint()),
        Line::from("save/undo: command palette -> Save config / Undo config change"),
    ]);
    for change in state.config_changes.iter().take(3) {
        lines.push(Line::from(Span::styled(
            format!("pending: {change}"),
            state.theme.warning_style(),
        )));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(panel_block(Panel::Settings, state))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn profile_validation_line(state: &AppState) -> Line<'static> {
    match &state.profile_validation {
        ProfileValidationState::Idle => {
            let Some(profile) = state.trading_profile.as_deref() else {
                return Line::from("profile validation: no profile");
            };
            Line::from(format!("profile validation: {profile} pending"))
        }
        ProfileValidationState::Loading { profile } => {
            Line::from(format!("profile validation: {profile} loading"))
        }
        ProfileValidationState::Ready { path, checks, .. } => {
            let failures = checks
                .iter()
                .filter(|check| check.required && !check.ok)
                .count();
            if failures == 0 {
                Line::from(format!("profile validation: ok  path={}", path.display()))
            } else {
                Line::from(Span::styled(
                    format!(
                        "profile validation: {failures} required failure(s)  path={}",
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
        .take(3)
        .map(|check| {
            Line::from(Span::styled(
                format!("profile validation failure: {}", check.message),
                state.theme.warning_style(),
            ))
        })
        .collect()
}

fn setting_rows(state: &AppState) -> Vec<Line<'static>> {
    SettingRow::ALL
        .into_iter()
        .map(|row| {
            let selected = state.settings_editor.selected() == row;
            let marker = if selected { ">" } else { " " };
            let value = row.value(&state.providers, &state.theme, &state.keymap);
            let text = format!("{marker} {}: {value}", row.label());
            if selected {
                Line::from(Span::styled(
                    text,
                    state.theme.accent_style().add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(text)
            }
        })
        .collect()
}
