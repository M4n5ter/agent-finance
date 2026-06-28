use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui_input::backend::crossterm::to_input_request;

use crate::model::FloatingKind;
use crate::state::{Action, AppState};

pub(crate) struct FloatingKeyRouting {
    kind: FloatingKeyRoute,
    action: Option<Action>,
}

impl FloatingKeyRouting {
    fn captured(action: Option<Action>) -> Self {
        Self {
            kind: FloatingKeyRoute::Captured,
            action,
        }
    }

    fn pass_through() -> Self {
        Self {
            kind: FloatingKeyRoute::PassThrough,
            action: None,
        }
    }

    pub(crate) fn captured_action(self) -> Option<Option<Action>> {
        match self.kind {
            FloatingKeyRoute::Captured => Some(self.action),
            FloatingKeyRoute::PassThrough => None,
        }
    }
}

enum FloatingKeyRoute {
    Captured,
    PassThrough,
}

pub(crate) fn key_route(state: &AppState, key: KeyEvent) -> FloatingKeyRouting {
    let action = match top_floating_kind(state) {
        Some(FloatingKind::CommandPalette) => command_palette_key_action(state, key),
        Some(FloatingKind::SymbolSearch) => symbol_search_key_action(key),
        Some(FloatingKind::WatchlistAdd) => watchlist_add_key_action(key),
        Some(FloatingKind::TradingProfile) => trading_profile_key_action(key),
        Some(FloatingKind::LiveWritesConfirmation) => live_writes_confirmation_key_action(key),
        Some(FloatingKind::StagedExecutionConfirmation) => {
            staged_execution_confirmation_key_action(key)
        }
        Some(FloatingKind::Help | FloatingKind::ProviderDetails) | None => {
            return FloatingKeyRouting::pass_through();
        }
    };
    FloatingKeyRouting::captured(action)
}

pub(crate) fn live_writes_confirmation_is_top(state: &AppState) -> bool {
    top_floating_kind(state) == Some(FloatingKind::LiveWritesConfirmation)
}

pub(crate) fn staged_execution_confirmation_is_top(state: &AppState) -> bool {
    top_floating_kind(state) == Some(FloatingKind::StagedExecutionConfirmation)
}

pub(crate) fn text_input_floating_is_top(state: &AppState) -> bool {
    top_floating_kind(state).is_some_and(FloatingKind::text_input)
}

fn top_floating_kind(state: &AppState) -> Option<FloatingKind> {
    state.floating.last().map(|pane| pane.kind)
}

fn command_palette_key_action(state: &AppState, key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Down => Some(Action::MoveCommandSelection(1)),
        KeyCode::Up => Some(Action::MoveCommandSelection(-1)),
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::MoveCommandSelection(1))
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::MoveCommandSelection(-1))
        }
        KeyCode::Enter => state.command_palette.selected_action().map(Action::Execute),
        KeyCode::Esc => Some(Action::CloseFocusedFloating),
        _ => to_input_request(&Event::Key(key)).map(Action::EditCommandQuery),
    }
}

fn symbol_search_key_action(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Down => Some(Action::MoveSymbolSearchSelection(1)),
        KeyCode::Up => Some(Action::MoveSymbolSearchSelection(-1)),
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::MoveSymbolSearchSelection(1))
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::MoveSymbolSearchSelection(-1))
        }
        KeyCode::Enter => Some(Action::AcceptSymbolSearch),
        KeyCode::Esc => Some(Action::CloseFocusedFloating),
        _ => to_input_request(&Event::Key(key)).map(Action::EditSymbolSearchQuery),
    }
}

fn watchlist_add_key_action(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Enter => Some(Action::AcceptWatchlistAdd),
        KeyCode::Esc => Some(Action::CloseFocusedFloating),
        _ => to_input_request(&Event::Key(key)).map(Action::EditWatchlistAddQuery),
    }
}

fn trading_profile_key_action(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Enter => Some(Action::AcceptTradingProfile),
        KeyCode::Esc => Some(Action::CloseFocusedFloating),
        _ => to_input_request(&Event::Key(key)).map(Action::EditTradingProfileQuery),
    }
}

fn live_writes_confirmation_key_action(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Enter => Some(Action::SetLiveWritesEnabled(true)),
        KeyCode::Esc => Some(Action::CloseFocusedFloating),
        _ => None,
    }
}

fn staged_execution_confirmation_key_action(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Enter => Some(Action::ConfirmStagedExecution),
        KeyCode::Esc => Some(Action::CancelStagedExecutionConfirmation),
        _ => None,
    }
}
