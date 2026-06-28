use std::path::PathBuf;

use agent_finance_core::{DiagnosticCheck, Profile, ProfilePermission, RiskPolicy};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TradingProfileSnapshot {
    pub declared_permissions: Vec<ProfilePermission>,
    pub required_permissions: Vec<ProfilePermission>,
    pub missing_permissions: Vec<ProfilePermission>,
    pub risk: RiskPolicy,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfileValidationSnapshot {
    pub profile: String,
    pub path: PathBuf,
    pub profile_config: Profile,
    pub checks: Vec<DiagnosticCheck>,
}

impl ProfileValidationSnapshot {
    pub fn from_profile(profile: &Profile, path: PathBuf) -> Self {
        Self {
            profile: profile.name.clone(),
            path,
            profile_config: profile.clone(),
            checks: agent_finance_core::local_profile_checks(profile),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProfileValidationState {
    Idle,
    Loading {
        profile: String,
    },
    Ready {
        profile: String,
        path: PathBuf,
        profile_config: Box<Profile>,
        checks: Vec<DiagnosticCheck>,
    },
    Failed {
        profile: String,
        error: String,
    },
}

impl ProfileValidationState {
    pub const fn idle() -> Self {
        Self::Idle
    }

    pub fn loading(profile: String) -> Self {
        Self::Loading { profile }
    }

    pub fn ready(snapshot: ProfileValidationSnapshot) -> Self {
        Self::Ready {
            profile: snapshot.profile,
            path: snapshot.path,
            profile_config: Box::new(snapshot.profile_config),
            checks: snapshot.checks,
        }
    }

    pub fn failed(profile: String, error: String) -> Self {
        Self::Failed { profile, error }
    }

    pub fn terminal_for(&self, profile: &str) -> bool {
        matches!(
            self,
            Self::Ready { profile: active, .. } | Self::Failed { profile: active, .. }
                if active == profile
        )
    }
}

impl From<&Profile> for TradingProfileSnapshot {
    fn from(profile: &Profile) -> Self {
        let declared_permissions = ProfilePermission::ALL
            .into_iter()
            .filter(|permission| profile.permissions.allows(*permission))
            .collect::<Vec<_>>();
        let required_permissions = profile
            .risk
            .required_profile_permissions()
            .iter()
            .collect::<Vec<_>>();
        let missing_permissions = required_permissions
            .iter()
            .copied()
            .filter(|permission| !declared_permissions.contains(permission))
            .collect();

        Self {
            declared_permissions,
            required_permissions,
            missing_permissions,
            risk: profile.risk.clone(),
        }
    }
}

#[cfg(test)]
pub(crate) fn test_trading_profile_snapshot() -> TradingProfileSnapshot {
    TradingProfileSnapshot::from(&test_profile("mainnet"))
}

#[cfg(test)]
pub(crate) fn test_profile_validation_snapshot(
    name: &str,
    path: impl Into<PathBuf>,
) -> ProfileValidationSnapshot {
    let profile = test_profile(name);
    ProfileValidationSnapshot::from_profile(&profile, path.into())
}

#[cfg(test)]
pub(crate) fn test_profile(name: &str) -> Profile {
    use std::collections::BTreeMap;

    use agent_finance_core::{
        DecimalValue, Environment, Market, OrderKind, ProfilePermissions, Provider, ProviderConfig,
        SymbolPolicy,
    };
    use rust_decimal::Decimal;

    Profile {
        name: name.to_string(),
        provider: ProviderConfig {
            provider: Provider::Binance,
            environment: Environment::Live,
            api_key_env: "BINANCE_API_KEY".to_string(),
            api_secret_env: "BINANCE_PRIVATE_KEY".to_string(),
            spot_base_url: None,
            usds_futures_base_url: None,
            sapi_base_url: None,
        },
        permissions: ProfilePermissions {
            spot_trading: true,
            usds_futures: false,
            universal_transfer: false,
        },
        risk: RiskPolicy {
            allow_live: true,
            max_daily_order_notional_usdt: Some(DecimalValue::new(Decimal::new(100, 0))),
            allowed_symbols: BTreeMap::from([(
                "btcusdt".to_string(),
                SymbolPolicy {
                    markets: vec![Market::Spot],
                    order_kinds: vec![OrderKind::Limit],
                    max_order_notional_usdt: DecimalValue::new(Decimal::new(50, 0)),
                },
            )]),
            allowed_transfers: Vec::new(),
            allowed_futures_state_changes: Vec::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_snapshot_keeps_risk_policy_typed_and_preserves_symbol_keys() {
        let profile = test_profile("mainnet");

        let snapshot = TradingProfileSnapshot::from(&profile);

        assert_eq!(
            snapshot.declared_permissions,
            vec![ProfilePermission::SpotTrading]
        );
        assert_eq!(
            snapshot.required_permissions,
            vec![ProfilePermission::SpotTrading]
        );
        assert!(snapshot.risk.allow_live);
        assert!(snapshot.risk.allowed_symbols.contains_key("btcusdt"));
        assert!(!snapshot.risk.allowed_symbols.contains_key("BTCUSDT"));
    }

    #[test]
    fn profile_snapshot_reports_missing_permissions() {
        let mut profile = test_profile("mainnet");
        profile.permissions.spot_trading = false;

        let snapshot = TradingProfileSnapshot::from(&profile);

        assert_eq!(
            snapshot.missing_permissions,
            vec![ProfilePermission::SpotTrading]
        );
    }

    #[test]
    fn profile_validation_snapshot_summarizes_required_failures() {
        let mut profile = test_profile("mainnet");
        profile.permissions.spot_trading = false;
        let snapshot =
            ProfileValidationSnapshot::from_profile(&profile, PathBuf::from("mainnet.toml"));

        assert_eq!(snapshot.profile, "mainnet");
        assert_eq!(snapshot.path, PathBuf::from("mainnet.toml"));
        assert!(snapshot.checks.iter().any(|check| {
            check.name == "profile-permission-spot-trading" && check.required && !check.ok
        }));
    }
}
