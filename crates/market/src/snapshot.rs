use std::collections::BTreeMap;

use anyhow::Result;
use serde::Serialize;

use crate::args::{AssetClass, CryptoInstrument, CryptoProvider, Provider, SessionMode};
use crate::crypto_capability::{CryptoCapability, resolve_instrument};
use crate::crypto_market_data::fetch_price_batch;
use crate::market_symbol::{MarketSymbol, canonical_lookup_key};
use crate::model::{PricePoint, PriceSummary, RegularBasis};
use crate::service::{self, MarketRuntime, PriceRequest, PriceResponse};
use crate::time;

#[derive(Debug, Clone)]
pub struct PublicQuoteSnapshotRequest {
    pub symbols: Vec<String>,
    pub equity_provider: Provider,
    pub crypto_provider: CryptoProvider,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MarketSnapshot {
    pub fetched_at_local: Option<String>,
    pub quotes: Vec<QuoteSnapshot>,
    pub errors: Vec<String>,
}

impl MarketSnapshot {
    pub fn quote_for(&self, symbol: &str) -> Option<&QuoteSnapshot> {
        self.quotes.iter().find(|quote| {
            quote.symbol == symbol || quote.aliases.iter().any(|alias| alias == symbol)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QuoteSnapshot {
    pub symbol: String,
    pub price: Option<f64>,
    pub currency: Option<String>,
    pub provider: String,
    pub session: Option<String>,
    pub market_time_local: Option<String>,
    pub change_pct: Option<f64>,
    pub regular_basis: RegularBasisSnapshot,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegularBasisSnapshot {
    pub previous_close: Option<f64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<u64>,
}

pub async fn fetch_public_quote_snapshot(
    runtime: &MarketRuntime,
    request: PublicQuoteSnapshotRequest,
) -> Result<MarketSnapshot> {
    let (crypto_symbols, equity_symbols): (Vec<_>, Vec<_>) = request
        .symbols
        .into_iter()
        .map(MarketSymbol::new)
        .partition(MarketSymbol::is_crypto);

    let (equity_result, crypto_result) = tokio::join!(
        fetch_equity_quotes(runtime, equity_symbols, request.equity_provider),
        fetch_crypto_quotes(runtime, crypto_symbols, request.crypto_provider)
    );

    let mut quotes = Vec::new();
    quotes.extend(equity_result.quotes);
    quotes.extend(crypto_result.quotes);

    let mut errors = Vec::new();
    errors.extend(equity_result.errors);
    errors.extend(crypto_result.errors);

    Ok(MarketSnapshot {
        fetched_at_local: Some(time::now_local(runtime.timezone())),
        quotes,
        errors,
    })
}

#[derive(Debug, Default)]
struct QuoteFetchResult {
    quotes: Vec<QuoteSnapshot>,
    errors: Vec<String>,
}

impl QuoteFetchResult {
    fn with_aliases(mut self, aliases_by_symbol: BTreeMap<String, Vec<String>>) -> Self {
        for quote in &mut self.quotes {
            quote.aliases = aliases_by_symbol
                .get(&canonical_lookup_key(&quote.symbol))
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|alias| alias != &quote.symbol)
                .collect();
        }
        self
    }
}

async fn fetch_equity_quotes(
    runtime: &MarketRuntime,
    symbols: Vec<MarketSymbol>,
    provider: Provider,
) -> QuoteFetchResult {
    if symbols.is_empty() {
        return QuoteFetchResult::default();
    }

    let response = service::price(
        runtime,
        PriceRequest {
            symbols: symbols.into_iter().map(|symbol| symbol.input).collect(),
            asset: AssetClass::Equity,
            instrument: CryptoInstrument::Auto,
            crypto_provider: CryptoProvider::Auto,
            provider,
            session: SessionMode::Smart,
            proxy_symbol: None,
        },
    )
    .await;
    let mut fetch = QuoteFetchResult::default();
    match response {
        Ok(PriceResponse::Equity(summaries)) => {
            for summary in summaries {
                append_equity_summary(&mut fetch.quotes, &mut fetch.errors, summary);
            }
        }
        Ok(PriceResponse::Crypto(_)) => {
            fetch
                .errors
                .push("equity price refresh returned crypto data".to_string());
        }
        Err(error) => fetch
            .errors
            .push(format!("equity price refresh failed: {error}")),
    }
    fetch
}

async fn fetch_crypto_quotes(
    runtime: &MarketRuntime,
    symbols: Vec<MarketSymbol>,
    provider: CryptoProvider,
) -> QuoteFetchResult {
    if symbols.is_empty() {
        return QuoteFetchResult::default();
    }
    let aliases_by_symbol = aliases_by_canonical_key(&symbols);

    let client = match runtime.client() {
        Ok(client) => client,
        Err(error) => {
            return QuoteFetchResult {
                quotes: Vec::new(),
                errors: vec![format!("crypto price refresh failed: {error}")],
            };
        }
    };
    let config = runtime.public_binance_config();
    let batch = fetch_price_batch(
        &client,
        &config,
        provider,
        resolve_instrument(CryptoInstrument::Auto, CryptoCapability::Quote),
        symbols.into_iter().map(|symbol| symbol.input).collect(),
        runtime.timezone(),
    )
    .await;

    let mut fetch = QuoteFetchResult::default();
    append_crypto_points(
        &mut fetch.quotes,
        &mut fetch.errors,
        batch.points,
        batch.errors,
    );
    fetch.with_aliases(aliases_by_symbol)
}

fn append_equity_summary(
    quotes: &mut Vec<QuoteSnapshot>,
    errors: &mut Vec<String>,
    summary: PriceSummary,
) {
    for (provider, error) in &summary.errors {
        errors.push(format!("{} {}: {}", summary.symbol, provider, error));
    }

    quotes.push(quote_from_price_summary(summary));
}

fn append_crypto_points(
    quotes: &mut Vec<QuoteSnapshot>,
    errors: &mut Vec<String>,
    points: Vec<PricePoint>,
    provider_errors: BTreeMap<String, String>,
) {
    errors.extend(
        provider_errors
            .into_iter()
            .map(|(provider, error)| format!("{provider}: {error}")),
    );
    quotes.extend(points.into_iter().map(quote_from_price_point));
}

fn quote_from_price_summary(summary: PriceSummary) -> QuoteSnapshot {
    let PriceSummary {
        symbol,
        current,
        regular_basis,
        ..
    } = summary;
    match current {
        Some(point) => quote_from_point_and_basis(symbol, point, regular_basis),
        None => QuoteSnapshot {
            symbol,
            price: None,
            currency: None,
            provider: "unavailable".to_string(),
            session: None,
            market_time_local: None,
            change_pct: None,
            regular_basis: regular_basis.into(),
            aliases: Vec::new(),
        },
    }
}

fn quote_from_price_point(point: PricePoint) -> QuoteSnapshot {
    let symbol = point.symbol.clone();
    quote_from_point_and_basis(symbol, point, empty_regular_basis())
}

fn quote_from_point_and_basis(
    symbol: String,
    point: PricePoint,
    regular_basis: RegularBasis,
) -> QuoteSnapshot {
    QuoteSnapshot {
        symbol,
        price: point.price,
        currency: point.currency,
        provider: point.provider,
        session: point.session,
        market_time_local: point.market_time_local.or(point.market_time_utc),
        change_pct: point.change_pct,
        regular_basis: regular_basis.into(),
        aliases: Vec::new(),
    }
}

fn aliases_by_canonical_key(symbols: &[MarketSymbol]) -> BTreeMap<String, Vec<String>> {
    let mut aliases = BTreeMap::<String, Vec<String>>::new();
    for symbol in symbols {
        aliases
            .entry(symbol.canonical_key.clone())
            .or_default()
            .push(symbol.input.clone());
    }
    aliases
}

impl From<RegularBasis> for RegularBasisSnapshot {
    fn from(value: RegularBasis) -> Self {
        Self {
            previous_close: value.previous_close,
            open: value.open,
            high: value.high,
            low: value.low,
            volume: value.volume,
        }
    }
}

fn empty_regular_basis() -> RegularBasis {
    RegularBasis {
        previous_close: None,
        open: None,
        high: None,
        low: None,
        volume: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_snapshot_preserves_symbol_lookup_contract() {
        let snapshot = MarketSnapshot {
            fetched_at_local: None,
            quotes: vec![QuoteSnapshot {
                symbol: "CRDO".to_string(),
                price: Some(250.0),
                currency: Some("USD".to_string()),
                provider: "yahoo".to_string(),
                session: Some("regular".to_string()),
                market_time_local: None,
                change_pct: Some(1.2),
                aliases: vec!["CRDO.US".to_string()],
                regular_basis: RegularBasisSnapshot {
                    previous_close: Some(247.0),
                    open: None,
                    high: None,
                    low: None,
                    volume: None,
                },
            }],
            errors: Vec::new(),
        };

        assert_eq!(
            snapshot.quote_for("CRDO").and_then(|quote| quote.price),
            Some(250.0)
        );
        assert_eq!(
            snapshot.quote_for("CRDO.US").and_then(|quote| quote.price),
            Some(250.0)
        );
        assert!(snapshot.quote_for("AAPL").is_none());
    }

    #[test]
    fn crypto_aliases_preserve_requested_pair_spellings_after_provider_normalization() {
        let fetch = QuoteFetchResult {
            quotes: vec![QuoteSnapshot {
                symbol: "BTCUSDT".to_string(),
                price: Some(60_000.0),
                currency: Some("USDT".to_string()),
                provider: "binance".to_string(),
                session: None,
                market_time_local: None,
                change_pct: None,
                aliases: Vec::new(),
                regular_basis: RegularBasisSnapshot {
                    previous_close: None,
                    open: None,
                    high: None,
                    low: None,
                    volume: None,
                },
            }],
            errors: Vec::new(),
        }
        .with_aliases(aliases_by_canonical_key(&[
            MarketSymbol::new("BTC/USDT".to_string()),
            MarketSymbol::new("BTC_USDT".to_string()),
            MarketSymbol::new("BTC:USDT".to_string()),
        ]));

        let snapshot = MarketSnapshot {
            fetched_at_local: Some("2026-06-25T09:30:00+08:00".to_string()),
            quotes: fetch.quotes,
            errors: fetch.errors,
        };

        assert_eq!(
            snapshot.quote_for("BTC/USDT").and_then(|quote| quote.price),
            Some(60_000.0)
        );
        assert_eq!(
            snapshot.quote_for("BTC_USDT").and_then(|quote| quote.price),
            Some(60_000.0)
        );
        assert_eq!(
            snapshot.quote_for("BTC:USDT").and_then(|quote| quote.price),
            Some(60_000.0)
        );
    }
}
