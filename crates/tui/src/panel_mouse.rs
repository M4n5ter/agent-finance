use ratatui::layout::Rect;

use crate::model::Panel;
use crate::state::{Action, AppState};

pub(crate) fn click_action(
    state: &AppState,
    panel: Panel,
    area: Rect,
    _column: u16,
    row: u16,
) -> Option<Action> {
    match panel {
        Panel::Watchlist => watchlist_click_action(state, area, row),
        Panel::OpenOrders => open_order_click_action(state, area, row),
        Panel::IntentReview => staged_change_click_action(state, area, row),
        Panel::Account
        | Panel::OrderTicket
        | Panel::TransferTicket
        | Panel::FuturesState
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

fn watchlist_click_action(state: &AppState, area: Rect, row: u16) -> Option<Action> {
    let index = content_row(area, row)?;
    (index < state.watchlist.len()).then_some(Action::SelectWatchlistSymbol(index))
}

fn open_order_click_action(state: &AppState, area: Rect, row: u16) -> Option<Action> {
    let open_orders = state.account_snapshot.as_ref()?.open_orders();
    let index = crate::open_order_view::open_order_index_at_content_row(
        &open_orders,
        state.selected_open_order,
        content_row(area, row)?,
    )?;
    Some(Action::SelectOpenOrder(index))
}

fn staged_change_click_action(state: &AppState, area: Rect, row: u16) -> Option<Action> {
    let visible_len = state.staged_change_review_views().len();
    let index = crate::intent_review_view::staged_change_index_at_content_row(
        visible_len,
        content_row(area, row)?,
    )?;
    Some(Action::SelectStagedChange(index))
}

fn content_row(area: Rect, row: u16) -> Option<usize> {
    if row <= area.y || row >= area.bottom().saturating_sub(1) {
        return None;
    }
    Some(row.saturating_sub(area.y).saturating_sub(1) as usize)
}
