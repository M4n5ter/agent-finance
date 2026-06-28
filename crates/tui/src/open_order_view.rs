use std::ops::Range;

use crate::account::OpenOrderSummary;

pub(crate) const VISIBLE_OPEN_ORDER_LIMIT: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum OpenOrderRow<'a> {
    Spacer,
    Header {
        total: usize,
    },
    Earlier {
        hidden: usize,
    },
    Order {
        index: usize,
        order: &'a OpenOrderSummary,
    },
    More {
        hidden: usize,
    },
}

pub(crate) fn visible_open_order_window(len: usize, selected: usize) -> Range<usize> {
    if len == 0 {
        return 0..0;
    }
    let selected = selected.min(len - 1);
    let start = selected
        .saturating_add(1)
        .saturating_sub(VISIBLE_OPEN_ORDER_LIMIT);
    start..(start + VISIBLE_OPEN_ORDER_LIMIT).min(len)
}

pub(crate) fn open_order_rows(
    open_orders: &[OpenOrderSummary],
    selected: usize,
) -> Vec<OpenOrderRow<'_>> {
    if open_orders.is_empty() {
        return Vec::new();
    }

    let visible = visible_open_order_window(open_orders.len(), selected);
    let mut rows = vec![
        OpenOrderRow::Spacer,
        OpenOrderRow::Header {
            total: open_orders.len(),
        },
    ];
    if visible.start > 0 {
        rows.push(OpenOrderRow::Earlier {
            hidden: visible.start,
        });
    }
    rows.extend(
        open_orders
            .iter()
            .enumerate()
            .skip(visible.start)
            .take(visible.len())
            .map(|(index, order)| OpenOrderRow::Order { index, order }),
    );
    let hidden_after = open_orders.len().saturating_sub(visible.end);
    if hidden_after > 0 {
        rows.push(OpenOrderRow::More {
            hidden: hidden_after,
        });
    }
    rows
}

pub(crate) fn open_order_index_at_content_row(
    open_orders: &[OpenOrderSummary],
    selected: usize,
    content_row: usize,
) -> Option<usize> {
    match open_order_rows(open_orders, selected).get(content_row)? {
        OpenOrderRow::Order { index, .. } => Some(*index),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_finance_core::Market;

    fn order(symbol: &str) -> OpenOrderSummary {
        OpenOrderSummary {
            market: Market::Spot,
            symbol: symbol.to_string(),
            order_id: None,
            client_order_id: None,
            side: None,
            order_type: None,
            original_quantity: None,
            executed_quantity: None,
            remaining_quantity: None,
            price: None,
        }
    }

    #[test]
    fn visible_window_keeps_selected_open_order_in_view() {
        assert_eq!(visible_open_order_window(0, 0), 0..0);
        assert_eq!(visible_open_order_window(2, 0), 0..2);
        assert_eq!(visible_open_order_window(8, 0), 0..4);
        assert_eq!(visible_open_order_window(8, 4), 1..5);
        assert_eq!(visible_open_order_window(8, 7), 4..8);
    }

    #[test]
    fn rows_mark_only_visible_orders_as_selectable() {
        let open_orders = ["BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT", "ADAUSDT"]
            .into_iter()
            .map(order)
            .collect::<Vec<_>>();

        let selected = 4;
        let rows = open_order_rows(&open_orders, selected);
        let selectable = rows
            .iter()
            .enumerate()
            .filter_map(|(row, item)| match item {
                OpenOrderRow::Order { index, order } => Some((row, *index, order.symbol.as_str())),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            selectable,
            vec![
                (3, 1, "ETHUSDT"),
                (4, 2, "SOLUSDT"),
                (5, 3, "BNBUSDT"),
                (6, 4, "ADAUSDT"),
            ]
        );
        assert_eq!(
            open_order_index_at_content_row(&open_orders, selected, 2),
            None
        );
        assert_eq!(
            open_order_index_at_content_row(&open_orders, selected, 3),
            Some(1)
        );
    }
}
