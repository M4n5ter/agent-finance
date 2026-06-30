use std::str::FromStr;

use rust_decimal::Decimal;

use crate::state::AppState;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChartOverlayLine {
    pub price: f64,
    pub label: String,
    pub kind: ChartOverlayKind,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ChartOverlayKind {
    Current,
    PreviousClose,
    DayOpen,
    DayHigh,
    DayLow,
    BuyOrder,
    SellOrder,
    LongPosition,
    ShortPosition,
}

pub(crate) fn lines_for_state(state: &AppState, symbol: &str) -> Vec<ChartOverlayLine> {
    let mut lines = Vec::new();
    append_quote_lines(&mut lines, state, symbol);
    append_account_lines(&mut lines, state, symbol);
    lines
}

fn append_quote_lines(lines: &mut Vec<ChartOverlayLine>, state: &AppState, symbol: &str) {
    let Some(quote) = state
        .market_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.quote_for(symbol))
    else {
        return;
    };

    push_price(lines, quote.price, "cur", ChartOverlayKind::Current);
    push_price(
        lines,
        quote.regular_basis.previous_close,
        "prev",
        ChartOverlayKind::PreviousClose,
    );
    push_price(
        lines,
        quote.regular_basis.open,
        "day open",
        ChartOverlayKind::DayOpen,
    );
    push_price(
        lines,
        quote.regular_basis.high,
        "day high",
        ChartOverlayKind::DayHigh,
    );
    push_price(
        lines,
        quote.regular_basis.low,
        "day low",
        ChartOverlayKind::DayLow,
    );
}

fn append_account_lines(lines: &mut Vec<ChartOverlayLine>, state: &AppState, symbol: &str) {
    let Some(snapshot) = state.account_snapshot.as_ref() else {
        return;
    };

    for order in snapshot
        .open_orders()
        .into_iter()
        .filter(|order| symbols_match(&order.symbol, symbol))
    {
        let Some(price) = parse_price(order.price.as_deref()) else {
            continue;
        };
        let side = order.side.as_deref().unwrap_or("order");
        let kind = if side.eq_ignore_ascii_case("SELL") {
            ChartOverlayKind::SellOrder
        } else {
            ChartOverlayKind::BuyOrder
        };
        let label = format!("{} order", side.to_ascii_lowercase());
        push_parsed_price(lines, price, label, kind);
    }

    for position in snapshot
        .holdings()
        .futures_positions
        .into_iter()
        .filter(|position| symbols_match(&position.symbol, symbol))
    {
        let Some(price) = parse_price(position.entry_price.as_deref()) else {
            continue;
        };
        let amount = Decimal::from_str(&position.position_amount).unwrap_or(Decimal::ZERO);
        let side = position.position_side.as_deref().unwrap_or_else(|| {
            if amount.is_sign_negative() {
                "SHORT"
            } else {
                "LONG"
            }
        });
        let kind = if side.eq_ignore_ascii_case("SHORT") || amount.is_sign_negative() {
            ChartOverlayKind::ShortPosition
        } else {
            ChartOverlayKind::LongPosition
        };
        let label = format!("{} pos", side.to_ascii_lowercase());
        push_parsed_price(lines, price, label, kind);
    }
}

fn push_price(
    lines: &mut Vec<ChartOverlayLine>,
    price: Option<f64>,
    label: &str,
    kind: ChartOverlayKind,
) {
    if let Some(price) = price.filter(|price| price.is_finite() && *price > 0.0) {
        push_parsed_price(lines, price, label.to_string(), kind);
    }
}

fn push_parsed_price(
    lines: &mut Vec<ChartOverlayLine>,
    price: f64,
    label: String,
    kind: ChartOverlayKind,
) {
    if price.is_finite() && price > 0.0 {
        lines.push(ChartOverlayLine { price, label, kind });
    }
}

fn parse_price(value: Option<&str>) -> Option<f64> {
    value
        .and_then(|value| f64::from_str(value).ok())
        .filter(|price| price.is_finite() && *price > 0.0)
}

fn symbols_match(left: &str, right: &str) -> bool {
    normalize_symbol(left) == normalize_symbol(right)
}

fn normalize_symbol(symbol: &str) -> String {
    symbol
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use agent_finance_core::{
        Environment, Market, Provider, SignedReadRequest, SignedReadSnapshot,
    };
    use agent_finance_market::snapshot::{MarketSnapshot, QuoteSnapshot, RegularBasisSnapshot};
    use serde_json::json;

    use super::*;

    #[test]
    fn chart_overlays_collect_quote_orders_and_position_entry_prices() {
        let mut state = AppState::from_config(crate::config::TuiConfig {
            watchlist: vec!["BTC/USDT".to_string()],
            ..crate::config::TuiConfig::default()
        });
        state.market_snapshot = Some(MarketSnapshot {
            fetched_at_local: None,
            quotes: vec![QuoteSnapshot {
                symbol: "BTCUSDT".to_string(),
                price: Some(65_000.0),
                currency: Some("USD".to_string()),
                provider: "test".to_string(),
                session: Some("regular".to_string()),
                market_time_local: None,
                change_pct: None,
                aliases: vec!["BTC/USDT".to_string()],
                regular_basis: RegularBasisSnapshot {
                    previous_close: Some(64_000.0),
                    open: Some(64_500.0),
                    high: Some(66_000.0),
                    low: Some(63_500.0),
                    volume: None,
                },
            }],
            errors: Vec::new(),
        });
        state.account_snapshot = Some(crate::AccountSnapshot::new(
            "mainnet".to_string(),
            Provider::Binance,
            Environment::Live,
            crate::profile_snapshot::test_trading_profile_snapshot(),
            vec![
                SignedReadSnapshot::new(
                    "mainnet",
                    Provider::Binance,
                    Environment::Live,
                    SignedReadRequest::OpenOrders {
                        market: Market::Spot,
                        symbol: None,
                    },
                    json!([
                        {
                            "symbol": "BTCUSDT",
                            "side": "BUY",
                            "type": "LIMIT",
                            "origQty": "0.10",
                            "executedQty": "0",
                            "price": "63000"
                        }
                    ]),
                ),
                SignedReadSnapshot::new(
                    "mainnet",
                    Provider::Binance,
                    Environment::Live,
                    SignedReadRequest::UsdsFuturesPositions,
                    json!({
                        "assets": [],
                        "positions": [
                            {
                                "symbol": "BTCUSDT",
                                "positionSide": "LONG",
                                "positionAmt": "0.002",
                                "entryPrice": "62000"
                            }
                        ]
                    }),
                ),
            ],
            Vec::new(),
        ));

        let lines = lines_for_state(&state, "BTC/USDT");

        assert!(
            lines
                .iter()
                .any(|line| line.kind == ChartOverlayKind::Current)
        );
        assert!(
            lines
                .iter()
                .any(|line| line.kind == ChartOverlayKind::BuyOrder && line.price == 63_000.0)
        );
        assert!(
            lines.iter().any(|line| {
                line.kind == ChartOverlayKind::LongPosition && line.price == 62_000.0
            })
        );
    }

    #[test]
    fn chart_overlays_do_not_mix_different_symbols() {
        let mut state = AppState::from_config(crate::config::TuiConfig {
            watchlist: vec!["BTCUSDT".to_string()],
            ..crate::config::TuiConfig::default()
        });
        state.account_snapshot = Some(crate::AccountSnapshot::new(
            "mainnet".to_string(),
            Provider::Binance,
            Environment::Live,
            crate::profile_snapshot::test_trading_profile_snapshot(),
            vec![SignedReadSnapshot::new(
                "mainnet",
                Provider::Binance,
                Environment::Live,
                SignedReadRequest::OpenOrders {
                    market: Market::Spot,
                    symbol: None,
                },
                json!([
                    {
                        "symbol": "ETHUSDT",
                        "side": "SELL",
                        "type": "LIMIT",
                        "origQty": "1",
                        "executedQty": "0",
                        "price": "3200"
                    }
                ]),
            )],
            Vec::new(),
        ));

        let lines = lines_for_state(&state, "BTCUSDT");

        assert!(lines.is_empty());
    }
}
