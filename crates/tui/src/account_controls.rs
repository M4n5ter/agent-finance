use crossterm::event::{KeyCode, KeyEvent};

use crate::state::Action;

const TRANSFER_HINTS: &[&str] = &[
    "[/] transfer field",
    "left/right transfer",
    "t stage transfer",
];
const FUTURES_STATE_HINTS: &[&str] = &["u futures field", "i futures adjust", "f stage state"];

pub(crate) fn account_key_action(key: KeyEvent) -> Option<Action> {
    if !key.modifiers.is_empty() {
        return None;
    }
    match key.code {
        KeyCode::Up | KeyCode::Down | KeyCode::Char('c') => {
            crate::open_order_controls::open_order_key_action(key)
        }
        KeyCode::Char('[') => Some(Action::MoveTransferTicketField(-1)),
        KeyCode::Char(']') => Some(Action::MoveTransferTicketField(1)),
        KeyCode::Left => Some(Action::AdjustTransferTicketField(-1)),
        KeyCode::Right | KeyCode::Enter => Some(Action::AdjustTransferTicketField(1)),
        KeyCode::Char('u') => Some(Action::MoveFuturesStateTicketField(1)),
        KeyCode::Char('i') => Some(Action::AdjustFuturesStateTicketField(1)),
        KeyCode::Char('f') => Some(Action::StageFuturesStateTicket),
        KeyCode::Char('t') => Some(Action::StageTransferTicket),
        _ => None,
    }
}

pub(crate) fn account_key_hints() -> Vec<String> {
    crate::open_order_controls::OPEN_ORDER_HINTS
        .iter()
        .chain(TRANSFER_HINTS)
        .chain(FUTURES_STATE_HINTS)
        .chain(&["q quit"])
        .copied()
        .map(str::to_string)
        .collect()
}

pub(crate) fn transfer_section_hint() -> String {
    section_hint(TRANSFER_HINTS)
}

pub(crate) fn futures_state_section_hint() -> String {
    section_hint(FUTURES_STATE_HINTS)
}

fn section_hint(hints: &[&str]) -> String {
    hints
        .iter()
        .map(|hint| {
            hint.strip_prefix("transfer ")
                .or_else(|| hint.strip_prefix("futures "))
                .unwrap_or(hint)
        })
        .collect::<Vec<_>>()
        .join("  ")
}
