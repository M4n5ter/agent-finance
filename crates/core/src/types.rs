use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use anyhow::{Result, anyhow};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    Binance,
}

impl fmt::Display for Provider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Binance => formatter.write_str("binance"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Environment {
    Testnet,
    Live,
}

impl Environment {
    pub const fn is_live(self) -> bool {
        matches!(self, Self::Live)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Market {
    Spot,
    UsdsFutures,
}

impl fmt::Display for Market {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spot => formatter.write_str("spot"),
            Self::UsdsFutures => formatter.write_str("usds-futures"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrderSide {
    Buy,
    Sell,
}

impl fmt::Display for OrderSide {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Buy => formatter.write_str("buy"),
            Self::Sell => formatter.write_str("sell"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrderKind {
    Market,
    Limit,
    StopLoss,
    TakeProfit,
}

impl fmt::Display for OrderKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Market => formatter.write_str("market"),
            Self::Limit => formatter.write_str("limit"),
            Self::StopLoss => formatter.write_str("stop-loss"),
            Self::TakeProfit => formatter.write_str("take-profit"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TimeInForce {
    Gtc,
    Ioc,
    Fok,
}

impl fmt::Display for TimeInForce {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gtc => formatter.write_str("GTC"),
            Self::Ioc => formatter.write_str("IOC"),
            Self::Fok => formatter.write_str("FOK"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PositionSide {
    Both,
    Long,
    Short,
}

impl fmt::Display for PositionSide {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Both => formatter.write_str("BOTH"),
            Self::Long => formatter.write_str("LONG"),
            Self::Short => formatter.write_str("SHORT"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MarginType {
    Cross,
    Isolated,
}

impl fmt::Display for MarginType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cross => formatter.write_str("CROSSED"),
            Self::Isolated => formatter.write_str("ISOLATED"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransferDirection {
    SpotToUsdsFutures,
    UsdsFuturesToSpot,
}

impl fmt::Display for TransferDirection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpotToUsdsFutures => formatter.write_str("spot-to-usds-futures"),
            Self::UsdsFuturesToSpot => formatter.write_str("usds-futures-to-spot"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecimalValue(#[serde(with = "rust_decimal::serde::str")] pub Decimal);

impl DecimalValue {
    pub fn new(value: Decimal) -> Self {
        Self(value)
    }

    pub fn checked_mul(&self, other: &Self) -> Option<Self> {
        self.0.checked_mul(other.0).map(Self)
    }
}

impl fmt::Display for DecimalValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0.normalize())
    }
}

impl FromStr for DecimalValue {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let decimal = Decimal::from_str(value.trim())
            .map_err(|_| anyhow!("invalid decimal value: {value}"))?;
        if decimal <= Decimal::ZERO {
            return Err(anyhow!("decimal value must be positive: {value}"));
        }
        Ok(Self(decimal))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderIntent {
    pub profile: String,
    pub provider: Provider,
    pub environment: Environment,
    pub market: Market,
    pub symbol: String,
    pub side: OrderSide,
    pub quantity: DecimalValue,
    pub spec: OrderSpec,
    pub reduce_only: bool,
    pub position_side: Option<PositionSide>,
    pub client_order_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OrderSpec {
    Market {
        valuation_price: DecimalValue,
    },
    Limit {
        price: DecimalValue,
        time_in_force: TimeInForce,
    },
    StopLoss {
        stop_price: DecimalValue,
    },
    TakeProfit {
        stop_price: DecimalValue,
    },
}

impl OrderSpec {
    pub fn new(
        market: Market,
        kind: OrderKind,
        price: Option<DecimalValue>,
        valuation_price: Option<DecimalValue>,
        stop_price: Option<DecimalValue>,
        time_in_force: Option<TimeInForce>,
    ) -> Result<Self> {
        match kind {
            OrderKind::Market => {
                reject_present("price", price.as_ref())?;
                reject_present("stop price", stop_price.as_ref())?;
                if time_in_force.is_some() {
                    return Err(anyhow!("market order does not accept time in force"));
                }
                Ok(Self::Market {
                    valuation_price: valuation_price
                        .ok_or_else(|| anyhow!("market order requires valuation price"))?,
                })
            }
            OrderKind::Limit => {
                reject_present("valuation price", valuation_price.as_ref())?;
                reject_present("stop price", stop_price.as_ref())?;
                Ok(Self::Limit {
                    price: price.ok_or_else(|| anyhow!("limit order requires price"))?,
                    time_in_force: time_in_force
                        .ok_or_else(|| anyhow!("limit order requires time in force"))?,
                })
            }
            OrderKind::StopLoss | OrderKind::TakeProfit if market == Market::UsdsFutures => {
                Err(anyhow!(
                    "{kind} is not supported for usds-futures yet; use a provider-specific order model once futures conditional orders are modeled"
                ))
            }
            OrderKind::StopLoss => {
                reject_present("price", price.as_ref())?;
                reject_present("valuation price", valuation_price.as_ref())?;
                if time_in_force.is_some() {
                    return Err(anyhow!("stop-loss order does not accept time in force"));
                }
                Ok(Self::StopLoss {
                    stop_price: stop_price
                        .ok_or_else(|| anyhow!("stop-loss order requires stop price"))?,
                })
            }
            OrderKind::TakeProfit => {
                reject_present("price", price.as_ref())?;
                reject_present("valuation price", valuation_price.as_ref())?;
                if time_in_force.is_some() {
                    return Err(anyhow!("take-profit order does not accept time in force"));
                }
                Ok(Self::TakeProfit {
                    stop_price: stop_price
                        .ok_or_else(|| anyhow!("take-profit order requires stop price"))?,
                })
            }
        }
    }

    pub const fn kind(&self) -> OrderKind {
        match self {
            Self::Market { .. } => OrderKind::Market,
            Self::Limit { .. } => OrderKind::Limit,
            Self::StopLoss { .. } => OrderKind::StopLoss,
            Self::TakeProfit { .. } => OrderKind::TakeProfit,
        }
    }

    pub const fn notional_price(&self) -> &DecimalValue {
        match self {
            Self::Market { valuation_price } => valuation_price,
            Self::Limit { price, .. } => price,
            Self::StopLoss { stop_price } | Self::TakeProfit { stop_price } => stop_price,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelIntent {
    pub profile: String,
    pub provider: Provider,
    pub environment: Environment,
    pub market: Market,
    pub symbol: String,
    pub target: CancelTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CancelTarget {
    OrderId { order_id: String },
    ClientOrderId { client_order_id: String },
}

impl CancelTarget {
    pub fn new(order_id: Option<String>, client_order_id: Option<String>) -> Result<Self> {
        match (order_id, client_order_id) {
            (Some(order_id), None) => Ok(Self::OrderId { order_id }),
            (None, Some(client_order_id)) => Ok(Self::ClientOrderId { client_order_id }),
            (Some(_), Some(_)) => Err(anyhow!(
                "cancel intent accepts exactly one of order id or client order id"
            )),
            (None, None) => Err(anyhow!(
                "cancel intent requires order id or client order id"
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferIntent {
    pub profile: String,
    pub provider: Provider,
    pub environment: Environment,
    pub direction: TransferDirection,
    pub asset: String,
    pub amount: DecimalValue,
    pub client_transfer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: Provider,
    pub environment: Environment,
    pub api_key_env: String,
    pub api_secret_env: String,
    pub spot_base_url: Option<String>,
    pub usds_futures_base_url: Option<String>,
    pub sapi_base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskPolicy {
    pub allow_live: bool,
    #[serde(default)]
    pub allowed_symbols: BTreeMap<String, SymbolPolicy>,
    #[serde(default)]
    pub allowed_transfers: Vec<TransferPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolPolicy {
    #[serde(default)]
    pub markets: Vec<Market>,
    #[serde(default)]
    pub order_kinds: Vec<OrderKind>,
    pub max_order_notional_usdt: DecimalValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferPolicy {
    pub direction: TransferDirection,
    pub asset: String,
    pub max_amount: DecimalValue,
}

impl fmt::Display for TransferPolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}<= {}",
            self.direction,
            self.asset.to_ascii_uppercase(),
            self.max_amount
        )
    }
}

fn reject_present<T>(name: &str, value: Option<&T>) -> Result<()> {
    if value.is_some() {
        return Err(anyhow!("{name} is not valid for this order kind"));
    }
    Ok(())
}
