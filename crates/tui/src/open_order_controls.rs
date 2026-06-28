use crossterm::event::{KeyCode, KeyEvent};

use crate::state::Action;

pub(crate) const OPEN_ORDER_HINTS: &[&str] = &["up/down open order", "c stage cancel"];

pub(crate) fn open_order_key_action(key: KeyEvent) -> Option<Action> {
    if !key.modifiers.is_empty() {
        return None;
    }
    match key.code {
        KeyCode::Up => Some(Action::MoveOpenOrderSelection(-1)),
        KeyCode::Down => Some(Action::MoveOpenOrderSelection(1)),
        KeyCode::Char('c') => Some(Action::StageSelectedOpenOrderCancel),
        _ => None,
    }
}

pub(crate) fn open_order_key_hints() -> Vec<String> {
    OPEN_ORDER_HINTS
        .iter()
        .chain(&["q quit"])
        .copied()
        .map(str::to_string)
        .collect()
}

pub(crate) fn open_order_section_hint() -> String {
    OPEN_ORDER_HINTS.join("  ")
}
