use ratatui::layout::Rect;

use crate::futures_state_ticket::FuturesStateTicketField;
use crate::model::Panel;
use crate::mouse_target::{MouseTarget, PanelMouseAction};
use crate::order_ticket::OrderTicketField;
use crate::state::{Action, AppState};
use crate::ticket_panel_view::{TicketPanelClick, TicketPanelRows};
use crate::transfer_ticket::TransferTicketField;

pub(crate) fn click_action(
    state: &AppState,
    panel: Panel,
    area: Rect,
    _column: u16,
    row: u16,
) -> Option<Action> {
    panel_hit_at(state, panel, area, row).and_then(|hit| hit.action_for(panel))
}

pub(crate) fn hover_target(
    state: &AppState,
    panel: Panel,
    area: Rect,
    _column: u16,
    row: u16,
) -> Option<MouseTarget> {
    panel_hit_at(state, panel, area, row)
        .map(|hit| MouseTarget::PanelAction {
            panel,
            action: hit.mouse_action(),
        })
        .or(Some(MouseTarget::Panel(panel)))
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PanelHit {
    Row(usize),
    TicketField(usize),
    TicketReadyAction,
}

impl PanelHit {
    fn action_for(self, panel: Panel) -> Option<Action> {
        match (panel, self) {
            (Panel::Watchlist, Self::Row(index)) => Some(Action::SelectWatchlistSymbol(index)),
            (Panel::OpenOrders, Self::Row(index)) => Some(Action::SelectOpenOrder(index)),
            (Panel::IntentReview, Self::Row(index)) => Some(Action::SelectStagedChange(index)),
            (Panel::OrderTicket, Self::TicketField(index)) => {
                Some(Action::SelectOrderTicketField(index))
            }
            (Panel::OrderTicket, Self::TicketReadyAction) => Some(Action::StageOrderTicket),
            (Panel::TransferTicket, Self::TicketField(index)) => {
                Some(Action::SelectTransferTicketField(index))
            }
            (Panel::TransferTicket, Self::TicketReadyAction) => Some(Action::StageTransferTicket),
            (Panel::FuturesState, Self::TicketField(index)) => {
                Some(Action::SelectFuturesStateTicketField(index))
            }
            (Panel::FuturesState, Self::TicketReadyAction) => Some(Action::StageFuturesStateTicket),
            _ => None,
        }
    }

    const fn mouse_action(self) -> PanelMouseAction {
        match self {
            Self::Row(index) => PanelMouseAction::SelectRow { index },
            Self::TicketField(index) => PanelMouseAction::SelectField { index },
            Self::TicketReadyAction => PanelMouseAction::StageReadyChange,
        }
    }
}

fn panel_hit_at(state: &AppState, panel: Panel, area: Rect, row: u16) -> Option<PanelHit> {
    match panel {
        Panel::Watchlist => {
            let index = content_row(area, row)?;
            (index < state.watchlist.len()).then_some(PanelHit::Row(index))
        }
        Panel::OpenOrders => {
            let open_orders = state.account_snapshot.as_ref()?.open_orders();
            crate::open_order_view::open_order_index_at_content_row(
                &open_orders,
                state.selected_open_order,
                content_row(area, row)?,
            )
            .map(PanelHit::Row)
        }
        Panel::IntentReview => crate::intent_review_view::staged_change_index_at_content_row(
            state.staged_change_review_views().len(),
            content_row(area, row)?,
        )
        .map(PanelHit::Row),
        Panel::OrderTicket => ticket_hit_at(content_row(area, row)?, order_ticket_rows(state)),
        Panel::TransferTicket => {
            ticket_hit_at(content_row(area, row)?, transfer_ticket_rows(state))
        }
        Panel::FuturesState => {
            ticket_hit_at(content_row(area, row)?, futures_state_ticket_rows(state))
        }
        Panel::Account
        | Panel::Settings
        | Panel::ProfileRisk
        | Panel::Quote
        | Panel::History
        | Panel::Evidence
        | Panel::Polymarket
        | Panel::Research
        | Panel::RiskAudit
        | Panel::ProviderHealth
        | Panel::TaskLog => None,
    }
}

fn ticket_hit_at(content_row: usize, rows: TicketPanelRows) -> Option<PanelHit> {
    match rows.click_at(content_row)? {
        TicketPanelClick::Field(index) => Some(PanelHit::TicketField(index)),
        TicketPanelClick::ReadyAction => Some(PanelHit::TicketReadyAction),
    }
}

fn order_ticket_rows(state: &AppState) -> TicketPanelRows {
    let preview = state.order_ticket_preview();
    TicketPanelRows {
        detail_count: 1,
        field_count: OrderTicketField::COUNT,
        ready: preview.ready,
        blocker_count: preview.blockers.len(),
    }
}

fn transfer_ticket_rows(state: &AppState) -> TicketPanelRows {
    let preview = state.transfer_ticket_preview();
    TicketPanelRows {
        detail_count: 0,
        field_count: TransferTicketField::COUNT,
        ready: preview.ready,
        blocker_count: preview.blockers.len(),
    }
}

fn futures_state_ticket_rows(state: &AppState) -> TicketPanelRows {
    let preview = state.futures_state_ticket_preview();
    TicketPanelRows {
        detail_count: 0,
        field_count: FuturesStateTicketField::MAX_COUNT,
        ready: preview.ready,
        blocker_count: preview.blockers.len(),
    }
}

fn content_row(area: Rect, row: u16) -> Option<usize> {
    if row <= area.y || row >= area.bottom().saturating_sub(1) {
        return None;
    }
    Some(row.saturating_sub(area.y).saturating_sub(1) as usize)
}
