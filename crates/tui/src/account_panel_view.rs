use ratatui::style::Modifier;
use ratatui::text::{Line, Span};

use crate::account::ACCOUNT_READ_PLAN;
use crate::model::Panel;
use crate::mouse_target::MouseTarget;
use crate::open_order_view::OpenOrderRow;
use crate::profile_snapshot::TradingProfileSnapshot;
use crate::state::AppState;

use crate::render::open_orders::open_order_line;
use crate::render::profile_policy::{ProfilePolicyFormat, profile_policy_lines};
use crate::render::widgets::compact_text;

const VISIBLE_TRANSFER_LIMIT: usize = 4;

pub(crate) struct AccountPanelRow {
    pub line: Line<'static>,
    pub open_order_index: Option<usize>,
}

impl AccountPanelRow {
    fn text(text: impl Into<String>) -> Self {
        Self {
            line: Line::from(text.into()),
            open_order_index: None,
        }
    }

    fn line(line: Line<'static>) -> Self {
        Self {
            line,
            open_order_index: None,
        }
    }

    fn open_order(line: Line<'static>, index: usize) -> Self {
        Self {
            line,
            open_order_index: Some(index),
        }
    }
}

pub(crate) fn rows(state: &AppState, mouse_target: Option<MouseTarget>) -> Vec<AccountPanelRow> {
    let mut rows = profile_rows(state);

    match state.account_snapshot.as_ref() {
        Some(snapshot) => {
            rows.extend(profile_risk_rows(state, &snapshot.profile_config));
            rows.extend(account_read_rows(snapshot));
            rows.extend(open_order_rows(state, snapshot, mouse_target));
            rows.extend(transfer_history_rows(state, snapshot));
            rows.extend(warning_rows(state, snapshot));
        }
        None if state.trading_profile.is_some() => rows.push(AccountPanelRow::text(
            "No account snapshot loaded yet. Waiting for signed read.",
        )),
        None => rows.push(AccountPanelRow::text(
            "Start the TUI with --profile <name> to enable signed account reads.",
        )),
    }

    rows
}

pub(crate) fn open_order_index_at_content_row(
    state: &AppState,
    content_row: usize,
) -> Option<usize> {
    rows(state, None).get(content_row)?.open_order_index
}

fn profile_rows(state: &AppState) -> Vec<AccountPanelRow> {
    if let Some(profile) = state.trading_profile.as_deref() {
        vec![AccountPanelRow::line(Line::from(vec![
            Span::styled(
                profile.to_string(),
                state.theme.accent_style().add_modifier(Modifier::BOLD),
            ),
            Span::raw(if state.account_loading() {
                " account loading..."
            } else {
                " account"
            }),
        ]))]
    } else {
        vec![AccountPanelRow::text("No trading profile selected.")]
    }
}

fn profile_risk_rows(state: &AppState, profile: &TradingProfileSnapshot) -> Vec<AccountPanelRow> {
    profile_policy_lines(&state.theme, profile, ProfilePolicyFormat::Account)
        .into_iter()
        .map(AccountPanelRow::line)
        .collect()
}

fn account_read_rows(snapshot: &crate::AccountSnapshot) -> Vec<AccountPanelRow> {
    let mut rows = vec![
        AccountPanelRow::text(""),
        AccountPanelRow::text(format!(
            "provider: {}  environment: {}",
            snapshot.provider, snapshot.environment
        )),
        AccountPanelRow::text(format!(
            "signed reads: {} ok / {} warning",
            snapshot.reads.len(),
            snapshot.errors.len()
        )),
    ];
    rows.extend(ACCOUNT_READ_PLAN.into_iter().map(|plan| {
        let request = plan.request();
        let label = if snapshot.read_request(&request).is_some() {
            "ok"
        } else {
            "missing"
        };
        AccountPanelRow::text(format!("{}: {label}", plan.label()))
    }));
    rows
}

fn open_order_rows(
    state: &AppState,
    snapshot: &crate::AccountSnapshot,
    mouse_target: Option<MouseTarget>,
) -> Vec<AccountPanelRow> {
    let open_orders = snapshot.open_orders();
    let selected = state
        .selected_open_order
        .min(open_orders.len().saturating_sub(1));
    crate::open_order_view::open_order_rows(&open_orders, selected)
        .into_iter()
        .map(|row| match row {
            OpenOrderRow::Order { index, order } => AccountPanelRow::open_order(
                open_order_line(state, Panel::Account, index, order, mouse_target),
                index,
            ),
            row => AccountPanelRow::line(non_order_open_order_line(state, row)),
        })
        .collect()
}

fn non_order_open_order_line(state: &AppState, row: OpenOrderRow<'_>) -> Line<'static> {
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
        OpenOrderRow::More { hidden } => Line::from(Span::styled(
            format!("+{hidden} more open orders"),
            state.theme.warning_style(),
        )),
        OpenOrderRow::Order { .. } => unreachable!("orders are rendered with action metadata"),
    }
}

fn transfer_history_rows(
    state: &AppState,
    snapshot: &crate::AccountSnapshot,
) -> Vec<AccountPanelRow> {
    let transfers = snapshot.transfer_history();
    if transfers.is_empty() {
        return Vec::new();
    }

    let mut rows = vec![
        AccountPanelRow::text(""),
        AccountPanelRow::line(Line::from(Span::styled(
            format!("transfer history ({})", transfers.len()),
            state.theme.accent_style().add_modifier(Modifier::BOLD),
        ))),
    ];
    rows.extend(
        transfers
            .iter()
            .take(VISIBLE_TRANSFER_LIMIT)
            .map(|transfer| {
                AccountPanelRow::text(format!(
                    "{} {} {} {} [{}]",
                    transfer.direction,
                    transfer.amount.as_deref().unwrap_or("-"),
                    transfer.asset.as_deref().unwrap_or("-"),
                    transfer.status.as_deref().unwrap_or("-"),
                    transfer.identifier()
                ))
            }),
    );
    if transfers.len() > VISIBLE_TRANSFER_LIMIT {
        rows.push(AccountPanelRow::line(Line::from(Span::styled(
            format!(
                "+{} more transfers",
                transfers.len() - VISIBLE_TRANSFER_LIMIT
            ),
            state.theme.warning_style(),
        ))));
    }
    rows
}

fn warning_rows(state: &AppState, snapshot: &crate::AccountSnapshot) -> Vec<AccountPanelRow> {
    snapshot
        .errors
        .iter()
        .take(2)
        .map(|error| {
            AccountPanelRow::line(Line::from(Span::styled(
                format!(
                    "{} warning: {}",
                    error.label,
                    compact_text(&error.error, 96)
                ),
                state.theme.warning_style(),
            )))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_finance_core::{
        Environment, Market, Provider, SignedReadRequest, SignedReadSnapshot,
    };

    #[test]
    fn rows_mark_rendered_open_orders_as_clickable_metadata() {
        let mut state = AppState::from_config(crate::config::TuiConfig {
            trading: crate::config::TradingConfig {
                default_profile: Some("mainnet".to_string()),
            },
            ..crate::config::TuiConfig::default()
        });
        state.account_snapshot = Some(account_snapshot_with_open_orders("mainnet"));

        let clickable = rows(&state, None)
            .into_iter()
            .filter_map(|row| row.open_order_index)
            .collect::<Vec<_>>();

        assert_eq!(clickable, vec![0, 1]);
    }

    fn account_snapshot_with_open_orders(profile: &str) -> crate::AccountSnapshot {
        crate::AccountSnapshot::new(
            profile.to_string(),
            Provider::Binance,
            Environment::Live,
            crate::profile_snapshot::test_trading_profile_snapshot(),
            vec![SignedReadSnapshot::new(
                profile.to_string(),
                Provider::Binance,
                Environment::Live,
                SignedReadRequest::OpenOrders {
                    market: Market::Spot,
                    symbol: None,
                },
                serde_json::json!([
                    {
                        "symbol": "BTCUSDT",
                        "orderId": 1001,
                        "clientOrderId": "spot-order",
                        "side": "BUY",
                        "type": "LIMIT",
                        "origQty": "0.10",
                        "executedQty": "0",
                        "price": "64000"
                    },
                    {
                        "symbol": "ETHUSDT",
                        "orderId": 1002,
                        "clientOrderId": "eth-order",
                        "side": "SELL",
                        "type": "LIMIT",
                        "origQty": "0.20",
                        "executedQty": "0.05",
                        "price": "3200"
                    }
                ]),
            )],
            Vec::new(),
        )
    }
}
