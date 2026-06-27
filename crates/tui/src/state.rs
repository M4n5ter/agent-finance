use agent_finance_core::submit::SubmitMode;
use agent_finance_market::crypto_evidence_snapshot::CryptoQuoteEvidenceSnapshot;
use agent_finance_market::history_snapshot::HistorySnapshot;
use agent_finance_market::model::ProviderProfile;
use agent_finance_market::research_snapshot::ResearchContextSnapshot;
use agent_finance_market::service;
use agent_finance_market::snapshot::MarketSnapshot;

use crate::account::AccountSnapshot;
use crate::command::{ActionId, CommandPaletteState};
use crate::config::{FloatingConfig, LayoutConfig, PanelConfig, TuiConfig, WorkspaceConfig};
use crate::keymap::KeymapConfig;
use crate::model::{DockedPanels, FloatingKind, FloatingPane, FloatingSize, Panel, WorkspaceKind};
use crate::order_ticket::{OrderTicket, OrderTicketPreview};
use crate::search::SymbolSearchState;
use crate::task_failure::TaskFailures;
use crate::task_log::TaskLog;
use crate::theme::ThemeConfig;

mod interaction;
mod lifecycle;
mod load;
mod staged_change;
mod workspace;

use load::LoadSlot;
pub use load::{SelectedDataState, SelectedSymbolLoad, SymbolSnapshot};
#[cfg(test)]
pub use staged_change::StagedChangeStage;
use staged_change::{
    CloseStagedChangeResult, OpenStagedChangeResult, QueueOrderSubmitResult, StagedChanges,
    TransitionResult,
};
pub use staged_change::{
    OrderTicketReview, StagedChangeEvent, StagedChangeRequest, StagedChangeSubject,
    StagedChangeView, StagedOrderSubmitRequest,
};

#[derive(Debug, Clone)]
pub struct AppState {
    pub watchlist: Vec<String>,
    pub selected_symbol: usize,
    pub workspace: WorkspaceKind,
    pub zoomed: bool,
    pub layout: LayoutConfig,
    pub panels: DockedPanels,
    pub floating: Vec<FloatingPane>,
    pub command_palette: CommandPaletteState,
    pub symbol_search: SymbolSearchState,
    pub keymap: KeymapConfig,
    pub task_log: TaskLog,
    pub provider_profiles: Vec<ProviderProfile>,
    pub market_snapshot: Option<MarketSnapshot>,
    refresh: LoadSlot<()>,
    pub history: SelectedSymbolLoad<HistorySnapshot>,
    pub evidence: SelectedSymbolLoad<CryptoQuoteEvidenceSnapshot>,
    pub research: SelectedSymbolLoad<ResearchContextSnapshot>,
    account: LoadSlot<String>,
    pub account_snapshot: Option<AccountSnapshot>,
    pub task_failures: TaskFailures,
    pub scheduler_error: Option<String>,
    pub theme: ThemeConfig,
    pub default_submit_mode: SubmitMode,
    pub live_writes_enabled: bool,
    pub trading_profile: Option<String>,
    pub order_ticket: OrderTicket,
    staged_changes: StagedChanges,
    pending_staged_order_submit: Option<StagedOrderSubmitRequest>,
}

impl AppState {
    pub fn from_config(config: TuiConfig) -> Self {
        let mut state = Self {
            watchlist: config.watchlist,
            selected_symbol: 0,
            workspace: config.workspace.current,
            zoomed: false,
            layout: config.layout,
            panels: DockedPanels::from_open_focused(config.panels.open, config.panels.focused),
            floating: config.floating.panes,
            command_palette: CommandPaletteState::default(),
            symbol_search: SymbolSearchState::default(),
            keymap: config.keymap,
            task_log: TaskLog::default(),
            provider_profiles: service::provider_profiles(),
            market_snapshot: None,
            refresh: LoadSlot::new(),
            history: SelectedSymbolLoad::new(),
            evidence: SelectedSymbolLoad::new(),
            research: SelectedSymbolLoad::new(),
            account: LoadSlot::new(),
            account_snapshot: None,
            task_failures: TaskFailures::default(),
            scheduler_error: None,
            theme: config.theme,
            default_submit_mode: SubmitMode::DryRun,
            live_writes_enabled: false,
            trading_profile: config.trading.default_profile,
            order_ticket: OrderTicket::default(),
            staged_changes: StagedChanges::default(),
            pending_staged_order_submit: None,
        };
        state.ensure_visible_focus();
        state
    }

    pub fn export_config(&self, base: &TuiConfig) -> TuiConfig {
        let mut config = base.clone();
        config.watchlist = self.watchlist.clone();
        config.workspace = WorkspaceConfig {
            current: self.workspace,
        };
        config.layout = self.layout.clone();
        config.panels = PanelConfig {
            open: self.panels.open_panels().to_vec(),
            focused: self.panels.focused(),
        };
        config.floating = FloatingConfig {
            panes: self
                .floating
                .iter()
                .copied()
                .filter(|pane| pane.kind.persistent())
                .collect(),
        };
        config.keymap = self.keymap.clone();
        config.theme = self.theme.clone();
        config.trading.default_profile = self.trading_profile.clone();
        config.normalize();
        config
    }

    pub fn refresh_loading(&self) -> bool {
        self.refresh.loading()
    }

    pub fn account_loading(&self) -> bool {
        self.account.loading()
    }

    pub fn staged_change_views(&self) -> Vec<StagedChangeView> {
        self.staged_changes.views()
    }

    pub fn take_pending_staged_order_submit(&mut self) -> Option<StagedOrderSubmitRequest> {
        self.pending_staged_order_submit.take()
    }

    pub const fn effective_submit_mode(&self) -> SubmitMode {
        if self.live_writes_enabled {
            self.default_submit_mode
        } else {
            SubmitMode::DryRun
        }
    }

    pub fn order_ticket_preview(&self) -> OrderTicketPreview {
        self.order_ticket.preview(
            self.selected_symbol(),
            self.trading_profile.as_deref(),
            self.live_writes_enabled,
            self.effective_submit_mode(),
            self.selected_quote_price(),
        )
    }

    fn stage_order_ticket(&mut self) {
        let preview = self.order_ticket_preview();
        self.focus_panel(Panel::IntentReview);
        if !preview.ready {
            self.task_log.warning_event(format!(
                "order ticket is not ready: {}",
                preview.blockers.join("; ")
            ));
            return;
        }

        let Some(review) = order_ticket_review(&preview) else {
            self.task_log
                .warning_event("order ticket review snapshot could not be built".to_string());
            return;
        };
        let request = StagedChangeRequest {
            id: order_ticket_staged_change_id(&review),
            subject: StagedChangeSubject::OrderTicket(review),
        };
        let change_id = request.id.clone();
        match self
            .staged_changes
            .open_ready(request, self.effective_submit_mode())
        {
            OpenStagedChangeResult::Opened => {
                self.task_log
                    .info(format!("staged order ticket {change_id}"));
            }
            OpenStagedChangeResult::Rejected => {
                self.task_log.warning_event(
                    "order ticket cannot replace an active staged change".to_string(),
                );
            }
        }
    }

    pub fn reduce(&mut self, action: Action) {
        match action {
            Action::Focus(panel) => {
                self.focus_panel(panel);
            }
            Action::MoveCommandSelection(direction) => {
                self.command_palette.shift(direction);
            }
            Action::EditCommandQuery(request) => {
                self.command_palette.edit_query(request);
            }
            Action::MoveSymbolSearchSelection(direction) => {
                self.symbol_search.shift(direction);
            }
            Action::EditSymbolSearchQuery(request) => {
                self.symbol_search.edit_query(&self.watchlist, request);
            }
            Action::AcceptSymbolSearch => {
                if let Some(index) = self.symbol_search.selected_symbol_index() {
                    self.selected_symbol = index;
                    self.close_floating(FloatingKind::SymbolSearch);
                }
            }
            Action::MoveOrderTicketField(direction) => {
                self.order_ticket.move_field(direction);
            }
            Action::AdjustOrderTicketField(direction) => {
                self.order_ticket
                    .adjust_selected_field(direction, self.selected_quote_price());
            }
            Action::StageOrderTicket => self.stage_order_ticket(),
            Action::SubmitStagedChange => self.submit_next_staged_change(),
            Action::Execute(action) => self.execute(action),
            Action::CloseFocusedPanel => {
                self.panels.close_focused();
                self.clear_zoom();
                self.ensure_visible_focus();
            }
            Action::RestorePanels => {
                self.panels.restore();
                self.clear_zoom();
                self.ensure_visible_focus();
            }
            Action::ShiftWorkspace(direction) => {
                self.set_workspace(self.workspace.shift(direction))
            }
            Action::SetWorkspace(workspace) => self.set_workspace(workspace),
            Action::FocusPanelBy(direction) => self.focus_panel_by(direction),
            Action::ToggleFocusedZoom => {
                if !self.visible_panels().is_empty() {
                    self.zoomed = !self.zoomed;
                }
            }
            Action::CloseFocusedFloating => {
                if let Some(pane) = self.floating.pop() {
                    self.reset_floating_state(pane.kind);
                }
            }
            Action::FocusFloating(kind) => self.focus_floating(kind),
            Action::ResizeFloating { kind, size } => self.resize_floating(kind, size),
            Action::ResetLayout => {
                self.reset_open_floating_state();
                self.floating.clear();
                self.clear_zoom();
                self.layout = LayoutConfig::default();
                self.panels = DockedPanels::default();
                self.ensure_visible_focus();
            }
            Action::ResizeDockedColumns {
                left_ratio,
                main_ratio,
            } => {
                self.layout.left_ratio = left_ratio;
                self.layout.main_ratio = main_ratio;
                self.layout.normalize();
            }
            Action::RefreshStarted(generation) => self.refresh_started(generation),
            Action::SnapshotLoaded {
                generation,
                snapshot,
            } => self.snapshot_loaded(generation, snapshot),
            Action::RefreshFailed { generation, error } => self.refresh_failed(generation, error),
            Action::HistoryStarted { generation, symbol } => {
                self.history_started(generation, symbol);
            }
            Action::HistoryLoaded {
                generation,
                snapshot,
            } => self.history_loaded(generation, snapshot),
            Action::HistoryFailed {
                generation,
                symbol,
                error,
            } => self.history_failed(generation, symbol, error),
            Action::EvidenceStarted { generation, symbol } => {
                self.evidence_started(generation, symbol);
            }
            Action::EvidenceLoaded {
                generation,
                snapshot,
            } => self.evidence_loaded(generation, snapshot),
            Action::EvidenceFailed {
                generation,
                symbol,
                error,
            } => self.evidence_failed(generation, symbol, error),
            Action::ResearchStarted { generation, symbol } => {
                self.research_started(generation, symbol);
            }
            Action::ResearchLoaded {
                generation,
                snapshot,
            } => self.research_loaded(generation, snapshot),
            Action::AccountStarted {
                generation,
                profile,
            } => self.account_started(generation, profile),
            Action::AccountLoaded {
                generation,
                snapshot,
            } => self.account_loaded(generation, snapshot),
            Action::AccountFailed {
                generation,
                profile,
                error,
            } => self.account_failed(generation, profile, error),
            Action::SchedulerFailed(error) => self.scheduler_failed(error),
            Action::SetDefaultSubmitMode(mode) => {
                self.default_submit_mode = mode;
                self.task_log
                    .info(format!("default write mode set to {mode}"));
            }
            Action::SetLiveWritesEnabled(enabled) => {
                self.live_writes_enabled = enabled;
                self.close_floating(FloatingKind::LiveWritesConfirmation);
                self.task_log.info(if enabled {
                    "live writes enabled for this TUI session".to_string()
                } else {
                    "live writes disabled for this TUI session".to_string()
                });
                if !enabled {
                    let abandoned = self.staged_changes.disable_live();
                    if abandoned > 0 {
                        self.task_log.warning_event(format!(
                            "abandoned {abandoned} pending live staged change(s)"
                        ));
                    }
                }
            }
            Action::OpenStagedChange(request) => {
                match self
                    .staged_changes
                    .open(request, self.effective_submit_mode())
                {
                    OpenStagedChangeResult::Opened => {}
                    OpenStagedChangeResult::Rejected => self
                        .task_log
                        .warning_event("staged change cannot replace an active change".to_string()),
                }
            }
            Action::ApplyStagedChangeEvent { id, event } => {
                match self.staged_changes.apply(&id, event) {
                    TransitionResult::Applied => {}
                    TransitionResult::Missing => self
                        .task_log
                        .warning_event(format!("staged change {id} is no longer present")),
                    TransitionResult::Rejected { current, event } => {
                        self.task_log.warning_event(format!(
                            "staged change {id} cannot apply {event:?} from {current}"
                        ));
                    }
                }
            }
            Action::CloseStagedChange(id) => match self.staged_changes.close(&id) {
                CloseStagedChangeResult::Closed => {}
                CloseStagedChangeResult::Missing => self
                    .task_log
                    .warning_event(format!("staged change {id} is no longer present")),
                CloseStagedChangeResult::Rejected { current } => self
                    .task_log
                    .warning_event(format!("staged change {id} cannot close while {current}")),
            },
            Action::Log(message) => self.task_log.info(message),
        }
    }

    fn submit_next_staged_change(&mut self) {
        match self.staged_changes.queue_next_order_submit() {
            QueueOrderSubmitResult::Queued(request) => {
                self.task_log.info(format!(
                    "submitting staged {} change {} as {}",
                    request.review.symbol, request.id, request.mode
                ));
                self.pending_staged_order_submit = Some(request);
            }
            QueueOrderSubmitResult::Missing => self
                .task_log
                .warning_event("no ready staged order change to submit".to_string()),
            QueueOrderSubmitResult::Rejected { current } => self.task_log.warning_event(format!(
                "staged change cannot submit from current state {current}"
            )),
        }
    }
}

fn order_ticket_review(preview: &OrderTicketPreview) -> Option<OrderTicketReview> {
    Some(OrderTicketReview {
        symbol: preview.symbol.clone()?,
        profile: preview.profile.clone()?,
        market: preview.market,
        side: preview.side,
        kind: preview.kind,
        quantity: preview.quantity.clone()?,
        price: preview.price.clone(),
        time_in_force: preview.time_in_force,
        reduce_only: preview.reduce_only,
        parsed_quantity: preview.parsed_quantity.clone()?,
        order_spec: preview.order_spec.clone()?,
        effective_mode: preview.effective_mode,
    })
}

fn order_ticket_staged_change_id(review: &OrderTicketReview) -> String {
    let mut parts = vec![
        "order-ticket".to_string(),
        review.profile.clone(),
        review.effective_mode.to_string(),
        review.market.to_string(),
        review.side.to_string(),
        review.kind.to_string(),
        review.time_in_force.to_string(),
        review.reduce_only.to_string(),
        review.symbol.clone(),
        review.quantity.clone(),
    ];
    parts.extend(review.price.clone());
    sanitize_staged_change_id(&parts.join("-"))
}

fn sanitize_staged_change_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .fold(String::new(), |mut normalized, character| {
            if character != '-' || !normalized.ends_with('-') {
                normalized.push(character);
            }
            normalized
        })
        .trim_matches('-')
        .to_string()
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Focus(Panel),
    MoveCommandSelection(isize),
    EditCommandQuery(tui_input::InputRequest),
    MoveSymbolSearchSelection(isize),
    EditSymbolSearchQuery(tui_input::InputRequest),
    AcceptSymbolSearch,
    MoveOrderTicketField(isize),
    AdjustOrderTicketField(isize),
    StageOrderTicket,
    SubmitStagedChange,
    Execute(ActionId),
    FocusPanelBy(isize),
    ToggleFocusedZoom,
    CloseFocusedPanel,
    RestorePanels,
    ShiftWorkspace(isize),
    SetWorkspace(WorkspaceKind),
    CloseFocusedFloating,
    FocusFloating(FloatingKind),
    ResizeFloating {
        kind: FloatingKind,
        size: FloatingSize,
    },
    ResetLayout,
    ResizeDockedColumns {
        left_ratio: u16,
        main_ratio: u16,
    },
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
    EvidenceStarted {
        generation: u64,
        symbol: String,
    },
    EvidenceLoaded {
        generation: u64,
        snapshot: CryptoQuoteEvidenceSnapshot,
    },
    EvidenceFailed {
        generation: u64,
        symbol: String,
        error: String,
    },
    ResearchStarted {
        generation: u64,
        symbol: String,
    },
    ResearchLoaded {
        generation: u64,
        snapshot: ResearchContextSnapshot,
    },
    AccountStarted {
        generation: u64,
        profile: String,
    },
    AccountLoaded {
        generation: u64,
        snapshot: AccountSnapshot,
    },
    AccountFailed {
        generation: u64,
        profile: String,
        error: String,
    },
    SchedulerFailed(String),
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "submit mode changes are reserved for a confirmed write-mode selector"
        )
    )]
    SetDefaultSubmitMode(SubmitMode),
    SetLiveWritesEnabled(bool),
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "staged change actions are part of the state contract before staged change panels bind them"
        )
    )]
    OpenStagedChange(StagedChangeRequest),
    ApplyStagedChangeEvent {
        id: String,
        event: StagedChangeEvent,
    },
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "staged change actions are part of the state contract before staged change panels bind them"
        )
    )]
    CloseStagedChange(String),
    Log(String),
}

#[cfg(test)]
mod tests;
