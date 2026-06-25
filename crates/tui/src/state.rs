use std::collections::VecDeque;

use agent_finance_market::model::ProviderProfile;
use agent_finance_market::service;

use crate::config::TuiConfig;

#[derive(Debug, Clone)]
pub struct AppState {
    pub watchlist: Vec<String>,
    pub selected_symbol: usize,
    pub focused_panel: Panel,
    pub open_panels: Vec<Panel>,
    pub floating: Vec<FloatingPane>,
    pub task_log: VecDeque<TaskLogEntry>,
    pub provider_profiles: Vec<ProviderProfile>,
}

impl AppState {
    pub fn from_config(config: TuiConfig) -> Self {
        let open_panels = vec![
            Panel::Watchlist,
            Panel::Quote,
            Panel::History,
            Panel::Evidence,
            Panel::Research,
            Panel::ProviderHealth,
            Panel::TaskLog,
        ];
        Self {
            watchlist: config.watchlist,
            selected_symbol: 0,
            focused_panel: Panel::Watchlist,
            open_panels,
            floating: Vec::new(),
            task_log: VecDeque::new(),
            provider_profiles: service::provider_profiles(),
        }
    }

    pub fn selected_symbol(&self) -> Option<&str> {
        self.watchlist.get(self.selected_symbol).map(String::as_str)
    }

    pub fn reduce(&mut self, action: Action) {
        match action {
            Action::Focus(panel) => {
                if self.has_panel(panel) {
                    self.focused_panel = panel;
                }
            }
            Action::SelectNextSymbol => self.shift_symbol(1),
            Action::SelectPreviousSymbol => self.shift_symbol(-1),
            Action::ToggleFloating(kind) => self.toggle_floating(kind),
            Action::CloseFocusedFloating => {
                self.floating.pop();
            }
            Action::ResetLayout => {
                self.floating.clear();
                self.focused_panel = Panel::Watchlist;
            }
            Action::Log(message) => self.push_log(TaskLogEntry::info(message)),
        }
    }

    fn has_panel(&self, panel: Panel) -> bool {
        self.open_panels.contains(&panel)
    }

    fn shift_symbol(&mut self, direction: isize) {
        if self.watchlist.is_empty() {
            self.selected_symbol = 0;
            return;
        }

        let len = self.watchlist.len() as isize;
        let selected = self.selected_symbol as isize;
        self.selected_symbol = (selected + direction).rem_euclid(len) as usize;
    }

    fn toggle_floating(&mut self, kind: FloatingKind) {
        if let Some(index) = self.floating.iter().position(|pane| pane.kind == kind) {
            self.floating.remove(index);
            return;
        }

        let next_z = self
            .floating
            .iter()
            .map(|pane| pane.z_index)
            .max()
            .unwrap_or(0)
            + 1;
        self.floating.push(FloatingPane {
            kind,
            z_index: next_z,
        });
    }

    fn push_log(&mut self, entry: TaskLogEntry) {
        self.task_log.push_back(entry);
        while self.task_log.len() > 200 {
            self.task_log.pop_front();
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Panel {
    Watchlist,
    Quote,
    History,
    Evidence,
    Research,
    ProviderHealth,
    TaskLog,
}

impl Panel {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Watchlist => "Watchlist",
            Self::Quote => "Quote / Sessions",
            Self::History => "History Chart",
            Self::Evidence => "Crypto Evidence",
            Self::Research => "News / Research",
            Self::ProviderHealth => "Provider Health",
            Self::TaskLog => "Task Log",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FloatingKind {
    CommandPalette,
    Help,
    ProviderDetails,
}

impl FloatingKind {
    pub const fn title(self) -> &'static str {
        match self {
            Self::CommandPalette => "Command Palette",
            Self::Help => "Help",
            Self::ProviderDetails => "Provider Details",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FloatingPane {
    pub kind: FloatingKind,
    pub z_index: u16,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Action {
    Focus(Panel),
    SelectNextSymbol,
    SelectPreviousSymbol,
    ToggleFloating(FloatingKind),
    CloseFocusedFloating,
    ResetLayout,
    Log(String),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TaskLevel {
    Info,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TaskLogEntry {
    pub level: TaskLevel,
    pub message: String,
}

impl TaskLogEntry {
    fn info(message: String) -> Self {
        Self {
            level: TaskLevel::Info,
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reducer_wraps_symbol_focus_across_watchlist_boundaries() {
        let mut state = AppState::from_config(TuiConfig {
            watchlist: vec!["AAPL".to_string(), "CRDO".to_string()],
            ..TuiConfig::default()
        });

        state.reduce(Action::SelectPreviousSymbol);

        assert_eq!(state.selected_symbol(), Some("CRDO"));

        state.reduce(Action::SelectNextSymbol);

        assert_eq!(state.selected_symbol(), Some("AAPL"));
    }

    #[test]
    fn floating_panes_keep_newest_overlay_on_top() {
        let mut state = AppState::from_config(TuiConfig::default());

        state.reduce(Action::ToggleFloating(FloatingKind::Help));
        state.reduce(Action::ToggleFloating(FloatingKind::CommandPalette));

        assert_eq!(state.floating[0].z_index, 1);
        assert_eq!(state.floating[1].z_index, 2);

        state.reduce(Action::CloseFocusedFloating);

        assert_eq!(state.floating.len(), 1);
        assert_eq!(state.floating[0].kind, FloatingKind::Help);
    }
}
