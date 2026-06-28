use crossterm::event::{KeyCode, KeyEvent};

use crate::state::Action;

const ORDER_TICKET_HINTS: &[&str] = &[
    "up/down field",
    "left/right adjust",
    "enter adjust",
    "s stage order",
];

pub(crate) fn order_ticket_key_action(key: KeyEvent) -> Option<Action> {
    if !key.modifiers.is_empty() {
        return None;
    }
    match key.code {
        KeyCode::Up => Some(Action::MoveOrderTicketField(-1)),
        KeyCode::Down => Some(Action::MoveOrderTicketField(1)),
        KeyCode::Left => Some(Action::AdjustOrderTicketField(-1)),
        KeyCode::Right | KeyCode::Enter => Some(Action::AdjustOrderTicketField(1)),
        KeyCode::Char('s') => Some(Action::StageOrderTicket),
        _ => None,
    }
}

pub(crate) fn order_ticket_key_hints() -> Vec<String> {
    ORDER_TICKET_HINTS
        .iter()
        .chain(&["q quit"])
        .copied()
        .map(str::to_string)
        .collect()
}

pub(crate) fn order_ticket_panel_hint() -> String {
    ORDER_TICKET_HINTS.join("  ")
}
