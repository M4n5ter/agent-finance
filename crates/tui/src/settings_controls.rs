use crossterm::event::{KeyCode, KeyEvent};

use crate::command::ActionId;
use crate::model::FloatingKind;
use crate::state::Action;

const PANEL_CONTROL_HINTS: &[&str] = &[
    "up/down select setting",
    "left/right adjust",
    "enter next value",
    "e profile",
    "v validate",
    "t risk",
    "u undo",
];

pub(crate) fn settings_key_action(key: KeyEvent) -> Option<Action> {
    if !key.modifiers.is_empty() {
        return None;
    }
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveSettingsSelection(-1)),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveSettingsSelection(1)),
        KeyCode::Left | KeyCode::Char('h') => Some(Action::AdjustSelectedSetting(-1)),
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
            Some(Action::AdjustSelectedSetting(1))
        }
        KeyCode::Char('e') => Some(Action::Execute(ActionId::OpenFloating(
            FloatingKind::TradingProfile,
        ))),
        KeyCode::Char('v') => Some(Action::Execute(ActionId::RevalidateTradingProfile)),
        KeyCode::Char('t') => Some(Action::Execute(ActionId::StageProfileLiveToggle)),
        KeyCode::Char('u') => Some(Action::UndoConfigChange),
        _ => None,
    }
}

pub(crate) fn settings_key_hints() -> Vec<String> {
    PANEL_CONTROL_HINTS
        .iter()
        .copied()
        .chain(["q quit"])
        .map(str::to_string)
        .collect()
}

pub(crate) fn settings_panel_hint() -> String {
    PANEL_CONTROL_HINTS.join("  ")
}

pub(crate) fn settings_profile_risk_hint() -> &'static str {
    "profile/risk: e edit profile  v revalidate  t stage risk.allow_live"
}
