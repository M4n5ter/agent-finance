use std::fmt;

use serde::{Deserialize, Serialize};

use crate::intent::{IntentEnvelope, IntentKind};
use crate::profile::Profile;
use crate::risk::RiskDecision;
use crate::types::{Environment, Provider};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SubmitMode {
    DryRun,
    Test,
    Live,
}

impl fmt::Display for SubmitMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DryRun => formatter.write_str("dry-run"),
            Self::Test => formatter.write_str("test"),
            Self::Live => formatter.write_str("live"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SubmitIntentKind {
    Order,
    Cancel,
    Transfer,
    FuturesState,
}

impl fmt::Display for SubmitIntentKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Order => formatter.write_str("order"),
            Self::Cancel => formatter.write_str("cancel"),
            Self::Transfer => formatter.write_str("transfer"),
            Self::FuturesState => formatter.write_str("futures-state"),
        }
    }
}

impl From<&IntentKind> for SubmitIntentKind {
    fn from(intent: &IntentKind) -> Self {
        match intent {
            IntentKind::Order(_) => Self::Order,
            IntentKind::Cancel(_) => Self::Cancel,
            IntentKind::Transfer(_) => Self::Transfer,
            IntentKind::FuturesState(_) => Self::FuturesState,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SubmitExecutionKind {
    Plan,
    OrderSubmit,
    Cancel,
    Transfer,
    FuturesState,
}

impl fmt::Display for SubmitExecutionKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plan => formatter.write_str("plan"),
            Self::OrderSubmit => formatter.write_str("order-submit"),
            Self::Cancel => formatter.write_str("cancel"),
            Self::Transfer => formatter.write_str("transfer"),
            Self::FuturesState => formatter.write_str("futures-state"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitExecutionSnapshot {
    pub kind: SubmitExecutionKind,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitSnapshot {
    pub profile: String,
    pub provider: Provider,
    pub environment: Environment,
    pub intent_id: String,
    pub intent_kind: SubmitIntentKind,
    pub mode: SubmitMode,
    pub risk: RiskDecision,
    pub execution: SubmitExecutionSnapshot,
}

impl SubmitSnapshot {
    pub fn from_envelope(
        profile: &Profile,
        envelope: &IntentEnvelope,
        mode: SubmitMode,
        risk: RiskDecision,
        execution: SubmitExecutionSnapshot,
    ) -> Self {
        Self {
            profile: profile.name.clone(),
            provider: profile.provider.provider,
            environment: profile.provider.environment,
            intent_id: envelope.id.clone(),
            intent_kind: SubmitIntentKind::from(&envelope.kind),
            mode,
            risk,
            execution,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::str::FromStr;

    use chrono::{TimeDelta, Utc};
    use serde_json::json;

    use super::*;
    use crate::intent::IntentStatus;
    use crate::types::{
        CancelIntent, DecimalValue, FuturesStateChange, FuturesStateIntent, Market,
        OrderIdentifier, OrderIntent, OrderSide, OrderSpec, ProfilePermissions, ProviderConfig,
        RiskPolicy, TimeInForce, TransferDirection, TransferIntent,
    };

    #[test]
    fn submit_discriminators_are_stable_cli_contract() {
        let modes = [
            (SubmitMode::DryRun, "dry-run"),
            (SubmitMode::Test, "test"),
            (SubmitMode::Live, "live"),
        ];
        for (mode, expected) in modes {
            assert_eq!(serde_json::to_value(mode).expect("mode json"), expected);
            assert_eq!(mode.to_string(), expected);
        }

        let intent_kinds = [
            (SubmitIntentKind::Order, "order"),
            (SubmitIntentKind::Cancel, "cancel"),
            (SubmitIntentKind::Transfer, "transfer"),
            (SubmitIntentKind::FuturesState, "futures-state"),
        ];
        for (kind, expected) in intent_kinds {
            assert_eq!(
                serde_json::to_value(kind).expect("intent kind json"),
                expected
            );
            assert_eq!(kind.to_string(), expected);
        }

        let execution_kinds = [
            (SubmitExecutionKind::Plan, "plan"),
            (SubmitExecutionKind::OrderSubmit, "order-submit"),
            (SubmitExecutionKind::Cancel, "cancel"),
            (SubmitExecutionKind::Transfer, "transfer"),
            (SubmitExecutionKind::FuturesState, "futures-state"),
        ];
        for (kind, expected) in execution_kinds {
            assert_eq!(
                serde_json::to_value(kind).expect("execution kind json"),
                expected
            );
            assert_eq!(kind.to_string(), expected);
        }
    }

    #[test]
    fn submit_snapshot_derives_intent_kind_from_intent() {
        let cases = [
            (
                IntentKind::Order(OrderIntent {
                    profile: "default".to_string(),
                    provider: Provider::Binance,
                    environment: Environment::Testnet,
                    market: Market::Spot,
                    symbol: "ETHUSDT".to_string(),
                    side: OrderSide::Buy,
                    quantity: decimal("0.01"),
                    spec: OrderSpec::Limit {
                        price: decimal("3000"),
                        time_in_force: TimeInForce::Gtc,
                    },
                    reduce_only: false,
                    position_side: None,
                    client_order_id: "order-1".to_string(),
                }),
                "order",
            ),
            (
                IntentKind::Cancel(CancelIntent {
                    profile: "default".to_string(),
                    provider: Provider::Binance,
                    environment: Environment::Testnet,
                    market: Market::Spot,
                    symbol: "ETHUSDT".to_string(),
                    target: OrderIdentifier::ClientOrderId {
                        client_order_id: "order-1".to_string(),
                    },
                }),
                "cancel",
            ),
            (
                IntentKind::Transfer(TransferIntent {
                    profile: "default".to_string(),
                    provider: Provider::Binance,
                    environment: Environment::Testnet,
                    direction: TransferDirection::SpotToUsdsFutures,
                    asset: "USDT".to_string(),
                    amount: decimal("5"),
                    client_transfer_id: "transfer-1".to_string(),
                }),
                "transfer",
            ),
            (
                IntentKind::FuturesState(FuturesStateIntent {
                    profile: "default".to_string(),
                    provider: Provider::Binance,
                    environment: Environment::Testnet,
                    change: FuturesStateChange::Leverage {
                        symbol: "ETHUSDT".to_string(),
                        leverage: 2,
                    },
                }),
                "futures-state",
            ),
        ];

        for (intent, expected_kind) in cases {
            let profile = test_profile();
            let envelope = test_envelope(intent);
            let snapshot = SubmitSnapshot::from_envelope(
                &profile,
                &envelope,
                SubmitMode::DryRun,
                RiskDecision {
                    allowed: true,
                    findings: Vec::new(),
                },
                SubmitExecutionSnapshot {
                    kind: SubmitExecutionKind::Plan,
                    payload: json!({"request": {"method": "POST"}}),
                },
            );

            let value = serde_json::to_value(snapshot).expect("submit snapshot json");

            assert_eq!(value["profile"], "default");
            assert_eq!(value["provider"], "binance");
            assert_eq!(value["environment"], "testnet");
            assert_eq!(value["intent_id"], "intent-1");
            assert_eq!(value["intent_kind"], expected_kind);
            assert_eq!(value["mode"], "dry-run");
            assert_eq!(value["risk"]["allowed"], true);
            assert_eq!(value["execution"]["kind"], "plan");
            assert_eq!(value["execution"]["payload"]["request"]["method"], "POST");
        }
    }

    fn decimal(value: &str) -> DecimalValue {
        DecimalValue::from_str(value).expect("valid decimal")
    }

    fn test_profile() -> Profile {
        Profile {
            name: "default".to_string(),
            provider: ProviderConfig {
                provider: Provider::Binance,
                environment: Environment::Testnet,
                api_key_env: "TEST_API_KEY".to_string(),
                api_secret_env: "TEST_API_SECRET".to_string(),
                spot_base_url: None,
                usds_futures_base_url: None,
                sapi_base_url: None,
            },
            permissions: ProfilePermissions::default(),
            risk: RiskPolicy {
                allow_live: false,
                max_daily_order_notional_usdt: None,
                allowed_symbols: BTreeMap::new(),
                allowed_transfers: Vec::new(),
                allowed_futures_state_changes: Vec::new(),
            },
        }
    }

    fn test_envelope(kind: IntentKind) -> IntentEnvelope {
        let created_at_utc = Utc::now();
        IntentEnvelope {
            id: "intent-1".to_string(),
            hash: "test-hash".to_string(),
            metadata: crate::IntentMetadata {
                created_at_utc,
                expires_at_utc: created_at_utc + TimeDelta::minutes(5),
                status: IntentStatus::Created,
            },
            kind,
        }
    }
}
