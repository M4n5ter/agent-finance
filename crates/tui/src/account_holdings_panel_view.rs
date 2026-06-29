use agent_finance_core::TransferDirection;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};

use crate::account_panel_view::{
    AccountPanelHit, AccountPanelRow, AccountTicketPreset, push_hidden_row,
};
use crate::futures_state_ticket::FuturesStateTicketPreset;
use crate::mouse_target::MouseTarget;
use crate::state::AppState;
use crate::transfer_ticket::TransferTicketPreset;

const VISIBLE_BALANCE_LIMIT: usize = 4;
const VISIBLE_POSITION_LIMIT: usize = 4;

pub(crate) fn rows(
    state: &AppState,
    snapshot: &crate::AccountSnapshot,
    mouse_target: Option<MouseTarget>,
    first_content_row: usize,
) -> Vec<AccountPanelRow> {
    let holdings = snapshot.holdings();
    if holdings.is_empty() {
        return Vec::new();
    }

    let sections = [
        LimitedAccountSection::new(
            format!("spot balances ({})", holdings.spot_balances.len()),
            "more spot balances",
            VISIBLE_BALANCE_LIMIT,
            holdings
                .spot_balances
                .iter()
                .map(|balance| {
                    let text = format!(
                        "{} free:{} locked:{}",
                        balance.asset,
                        balance.free.as_deref().unwrap_or("-"),
                        balance.locked.as_deref().unwrap_or("-")
                    );
                    transfer_row(
                        text,
                        TransferDirection::SpotToUsdsFutures,
                        &balance.asset,
                        balance.free.as_deref(),
                    )
                })
                .collect(),
        ),
        LimitedAccountSection::new(
            format!("USD-M assets ({})", holdings.futures_assets.len()),
            "more USD-M assets",
            VISIBLE_BALANCE_LIMIT,
            holdings
                .futures_assets
                .iter()
                .map(|asset| {
                    let text = format!(
                        "{} wallet:{} availableUsd:{} margin:{} withdraw:{} upnl:{}",
                        asset.asset,
                        asset.wallet_balance.as_deref().unwrap_or("-"),
                        asset.available_balance_usd.as_deref().unwrap_or("-"),
                        asset.margin_balance.as_deref().unwrap_or("-"),
                        asset.max_withdraw_amount.as_deref().unwrap_or("-"),
                        asset.unrealized_profit.as_deref().unwrap_or("-")
                    );
                    transfer_row(
                        text,
                        TransferDirection::UsdsFuturesToSpot,
                        &asset.asset,
                        asset.max_withdraw_amount.as_deref(),
                    )
                })
                .collect(),
        ),
        LimitedAccountSection::new(
            format!("USD-M positions ({})", holdings.futures_positions.len()),
            "more USD-M positions",
            VISIBLE_POSITION_LIMIT,
            holdings
                .futures_positions
                .iter()
                .map(|position| {
                    futures_state_row(
                        format!(
                            "{} {} amt:{} notional:{} isoMargin:{} isoWallet:{} upnl:{}",
                            position.symbol,
                            position.position_side.as_deref().unwrap_or("-"),
                            position.position_amount,
                            position.notional.as_deref().unwrap_or("-"),
                            position.isolated_margin.as_deref().unwrap_or("-"),
                            position.isolated_wallet.as_deref().unwrap_or("-"),
                            position.unrealized_profit.as_deref().unwrap_or("-")
                        ),
                        &position.symbol,
                    )
                })
                .collect(),
        ),
    ];

    let mut next_content_row = first_content_row;
    sections.into_iter().fold(Vec::new(), |mut rows, section| {
        let include_spacer = rows.is_empty();
        rows.extend(section.into_panel_rows(
            state,
            include_spacer,
            mouse_target,
            &mut next_content_row,
        ));
        rows
    })
}

struct LimitedAccountSection {
    title: String,
    hidden_label: &'static str,
    visible_limit: usize,
    rows: Vec<LimitedAccountRow>,
}

impl LimitedAccountSection {
    fn new(
        title: impl Into<String>,
        hidden_label: &'static str,
        visible_limit: usize,
        rows: Vec<LimitedAccountRow>,
    ) -> Self {
        Self {
            title: title.into(),
            hidden_label,
            visible_limit,
            rows,
        }
    }

    fn into_panel_rows(
        self,
        state: &AppState,
        include_spacer: bool,
        mouse_target: Option<MouseTarget>,
        next_content_row: &mut usize,
    ) -> Vec<AccountPanelRow> {
        if self.rows.is_empty() {
            return Vec::new();
        }

        let mut rows = Vec::new();
        if include_spacer {
            rows.push(AccountPanelRow::text(""));
            *next_content_row += 1;
        }
        rows.push(AccountPanelRow::line(Line::from(Span::styled(
            self.title,
            state.theme.accent_style().add_modifier(Modifier::BOLD),
        ))));
        *next_content_row += 1;

        let total = self.rows.len();
        rows.extend(self.rows.into_iter().take(self.visible_limit).map(|row| {
            let content_row = *next_content_row;
            *next_content_row += 1;
            match row.hit {
                Some(hit) => {
                    AccountPanelRow::clickable_text(state, row.text, content_row, hit, mouse_target)
                }
                None => AccountPanelRow::text(row.text),
            }
        }));

        let before_hidden = rows.len();
        push_hidden_row(
            state,
            &mut rows,
            total.saturating_sub(self.visible_limit),
            self.hidden_label,
        );
        *next_content_row += rows.len().saturating_sub(before_hidden);
        rows
    }
}

struct LimitedAccountRow {
    text: String,
    hit: Option<AccountPanelHit>,
}

impl LimitedAccountRow {
    fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            hit: None,
        }
    }

    fn transfer_preset(text: impl Into<String>, preset: TransferTicketPreset) -> Self {
        Self {
            text: text.into(),
            hit: Some(AccountPanelHit::TicketPreset(
                AccountTicketPreset::Transfer(preset),
            )),
        }
    }

    fn futures_state_preset(text: impl Into<String>, preset: FuturesStateTicketPreset) -> Self {
        Self {
            text: text.into(),
            hit: Some(AccountPanelHit::TicketPreset(
                AccountTicketPreset::FuturesState(preset),
            )),
        }
    }
}

fn transfer_row(
    text: String,
    direction: TransferDirection,
    asset: &str,
    amount: Option<&str>,
) -> LimitedAccountRow {
    let Some(amount) = amount.filter(|amount| is_positive_amount(amount)) else {
        return LimitedAccountRow::text(text);
    };
    LimitedAccountRow::transfer_preset(
        text,
        TransferTicketPreset {
            direction,
            asset: asset.to_string(),
            amount: amount.to_string(),
        },
    )
}

fn futures_state_row(text: String, symbol: &str) -> LimitedAccountRow {
    LimitedAccountRow::futures_state_preset(
        text,
        FuturesStateTicketPreset {
            symbol: symbol.to_string(),
        },
    )
}

fn is_positive_amount(value: &str) -> bool {
    value
        .parse::<rust_decimal::Decimal>()
        .is_ok_and(|amount| amount > rust_decimal::Decimal::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_finance_core::{Environment, Provider, SignedReadRequest, SignedReadSnapshot};

    #[test]
    fn rows_only_mark_transferable_holdings_as_presets() {
        let mut state = AppState::from_config(crate::config::TuiConfig {
            trading: crate::config::TradingConfig {
                default_profile: Some("mainnet".to_string()),
            },
            ..crate::config::TuiConfig::default()
        });
        let snapshot = account_snapshot_with_non_transferable_holdings("mainnet");
        state.account_snapshot = Some(snapshot.clone());

        let rows = rows(&state, &snapshot, None, 0);
        let text = rows_text(&rows);

        assert!(text.contains("LOCKED free:0 locked:9"));
        assert!(text.contains("BAD free:not-a-decimal locked:1"));
        assert!(text.contains("USDT wallet:7.25 availableUsd:5 margin:6.75 withdraw:0 upnl:0"));
        assert_eq!(
            rows.iter()
                .filter_map(transfer_preset)
                .cloned()
                .collect::<Vec<_>>(),
            vec![TransferTicketPreset {
                direction: TransferDirection::SpotToUsdsFutures,
                asset: "USDC".to_string(),
                amount: "3.25".to_string(),
            }]
        );
    }

    #[test]
    fn rows_mark_visible_futures_positions_as_futures_state_presets() {
        let state = AppState::from_config(crate::config::TuiConfig {
            trading: crate::config::TradingConfig {
                default_profile: Some("mainnet".to_string()),
            },
            ..crate::config::TuiConfig::default()
        });
        let snapshot = account_snapshot_with_position("mainnet");

        let rows = rows(&state, &snapshot, None, 0);
        let text = rows_text(&rows);

        assert!(text.contains("USD-M positions (1)"));
        assert!(text.contains("ETHUSDT LONG amt:0.25"));
        assert_eq!(
            rows.iter()
                .filter_map(futures_state_preset)
                .cloned()
                .collect::<Vec<_>>(),
            vec![FuturesStateTicketPreset {
                symbol: "ETHUSDT".to_string(),
            }]
        );
    }

    fn account_snapshot_with_non_transferable_holdings(profile: &str) -> crate::AccountSnapshot {
        crate::AccountSnapshot::new(
            profile.to_string(),
            Provider::Binance,
            Environment::Live,
            crate::profile_snapshot::test_trading_profile_snapshot(),
            vec![
                SignedReadSnapshot::new(
                    profile.to_string(),
                    Provider::Binance,
                    Environment::Live,
                    SignedReadRequest::SpotBalances,
                    serde_json::json!({
                        "balances": [
                            { "asset": "ZERO", "free": "0", "locked": "0" },
                            { "asset": "LOCKED", "free": "0", "locked": "9" },
                            { "asset": "BAD", "free": "not-a-decimal", "locked": "1" },
                            { "asset": "USDC", "free": "3.25", "locked": "0" }
                        ]
                    }),
                ),
                SignedReadSnapshot::new(
                    profile.to_string(),
                    Provider::Binance,
                    Environment::Live,
                    SignedReadRequest::UsdsFuturesPositions,
                    serde_json::json!({
                        "assets": [
                            {
                                "asset": "USDT",
                                "walletBalance": "7.25",
                                "availableBalance": "5",
                                "marginBalance": "6.75",
                                "maxWithdrawAmount": "0",
                                "unrealizedProfit": "0"
                            }
                        ],
                        "positions": []
                    }),
                ),
            ],
            Vec::new(),
        )
    }

    fn account_snapshot_with_position(profile: &str) -> crate::AccountSnapshot {
        crate::AccountSnapshot::new(
            profile.to_string(),
            Provider::Binance,
            Environment::Live,
            crate::profile_snapshot::test_trading_profile_snapshot(),
            vec![SignedReadSnapshot::new(
                profile.to_string(),
                Provider::Binance,
                Environment::Live,
                SignedReadRequest::UsdsFuturesPositions,
                serde_json::json!({
                    "assets": [],
                    "positions": [
                        {
                            "symbol": "ETHUSDT",
                            "positionSide": "LONG",
                            "positionAmt": "0.25",
                            "notional": "1000",
                            "isolatedMargin": "0",
                            "isolatedWallet": "0",
                            "unrealizedProfit": "12.5"
                        },
                        {
                            "symbol": "BTCUSDT",
                            "positionSide": "BOTH",
                            "positionAmt": "0",
                            "notional": "0",
                            "isolatedMargin": "0",
                            "isolatedWallet": "0",
                            "unrealizedProfit": "0"
                        }
                    ]
                }),
            )],
            Vec::new(),
        )
    }

    fn rows_text(rows: &[AccountPanelRow]) -> String {
        rows.iter()
            .map(|row| {
                row.line
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn transfer_preset(row: &AccountPanelRow) -> Option<&TransferTicketPreset> {
        match row.hit.as_ref()? {
            AccountPanelHit::OpenOrder(_) => None,
            AccountPanelHit::TicketPreset(AccountTicketPreset::FuturesState(_)) => None,
            AccountPanelHit::TicketPreset(AccountTicketPreset::Transfer(preset)) => Some(preset),
        }
    }

    fn futures_state_preset(row: &AccountPanelRow) -> Option<&FuturesStateTicketPreset> {
        match row.hit.as_ref()? {
            AccountPanelHit::OpenOrder(_) => None,
            AccountPanelHit::TicketPreset(AccountTicketPreset::Transfer(_)) => None,
            AccountPanelHit::TicketPreset(AccountTicketPreset::FuturesState(preset)) => {
                Some(preset)
            }
        }
    }
}
