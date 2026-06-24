use agent_finance_core::{IntentKind, Market, Profile, RiskPolicy, SymbolPolicy, TransferPolicy};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct PermissionCheck {
    pub name: &'static str,
    pub ok: bool,
    pub required: bool,
    pub message: String,
}

pub fn profile_permission_checks(profile: &Profile, payload: &Value) -> Vec<PermissionCheck> {
    let requirements = requirements_for_risk(&profile.risk);
    permission_checks(requirements, payload)
}

pub fn intent_permission_checks(intent: &IntentKind, payload: &Value) -> Vec<PermissionCheck> {
    let requirements = requirements_for_intent(intent);
    permission_checks(requirements, payload)
}

pub fn blocking_permission_error(checks: &[PermissionCheck]) -> Option<String> {
    let failed = checks
        .iter()
        .filter(|check| check.required && !check.ok)
        .map(|check| check.name)
        .collect::<Vec<_>>();
    if failed.is_empty() {
        None
    } else {
        Some(format!(
            "Binance API key permissions are insufficient: {}",
            failed.join(", ")
        ))
    }
}

fn permission_checks(
    requirements: PermissionRequirements,
    payload: &Value,
) -> Vec<PermissionCheck> {
    [
        permission_check(
            "binance-spot-trading",
            requirements.spot_trading,
            bool_any(payload, &["enableSpotAndMarginTrading"]),
            "spot trading is required for live spot orders and cancels",
        ),
        permission_check(
            "binance-usds-futures",
            requirements.usds_futures,
            bool_any(payload, &["enableFutures"]),
            "USD-M futures permission is required for futures orders and state changes",
        ),
        permission_check(
            "binance-universal-transfer",
            requirements.universal_transfer,
            bool_any(payload, &["permitsUniversalTransfer"]),
            "universal transfer permission is required for Spot <-> USD-M transfers",
        ),
    ]
    .into_iter()
    .collect()
}

fn permission_check(
    name: &'static str,
    required: bool,
    granted: Option<bool>,
    required_message: &str,
) -> PermissionCheck {
    let ok = !required || granted == Some(true);
    let message = match (required, granted) {
        (false, Some(true)) => "permission is present but not required by this profile".to_string(),
        (false, Some(false)) => "permission is absent and not required by this profile".to_string(),
        (false, None) => "permission field is missing and not required by this profile".to_string(),
        (true, Some(true)) => "required permission is present".to_string(),
        (true, Some(false)) => format!("required permission is disabled; {required_message}"),
        (true, None) => format!("required permission field is missing; {required_message}"),
    };
    PermissionCheck {
        name,
        ok,
        required,
        message,
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct PermissionRequirements {
    spot_trading: bool,
    usds_futures: bool,
    universal_transfer: bool,
}

fn requirements_for_risk(risk: &RiskPolicy) -> PermissionRequirements {
    PermissionRequirements {
        spot_trading: risk.allowed_symbols.values().any(policy_allows_spot),
        usds_futures: risk
            .allowed_symbols
            .values()
            .any(policy_allows_usds_futures)
            || !risk.allowed_futures_state_changes.is_empty(),
        universal_transfer: risk
            .allowed_transfers
            .iter()
            .any(transfer_requires_universal),
    }
}

fn requirements_for_intent(intent: &IntentKind) -> PermissionRequirements {
    match intent {
        IntentKind::Order(intent) => market_requirements(intent.market),
        IntentKind::Cancel(intent) => market_requirements(intent.market),
        IntentKind::Transfer(_) => PermissionRequirements {
            universal_transfer: true,
            ..PermissionRequirements::default()
        },
        IntentKind::FuturesState(_) => PermissionRequirements {
            usds_futures: true,
            ..PermissionRequirements::default()
        },
    }
}

fn market_requirements(market: Market) -> PermissionRequirements {
    match market {
        Market::Spot => PermissionRequirements {
            spot_trading: true,
            ..PermissionRequirements::default()
        },
        Market::UsdsFutures => PermissionRequirements {
            usds_futures: true,
            ..PermissionRequirements::default()
        },
    }
}

fn policy_allows_spot(policy: &SymbolPolicy) -> bool {
    policy.markets.contains(&Market::Spot)
}

fn policy_allows_usds_futures(policy: &SymbolPolicy) -> bool {
    policy.markets.contains(&Market::UsdsFutures)
}

fn transfer_requires_universal(_policy: &TransferPolicy) -> bool {
    true
}

fn bool_any(payload: &Value, keys: &[&str]) -> Option<bool> {
    keys.iter().find_map(|key| payload.get(*key)?.as_bool())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_finance_core::{
        DecimalValue, Environment, FuturesStateIntent, OrderIntent, OrderSide, OrderSpec, Provider,
        ProviderConfig, RiskPolicy, TimeInForce,
    };
    use rust_decimal::Decimal;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn profile_checks_follow_declared_risk_surface() {
        let profile = profile_with_risk(RiskPolicy {
            allow_live: true,
            max_daily_order_notional_usdt: None,
            allowed_symbols: BTreeMap::from([(
                "BTCUSDT".to_string(),
                agent_finance_core::SymbolPolicy {
                    markets: vec![Market::Spot, Market::UsdsFutures],
                    order_kinds: Vec::new(),
                    max_order_notional_usdt: decimal("10"),
                },
            )]),
            allowed_transfers: vec![agent_finance_core::TransferPolicy {
                direction: agent_finance_core::TransferDirection::SpotToUsdsFutures,
                asset: "USDT".to_string(),
                max_amount: decimal("1"),
            }],
            allowed_futures_state_changes: Vec::new(),
        });
        let checks = profile_permission_checks(
            &profile,
            &json!({
                "enableSpotAndMarginTrading": true,
                "enableFutures": false,
                "permitsUniversalTransfer": false
            }),
        );

        assert!(check(&checks, "binance-spot-trading").ok);
        assert!(!check(&checks, "binance-usds-futures").ok);
        assert!(!check(&checks, "binance-universal-transfer").ok);
        assert!(blocking_permission_error(&checks).is_some());
    }

    #[test]
    fn intent_checks_are_scoped_to_the_live_write() {
        let payload = json!({
            "enableSpotAndMarginTrading": false,
            "enableFutures": true,
            "permitsUniversalTransfer": false
        });
        let spot = intent_permission_checks(&spot_order(), &payload);
        let futures = intent_permission_checks(&futures_state(), &payload);

        assert!(!check(&spot, "binance-spot-trading").ok);
        assert!(!check(&spot, "binance-usds-futures").required);
        assert!(check(&futures, "binance-usds-futures").ok);
        assert!(!check(&futures, "binance-spot-trading").required);
    }

    fn check<'a>(checks: &'a [PermissionCheck], name: &str) -> &'a PermissionCheck {
        checks.iter().find(|check| check.name == name).expect(name)
    }

    fn profile_with_risk(risk: RiskPolicy) -> Profile {
        Profile {
            name: "test".to_string(),
            provider: ProviderConfig {
                provider: Provider::Binance,
                environment: Environment::Live,
                api_key_env: "BINANCE_API_KEY".to_string(),
                api_secret_env: "BINANCE_PRIVATE_KEY".to_string(),
                spot_base_url: None,
                usds_futures_base_url: None,
                sapi_base_url: None,
            },
            risk,
        }
    }

    fn spot_order() -> IntentKind {
        IntentKind::Order(OrderIntent {
            profile: "test".to_string(),
            provider: Provider::Binance,
            environment: Environment::Live,
            market: Market::Spot,
            symbol: "BTCUSDT".to_string(),
            side: OrderSide::Buy,
            quantity: decimal("0.001"),
            spec: OrderSpec::Limit {
                price: decimal("50000"),
                time_in_force: TimeInForce::Gtc,
            },
            reduce_only: false,
            position_side: None,
            client_order_id: "af-test".to_string(),
        })
    }

    fn futures_state() -> IntentKind {
        IntentKind::FuturesState(FuturesStateIntent {
            profile: "test".to_string(),
            provider: Provider::Binance,
            environment: Environment::Live,
            change: agent_finance_core::FuturesStateChange::Leverage {
                symbol: "BTCUSDT".to_string(),
                leverage: 2,
            },
        })
    }

    fn decimal(value: &str) -> DecimalValue {
        DecimalValue(value.parse::<Decimal>().expect("decimal"))
    }
}
