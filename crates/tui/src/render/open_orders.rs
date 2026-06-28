use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem};

use crate::model::Panel;
use crate::open_order_view::OpenOrderRow;
use crate::state::AppState;

use super::widgets::panel_block;

pub(super) fn render_open_orders(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let mut lines = Vec::new();
    match state.account_snapshot.as_ref() {
        Some(snapshot) => {
            lines.extend(open_order_lines(state, snapshot));
            if snapshot.open_orders().is_empty() {
                lines.push(Line::from("No open orders."));
            } else {
                lines.push(Line::from(
                    crate::open_order_controls::open_order_section_hint(),
                ));
            }
        }
        None if state.trading_profile.is_some() => lines.push(Line::from(
            "No account snapshot loaded yet. Waiting for signed open order reads.",
        )),
        None => lines.push(Line::from(
            "Start the TUI with --profile <name> to load open orders.",
        )),
    }

    let items = lines.into_iter().map(ListItem::new);
    frame.render_widget(
        List::new(items).block(panel_block(Panel::OpenOrders, state)),
        area,
    );
}

pub(super) fn open_order_lines(
    state: &AppState,
    snapshot: &crate::AccountSnapshot,
) -> Vec<Line<'static>> {
    let open_orders = snapshot.open_orders();
    if open_orders.is_empty() {
        return Vec::new();
    }

    let selected = state
        .selected_open_order
        .min(open_orders.len().saturating_sub(1));
    crate::open_order_view::open_order_rows(&open_orders, selected)
        .into_iter()
        .map(|row| open_order_line(state, row))
        .collect()
}

fn open_order_line(state: &AppState, row: OpenOrderRow<'_>) -> Line<'static> {
    match row {
        OpenOrderRow::Spacer => Line::from(""),
        OpenOrderRow::Header { total } => Line::from(Span::styled(
            format!("open orders ({total})"),
            state.theme.accent_style().add_modifier(Modifier::BOLD),
        )),
        OpenOrderRow::Earlier { hidden } => Line::from(Span::styled(
            format!("+{hidden} earlier open orders"),
            state.theme.warning_style(),
        )),
        OpenOrderRow::Order { index, order } => {
            let marker = if index == state.selected_open_order {
                ">"
            } else {
                " "
            };
            Line::from(format!(
                "{marker} {} {} {} {} @ {} [{}]",
                order.market,
                order.side.as_deref().unwrap_or("-"),
                order.remaining_quantity.as_deref().unwrap_or("-"),
                order.symbol,
                order.price.as_deref().unwrap_or("-"),
                order.identifier()
            ))
        }
        OpenOrderRow::More { hidden } => Line::from(Span::styled(
            format!("+{hidden} more open orders"),
            state.theme.warning_style(),
        )),
    }
}
