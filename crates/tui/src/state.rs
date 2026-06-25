use std::collections::VecDeque;

use agent_finance_market::history_snapshot::HistorySnapshot;
use agent_finance_market::model::ProviderProfile;
use agent_finance_market::service;
use agent_finance_market::snapshot::MarketSnapshot;

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
    pub market_snapshot: Option<MarketSnapshot>,
    pub refresh: LoadSlot<()>,
    pub history_snapshot: Option<HistorySnapshot>,
    pub history: LoadSlot<String>,
    pub scheduler_error: Option<String>,
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
            market_snapshot: None,
            refresh: LoadSlot::new(),
            history_snapshot: None,
            history: LoadSlot::new(),
            scheduler_error: None,
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
            Action::RefreshStarted(generation) => {
                self.refresh.start(generation, ());
            }
            Action::SnapshotLoaded {
                generation,
                snapshot,
            } => {
                if self.refresh.finish(generation) {
                    if !snapshot.errors.is_empty() {
                        self.push_log(TaskLogEntry::warning(format!(
                            "refresh completed with {} provider errors",
                            snapshot.errors.len()
                        )));
                    } else {
                        self.push_log(TaskLogEntry::info("market snapshot refreshed".to_string()));
                    }
                    self.market_snapshot = Some(snapshot);
                } else {
                    self.push_log(TaskLogEntry::warning(format!(
                        "ignored stale market snapshot generation {generation}",
                    )));
                }
            }
            Action::RefreshFailed { generation, error } => {
                if self.refresh.finish(generation) {
                    self.push_log(TaskLogEntry::warning(format!(
                        "market refresh failed: {error}"
                    )));
                }
            }
            Action::HistoryStarted { generation, symbol } => {
                self.history.start(generation, symbol);
            }
            Action::HistoryLoaded {
                generation,
                snapshot,
            } => {
                if self.history.finish(generation) {
                    if !snapshot.errors.is_empty() {
                        self.push_log(TaskLogEntry::warning(format!(
                            "history loaded with {} warnings",
                            snapshot.errors.len()
                        )));
                    } else {
                        self.push_log(TaskLogEntry::info(format!(
                            "{} history loaded",
                            snapshot.symbol
                        )));
                    }
                    self.history_snapshot = Some(snapshot);
                } else {
                    self.push_log(TaskLogEntry::warning(format!(
                        "ignored stale history generation {generation}",
                    )));
                }
            }
            Action::HistoryFailed {
                generation,
                symbol,
                error,
            } => {
                if self.history.finish(generation) {
                    self.push_log(TaskLogEntry::warning(format!(
                        "{symbol} history failed: {error}"
                    )));
                }
            }
            Action::SchedulerFailed(error) => {
                self.refresh.stop();
                self.history.stop();
                self.scheduler_error = Some(error.clone());
                self.push_log(TaskLogEntry::warning(format!("scheduler failed: {error}")));
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LoadSlot<K> {
    pub generation: u64,
    pub loading: bool,
    pub key: Option<K>,
}

impl<K> LoadSlot<K> {
    fn new() -> Self {
        Self {
            generation: 0,
            loading: false,
            key: None,
        }
    }

    fn start(&mut self, generation: u64, key: K) {
        self.generation = generation;
        self.loading = true;
        self.key = Some(key);
    }

    fn finish(&mut self, generation: u64) -> bool {
        if generation != self.generation {
            return false;
        }
        self.loading = false;
        true
    }

    fn stop(&mut self) {
        self.loading = false;
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

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Focus(Panel),
    SelectNextSymbol,
    SelectPreviousSymbol,
    ToggleFloating(FloatingKind),
    CloseFocusedFloating,
    ResetLayout,
    RefreshStarted(u64),
    SnapshotLoaded {
        generation: u64,
        snapshot: MarketSnapshot,
    },
    RefreshFailed {
        generation: u64,
        error: String,
    },
    HistoryStarted {
        generation: u64,
        symbol: String,
    },
    HistoryLoaded {
        generation: u64,
        snapshot: HistorySnapshot,
    },
    HistoryFailed {
        generation: u64,
        symbol: String,
        error: String,
    },
    SchedulerFailed(String),
    Log(String),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TaskLevel {
    Info,
    Warning,
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

    fn warning(message: String) -> Self {
        Self {
            level: TaskLevel::Warning,
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_finance_market::history_snapshot::HistorySnapshot;
    use agent_finance_market::snapshot::{QuoteSnapshot, RegularBasisSnapshot};

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

    #[test]
    fn reducer_accepts_current_snapshot_and_ignores_stale_snapshot() {
        let mut state = AppState::from_config(TuiConfig::default());
        let current = snapshot(2, "CRDO");
        let stale = snapshot(1, "AAPL");

        state.reduce(Action::RefreshStarted(2));
        state.reduce(Action::SnapshotLoaded {
            generation: 1,
            snapshot: stale,
        });
        assert!(state.market_snapshot.is_none());
        assert!(state.refresh.loading);

        state.reduce(Action::SnapshotLoaded {
            generation: 2,
            snapshot: current,
        });
        assert_eq!(
            state
                .market_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.quote_for("CRDO"))
                .and_then(|quote| quote.price),
            Some(250.0)
        );
        assert!(!state.refresh.loading);
    }

    #[test]
    fn reducer_clears_in_flight_refresh_on_scheduler_fatal_failure() {
        let mut state = AppState::from_config(TuiConfig::default());

        state.reduce(Action::RefreshStarted(1));
        state.reduce(Action::HistoryStarted {
            generation: 1,
            symbol: "CRDO".to_string(),
        });
        state.reduce(Action::SchedulerFailed(
            "scheduler runtime failed".to_string(),
        ));

        assert!(!state.refresh.loading);
        assert!(!state.history.loading);
        assert_eq!(
            state.scheduler_error.as_deref(),
            Some("scheduler runtime failed")
        );
    }

    #[test]
    fn reducer_accepts_current_history_and_ignores_stale_history() {
        let mut state = AppState::from_config(TuiConfig::default());

        state.reduce(Action::HistoryStarted {
            generation: 2,
            symbol: "CRDO".to_string(),
        });
        state.reduce(Action::HistoryLoaded {
            generation: 1,
            snapshot: history_snapshot("AAPL", 100.0),
        });
        assert!(state.history_snapshot.is_none());
        assert!(state.history.loading);

        state.reduce(Action::HistoryLoaded {
            generation: 2,
            snapshot: history_snapshot("CRDO", 250.0),
        });
        assert_eq!(
            state
                .history_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.latest_close),
            Some(250.0)
        );
        assert!(!state.history.loading);
    }

    fn snapshot(_generation: u64, symbol: &str) -> MarketSnapshot {
        MarketSnapshot {
            fetched_at_local: Some("2026-06-25 09:30:00".to_string()),
            quotes: vec![QuoteSnapshot {
                symbol: symbol.to_string(),
                price: Some(250.0),
                currency: Some("USD".to_string()),
                provider: "test".to_string(),
                session: Some("regular".to_string()),
                market_time_local: None,
                change_pct: Some(1.0),
                aliases: Vec::new(),
                regular_basis: RegularBasisSnapshot {
                    previous_close: Some(247.0),
                    open: None,
                    high: None,
                    low: None,
                    volume: None,
                },
            }],
            errors: Vec::new(),
        }
    }

    fn history_snapshot(symbol: &str, latest_close: f64) -> HistorySnapshot {
        HistorySnapshot {
            requested_symbol: symbol.to_string(),
            symbol: symbol.to_string(),
            provider: "test".to_string(),
            interval: "1d".to_string(),
            fetched_at_local: Some("2026-06-25 09:30:00".to_string()),
            latest_close: Some(latest_close),
            latest_time: Some("2026-06-25".to_string()),
            return_pct: Some(1.0),
            volume: Some(10_000.0),
            bars: Vec::new(),
            errors: Vec::new(),
        }
    }
}
