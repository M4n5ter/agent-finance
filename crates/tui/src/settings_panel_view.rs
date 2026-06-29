use ratatui::style::Modifier;
use ratatui::text::{Line, Span};

use crate::model::Panel;
use crate::mouse_target::MouseTarget;
use crate::settings_editor::SettingRow;
use crate::state::AppState;

pub(crate) struct SettingsPanelRow {
    pub line: Line<'static>,
    pub setting_index: Option<usize>,
}

impl SettingsPanelRow {
    fn text(text: impl Into<String>) -> Self {
        Self {
            line: Line::from(text.into()),
            setting_index: None,
        }
    }

    fn line(line: Line<'static>) -> Self {
        Self {
            line,
            setting_index: None,
        }
    }

    fn setting(line: Line<'static>, index: usize) -> Self {
        Self {
            line,
            setting_index: Some(index),
        }
    }
}

pub(crate) fn rows(state: &AppState, mouse_target: Option<MouseTarget>) -> Vec<SettingsPanelRow> {
    let dirty = if state.config_changes.is_empty() {
        "clean".to_string()
    } else {
        state.config_changes.join(", ")
    };
    let profile = state.trading_profile.as_deref().unwrap_or("-");
    let mut rows = vec![
        SettingsPanelRow::line(Line::from(Span::styled(
            "configuration cockpit",
            state.theme.accent_style().add_modifier(Modifier::BOLD),
        ))),
        SettingsPanelRow::text(format!("workspace: {}", state.workspace)),
        SettingsPanelRow::text(format!("dirty config: {dirty}")),
        SettingsPanelRow::text(format!(
            "watchlist: {} symbols  selected={}",
            state.watchlist.len(),
            state.selected_symbol().unwrap_or("-")
        )),
        SettingsPanelRow::text(format!(
            "trading profile: {profile}  live writes={}",
            if state.live_writes_enabled {
                "on"
            } else {
                "off"
            }
        )),
        SettingsPanelRow::text(format!(
            "default submit mode: {}  effective={}",
            state.default_submit_mode,
            state.effective_submit_mode()
        )),
        SettingsPanelRow::text(format!(
            "provider preferences: equity={}  crypto={}",
            state.providers.equity, state.providers.crypto
        )),
        SettingsPanelRow::text(format!(
            "theme: accent={}  selection={}/{}",
            state.theme.accent, state.theme.selection_background, state.theme.selection_foreground
        )),
        SettingsPanelRow::text(format!(
            "provider capability profiles: {}",
            state.provider_profiles.len()
        )),
        SettingsPanelRow::text(format!(
            "normal key bindings: {}",
            state.keymap.normal_len()
        )),
        SettingsPanelRow::text(""),
        SettingsPanelRow::text("settings editor"),
    ];
    rows.extend(setting_rows(state, mouse_target));
    rows.extend([
        SettingsPanelRow::text(""),
        SettingsPanelRow::text(crate::settings_controls::settings_panel_hint()),
        SettingsPanelRow::text(""),
        SettingsPanelRow::text("available editors"),
        SettingsPanelRow::text(
            ": command palette  a add symbols  d delete symbol  watchlist left/right reorder",
        ),
        SettingsPanelRow::text(
            "profile/risk: use the Profile / Risk panel for validation and risk changes",
        ),
        SettingsPanelRow::text("save/undo: command palette -> Save config / Undo config change"),
    ]);
    rows.extend(state.config_changes.iter().take(3).map(|change| {
        SettingsPanelRow::line(Line::from(Span::styled(
            format!("pending: {change}"),
            state.theme.warning_style(),
        )))
    }));
    rows
}

pub(crate) fn setting_index_at_content_row(state: &AppState, content_row: usize) -> Option<usize> {
    rows(state, None).get(content_row)?.setting_index
}

fn setting_rows(state: &AppState, mouse_target: Option<MouseTarget>) -> Vec<SettingsPanelRow> {
    SettingRow::ALL
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let selected = state.settings_editor.selected() == row;
            let hovered =
                mouse_target.is_some_and(|target| target.panel_row_hovered(Panel::Settings, index));
            let marker = if selected { ">" } else { " " };
            let value = row.value(&state.providers, &state.theme, &state.keymap);
            let text = format!("{marker} {}: {value}", row.label());
            let line = if hovered {
                Line::from(Span::styled(
                    text,
                    state.theme.selected_style().add_modifier(Modifier::BOLD),
                ))
            } else if selected {
                Line::from(Span::styled(
                    text,
                    state.theme.accent_style().add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(text)
            };
            SettingsPanelRow::setting(line, index)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_mark_all_rendered_settings_as_clickable_metadata() {
        let state = AppState::from_config(crate::config::TuiConfig::default());

        let clickable = rows(&state, None)
            .into_iter()
            .filter_map(|row| row.setting_index)
            .collect::<Vec<_>>();

        assert_eq!(clickable, (0..SettingRow::ALL.len()).collect::<Vec<_>>());
    }
}
