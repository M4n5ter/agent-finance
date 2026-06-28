use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::model::Panel;
use crate::state::AppState;

use super::widgets::panel_block;

const VISIBLE_OPEN_ORDER_LIMIT: usize = 4;

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

    frame.render_widget(
        Paragraph::new(lines)
            .block(panel_block(Panel::OpenOrders, state))
            .wrap(Wrap { trim: true }),
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

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("open orders ({})", open_orders.len()),
            state.theme.accent_style().add_modifier(Modifier::BOLD),
        )),
    ];
    let selected = state
        .selected_open_order
        .min(open_orders.len().saturating_sub(1));
    let start = selected
        .saturating_add(1)
        .saturating_sub(VISIBLE_OPEN_ORDER_LIMIT);
    if start > 0 {
        lines.push(Line::from(Span::styled(
            format!("+{start} earlier open orders"),
            state.theme.warning_style(),
        )));
    }
    for (index, order) in open_orders
        .iter()
        .enumerate()
        .skip(start)
        .take(VISIBLE_OPEN_ORDER_LIMIT)
    {
        let marker = if index == state.selected_open_order {
            ">"
        } else {
            " "
        };
        lines.push(Line::from(format!(
            "{marker} {} {} {} {} @ {} [{}]",
            order.market,
            order.side.as_deref().unwrap_or("-"),
            order.remaining_quantity.as_deref().unwrap_or("-"),
            order.symbol,
            order.price.as_deref().unwrap_or("-"),
            order.identifier()
        )));
    }
    let hidden_after = open_orders
        .len()
        .saturating_sub(start.saturating_add(VISIBLE_OPEN_ORDER_LIMIT));
    if hidden_after > 0 {
        lines.push(Line::from(Span::styled(
            format!("+{hidden_after} more open orders"),
            state.theme.warning_style(),
        )));
    }
    lines
}
