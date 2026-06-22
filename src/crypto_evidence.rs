use anyhow::{Result, anyhow};
use serde::Serialize;

use crate::cli::{CryptoDiscoverKind, CryptoInstrument, CryptoProvider};
use crate::crypto_capabilities::{
    CryptoCapability, binance_market, provider_supports, resolve_instrument, selected_providers,
};
use crate::http::http_client;
use crate::providers::{binance, coinbase, coingecko, okx};

pub async fn run_quote(
    args: crate::cli::CryptoEvidenceSymbolArgs,
    proxy: Option<&str>,
    no_proxy: bool,
    timeout_seconds: u64,
) -> Result<()> {
    let client = http_client(timeout_seconds, proxy, no_proxy)?;
    let config = binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy);
    let capability = CryptoCapability::Quote;
    let instrument = resolve_instrument(args.instrument, capability);
    let mut results = Vec::new();
    for provider in selected_providers(args.provider, instrument, capability) {
        if !provider_supports(provider, instrument, capability) {
            results.push(unsupported_provider(
                provider.label(),
                capability,
                instrument,
            ));
            continue;
        }
        results.push(match provider {
            CryptoProvider::Binance => provider_from_endpoints(
                "binance",
                vec![required_payload(
                    "quote",
                    binance::fetch_quote(&config, binance_market(instrument), &args.symbol).await,
                )],
            ),
            CryptoProvider::Coinbase => provider_from_endpoints(
                "coinbase",
                vec![
                    required_value("ticker", coinbase::ticker(&client, &args.symbol).await),
                    supplemental_value("stats", coinbase::stats(&client, &args.symbol).await),
                    supplemental_value("product", coinbase::product(&client, &args.symbol).await),
                ],
            ),
            CryptoProvider::Okx => provider_from_endpoints(
                "okx",
                vec![required_value(
                    "ticker",
                    okx::ticker(&client, &args.symbol, instrument).await,
                )],
            ),
            CryptoProvider::Coingecko => provider_from_endpoints(
                "coingecko",
                vec![
                    required_value(
                        "simple-price",
                        coingecko::simple_price(&client, &args.symbol).await,
                    ),
                    supplemental_value("coin", coingecko::coin(&client, &args.symbol).await),
                ],
            ),
            CryptoProvider::Auto => unreachable!(),
        });
    }
    print_evidence_report(
        evidence_report(capability, instrument, Some(&args.symbol), results),
        args.json,
        args.raw,
    )
}

pub async fn run_book(
    args: crate::cli::CryptoEvidenceBookArgs,
    proxy: Option<&str>,
    no_proxy: bool,
    timeout_seconds: u64,
) -> Result<()> {
    let client = http_client(timeout_seconds, proxy, no_proxy)?;
    let config = binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy);
    let capability = CryptoCapability::Book;
    let instrument = resolve_instrument(args.instrument, capability);
    let mut results = Vec::new();
    for provider in selected_providers(args.provider, instrument, capability) {
        if !provider_supports(provider, instrument, capability) {
            results.push(unsupported_provider(
                provider.label(),
                capability,
                instrument,
            ));
            continue;
        }
        results.push(match provider {
            CryptoProvider::Binance => provider_from_endpoints(
                "binance",
                vec![match instrument {
                    CryptoInstrument::Swap | CryptoInstrument::Futures => required_payload(
                        "book",
                        binance::futures_book(&config, &args.symbol, args.limit).await,
                    ),
                    _ => required_payload(
                        "book",
                        binance::spot_book(&config, &args.symbol, args.limit).await,
                    ),
                }],
            ),
            CryptoProvider::Coinbase => provider_from_endpoints(
                "coinbase",
                vec![required_value(
                    "book",
                    coinbase::book(&client, &args.symbol, args.limit).await,
                )],
            ),
            CryptoProvider::Okx => provider_from_endpoints(
                "okx",
                vec![required_value(
                    "book",
                    okx::book(&client, &args.symbol, instrument, args.limit).await,
                )],
            ),
            CryptoProvider::Coingecko => unsupported_provider("coingecko", capability, instrument),
            CryptoProvider::Auto => unreachable!(),
        });
    }
    print_evidence_report(
        evidence_report(capability, instrument, Some(&args.symbol), results),
        args.json,
        args.raw,
    )
}

pub async fn run_trades(
    args: crate::cli::CryptoEvidenceTradesArgs,
    proxy: Option<&str>,
    no_proxy: bool,
    timeout_seconds: u64,
) -> Result<()> {
    let client = http_client(timeout_seconds, proxy, no_proxy)?;
    let config = binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy);
    let capability = CryptoCapability::Trades;
    let instrument = resolve_instrument(args.instrument, capability);
    let mut results = Vec::new();
    for provider in selected_providers(args.provider, instrument, capability) {
        if !provider_supports(provider, instrument, capability) {
            results.push(unsupported_provider(
                provider.label(),
                capability,
                instrument,
            ));
            continue;
        }
        results.push(match provider {
            CryptoProvider::Binance => provider_from_endpoints(
                "binance",
                vec![match instrument {
                    CryptoInstrument::Swap | CryptoInstrument::Futures => required_payload(
                        "trades",
                        binance::futures_trades(&config, &args.symbol, args.limit).await,
                    ),
                    _ => required_payload(
                        "trades",
                        binance::spot_trades(&config, &args.symbol, args.limit, args.aggregate)
                            .await,
                    ),
                }],
            ),
            CryptoProvider::Coinbase => provider_from_endpoints(
                "coinbase",
                vec![required_value(
                    "trades",
                    coinbase::trades(&client, &args.symbol, args.limit).await,
                )],
            ),
            CryptoProvider::Okx => provider_from_endpoints(
                "okx",
                vec![required_value(
                    "trades",
                    okx::trades(&client, &args.symbol, instrument, args.limit).await,
                )],
            ),
            CryptoProvider::Coingecko => unreachable!("unsupported provider handled earlier"),
            CryptoProvider::Auto => unreachable!(),
        });
    }
    print_evidence_report(
        evidence_report(capability, instrument, Some(&args.symbol), results),
        args.json,
        args.raw,
    )
}

pub async fn run_candles(
    args: crate::cli::CryptoEvidenceKlinesArgs,
    proxy: Option<&str>,
    no_proxy: bool,
    timeout_seconds: u64,
) -> Result<()> {
    let client = http_client(timeout_seconds, proxy, no_proxy)?;
    let config = binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy);
    let capability = CryptoCapability::Candles;
    let instrument = resolve_instrument(args.instrument, capability);
    let mut results = Vec::new();
    for provider in selected_providers(args.provider, instrument, capability) {
        if !provider_supports(provider, instrument, capability) {
            results.push(unsupported_provider(
                provider.label(),
                capability,
                instrument,
            ));
            continue;
        }
        results.push(match provider {
            CryptoProvider::Binance => provider_from_endpoints(
                "binance",
                vec![match instrument {
                    CryptoInstrument::Swap | CryptoInstrument::Futures => required_payload(
                        "klines",
                        binance::futures_klines(&config, &args.symbol, &args.interval, args.limit)
                            .await,
                    ),
                    _ => required_payload(
                        "klines",
                        binance::spot_klines(&config, &args.symbol, &args.interval, args.limit)
                            .await,
                    ),
                }],
            ),
            CryptoProvider::Coinbase => provider_from_endpoints(
                "coinbase",
                vec![required_value(
                    "candles",
                    coinbase::candles(&client, &args.symbol, &args.interval, args.limit).await,
                )],
            ),
            CryptoProvider::Okx => provider_from_endpoints(
                "okx",
                vec![
                    required_value(
                        "candles",
                        okx::candles(
                            &client,
                            &args.symbol,
                            instrument,
                            &args.interval,
                            args.limit,
                        )
                        .await,
                    ),
                    supplemental_value(
                        "history-candles",
                        okx::history_candles(
                            &client,
                            &args.symbol,
                            instrument,
                            &args.interval,
                            args.limit,
                        )
                        .await,
                    ),
                ],
            ),
            CryptoProvider::Coingecko => provider_from_endpoints(
                "coingecko",
                vec![
                    required_value(
                        "ohlc",
                        coingecko::ohlc(&client, &args.symbol, &args.interval, args.limit).await,
                    ),
                    supplemental_value(
                        "market-chart",
                        coingecko::market_chart(&client, &args.symbol, "1", args.limit).await,
                    ),
                ],
            ),
            CryptoProvider::Auto => unreachable!(),
        });
    }
    print_evidence_report(
        evidence_report(capability, instrument, Some(&args.symbol), results),
        args.json,
        args.raw,
    )
}

pub async fn run_funding(
    args: crate::cli::CryptoEvidenceFundingArgs,
    proxy: Option<&str>,
    no_proxy: bool,
    timeout_seconds: u64,
) -> Result<()> {
    let client = http_client(timeout_seconds, proxy, no_proxy)?;
    let config = binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy);
    let capability = CryptoCapability::Funding;
    let instrument = resolve_instrument(args.instrument, capability);
    let mut results = Vec::new();
    for provider in selected_providers(args.provider, instrument, capability) {
        if !provider_supports(provider, instrument, capability) {
            results.push(unsupported_provider(
                provider.label(),
                capability,
                instrument,
            ));
            continue;
        }
        results.push(match provider {
            CryptoProvider::Binance => provider_from_endpoints(
                "binance",
                vec![required_payload(
                    "funding",
                    binance::futures_funding(&config, &args.symbol, args.limit).await,
                )],
            ),
            CryptoProvider::Okx => provider_from_endpoints(
                "okx",
                vec![
                    required_value(
                        "funding-rate",
                        okx::funding_rate(&client, &args.symbol, instrument).await,
                    ),
                    supplemental_value(
                        "funding-rate-history",
                        okx::funding_rate_history(&client, &args.symbol, instrument, args.limit)
                            .await,
                    ),
                ],
            ),
            provider => unsupported_provider(provider.label(), capability, instrument),
        });
    }
    print_evidence_report(
        evidence_report(capability, instrument, Some(&args.symbol), results),
        args.json,
        args.raw,
    )
}

pub async fn run_open_interest(
    args: crate::cli::CryptoEvidenceOpenInterestArgs,
    proxy: Option<&str>,
    no_proxy: bool,
    timeout_seconds: u64,
) -> Result<()> {
    let client = http_client(timeout_seconds, proxy, no_proxy)?;
    let config = binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy);
    let capability = CryptoCapability::OpenInterest;
    let instrument = resolve_instrument(args.instrument, capability);
    let mut results = Vec::new();
    for provider in selected_providers(args.provider, instrument, capability) {
        if !provider_supports(provider, instrument, capability) {
            results.push(unsupported_provider(
                provider.label(),
                capability,
                instrument,
            ));
            continue;
        }
        results.push(match provider {
            CryptoProvider::Binance => provider_from_endpoints(
                "binance",
                vec![required_payload(
                    "open-interest",
                    binance::futures_open_interest(&config, &args.symbol).await,
                )],
            ),
            CryptoProvider::Okx => provider_from_endpoints(
                "okx",
                vec![
                    required_value(
                        "open-interest",
                        okx::open_interest(&client, &args.symbol, instrument).await,
                    ),
                    supplemental_value(
                        "mark-price",
                        okx::mark_price(&client, &args.symbol, instrument).await,
                    ),
                ],
            ),
            provider => unsupported_provider(provider.label(), capability, instrument),
        });
    }
    print_evidence_report(
        evidence_report(capability, instrument, Some(&args.symbol), results),
        args.json,
        args.raw,
    )
}

pub async fn run_discover(
    args: crate::cli::CryptoDiscoverArgs,
    proxy: Option<&str>,
    no_proxy: bool,
    timeout_seconds: u64,
) -> Result<()> {
    let client = http_client(timeout_seconds, proxy, no_proxy)?;
    let config = binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy);
    let capability = CryptoCapability::Discover(args.kind);
    let instrument = resolve_instrument(args.instrument, capability);
    let mut results = Vec::new();
    for provider in selected_providers(args.provider, instrument, capability) {
        if !provider_supports(provider, instrument, capability) {
            results.push(unsupported_provider(
                provider.label(),
                capability,
                instrument,
            ));
            continue;
        }
        results.push(match (provider, args.kind) {
            (
                CryptoProvider::Binance,
                CryptoDiscoverKind::Markets | CryptoDiscoverKind::Instruments,
            ) => provider_from_endpoints(
                "binance",
                vec![required_payload(
                    "exchange-info",
                    binance::spot_exchange_info(&config, None).await,
                )],
            ),
            (
                CryptoProvider::Coinbase,
                CryptoDiscoverKind::Markets | CryptoDiscoverKind::Instruments,
            ) => provider_from_endpoints(
                "coinbase",
                vec![required_value(
                    "products",
                    coinbase::products(&client).await,
                )],
            ),
            (CryptoProvider::Coinbase, CryptoDiscoverKind::VolumeSummary) => {
                provider_from_endpoints(
                    "coinbase",
                    vec![required_value(
                        "volume-summary",
                        coinbase::volume_summary(&client).await,
                    )],
                )
            }
            (
                CryptoProvider::Okx,
                CryptoDiscoverKind::Markets | CryptoDiscoverKind::Instruments,
            ) => provider_from_endpoints(
                "okx",
                vec![required_value(
                    "instruments",
                    okx::instruments(&client, instrument).await,
                )],
            ),
            (CryptoProvider::Okx, CryptoDiscoverKind::Tickers) => provider_from_endpoints(
                "okx",
                vec![required_value(
                    "tickers",
                    okx::tickers(&client, instrument).await,
                )],
            ),
            (CryptoProvider::Coingecko, CryptoDiscoverKind::Markets) => provider_from_endpoints(
                "coingecko",
                vec![required_value(
                    "markets",
                    coingecko::markets(&client, &args.vs_currency, args.limit).await,
                )],
            ),
            (CryptoProvider::Coingecko, CryptoDiscoverKind::Trending) => provider_from_endpoints(
                "coingecko",
                vec![required_value(
                    "trending",
                    coingecko::trending(&client).await,
                )],
            ),
            (CryptoProvider::Coingecko, CryptoDiscoverKind::Global) => provider_from_endpoints(
                "coingecko",
                vec![required_value("global", coingecko::global(&client).await)],
            ),
            (CryptoProvider::Coingecko, CryptoDiscoverKind::Exchanges) => provider_from_endpoints(
                "coingecko",
                vec![required_value(
                    "exchanges",
                    coingecko::exchanges(&client, args.limit).await,
                )],
            ),
            (CryptoProvider::Coingecko, CryptoDiscoverKind::Derivatives) => {
                provider_from_endpoints(
                    "coingecko",
                    vec![required_value(
                        "derivatives",
                        coingecko::derivatives(&client, args.limit).await,
                    )],
                )
            }
            (CryptoProvider::Coingecko, CryptoDiscoverKind::DerivativesExchanges) => {
                provider_from_endpoints(
                    "coingecko",
                    vec![required_value(
                        "derivatives-exchanges",
                        coingecko::derivatives_exchanges(&client, args.limit).await,
                    )],
                )
            }
            (CryptoProvider::Coingecko, CryptoDiscoverKind::CoinsList) => provider_from_endpoints(
                "coingecko",
                vec![required_value(
                    "coins-list",
                    coingecko::coins_list(&client, args.limit).await,
                )],
            ),
            (provider, _) => unsupported_provider(provider.label(), capability, instrument),
        });
    }
    print_evidence_report(
        evidence_report(capability, instrument, None, results),
        args.json,
        args.raw,
    )
}

fn evidence_report(
    capability: CryptoCapability,
    instrument: CryptoInstrument,
    symbol: Option<&str>,
    results: Vec<ProviderEvidence>,
) -> CryptoEvidenceReport {
    CryptoEvidenceReport {
        capability: capability.label().to_string(),
        instrument: instrument.label().to_string(),
        symbol: symbol.map(ToString::to_string),
        fetched_at_utc: crate::http::utc_now(),
        results,
    }
}

fn required_payload<T: Serialize>(endpoint: &str, result: Result<T>) -> EndpointEvidence {
    match result {
        Ok(payload) => required_value(
            endpoint,
            serde_json::to_value(payload).map_err(anyhow::Error::from),
        ),
        Err(error) => EndpointEvidence::error(endpoint, true, format!("{error:#}")),
    }
}

fn required_value(endpoint: &str, result: Result<serde_json::Value>) -> EndpointEvidence {
    endpoint_value(endpoint, true, result)
}

fn supplemental_value(endpoint: &str, result: Result<serde_json::Value>) -> EndpointEvidence {
    endpoint_value(endpoint, false, result)
}

fn endpoint_value(
    endpoint: &str,
    required: bool,
    result: Result<serde_json::Value>,
) -> EndpointEvidence {
    match result {
        Ok(payload) => EndpointEvidence {
            endpoint: endpoint.to_string(),
            required,
            ok: true,
            error: None,
            payload: Some(payload),
        },
        Err(error) => EndpointEvidence::error(endpoint, required, format!("{error:#}")),
    }
}

fn provider_from_endpoints(provider: &str, endpoints: Vec<EndpointEvidence>) -> ProviderEvidence {
    let required_endpoints = endpoints
        .iter()
        .filter(|endpoint| endpoint.required)
        .collect::<Vec<_>>();
    let ok = if required_endpoints.is_empty() {
        endpoints.iter().any(|endpoint| endpoint.ok)
    } else {
        required_endpoints.iter().all(|endpoint| endpoint.ok)
    };
    ProviderEvidence {
        provider: provider.to_string(),
        ok,
        endpoints,
    }
}

fn unsupported_provider(
    provider: &str,
    capability: CryptoCapability,
    instrument: CryptoInstrument,
) -> ProviderEvidence {
    provider_from_endpoints(
        provider,
        vec![EndpointEvidence::error(
            capability.label(),
            true,
            format!(
                "provider does not support capability={} instrument={}",
                capability.label(),
                instrument.label()
            ),
        )],
    )
}

fn print_evidence_report(report: CryptoEvidenceReport, json: bool, raw: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "crypto {} instrument={} symbol={} fetched={}",
            report.capability,
            report.instrument,
            report.symbol.as_deref().unwrap_or("-"),
            report.fetched_at_utc
        );
        for result in &report.results {
            println!(
                "{} {}",
                result.provider,
                if result.ok { "ok" } else { "error" }
            );
            for endpoint in &result.endpoints {
                println!(
                    "  {} {}",
                    endpoint.endpoint,
                    if endpoint.ok { "ok" } else { "error" }
                );
                if let Some(error) = endpoint.error.as_deref() {
                    println!("    {error}");
                } else if let Some(payload) = endpoint.payload.as_ref() {
                    if raw {
                        println!("{}", serde_json::to_string_pretty(payload)?);
                    } else {
                        println!("    payload: {}", payload_summary(payload));
                    }
                }
            }
        }
    }
    if report.results.iter().any(|result| result.ok) {
        Ok(())
    } else {
        Err(anyhow!(
            "no provider returned crypto {} evidence for instrument={}",
            report.capability,
            report.instrument
        ))
    }
}

fn payload_summary(payload: &serde_json::Value) -> String {
    match payload {
        serde_json::Value::Array(rows) => format!("array rows={}", rows.len()),
        serde_json::Value::Object(fields) => format!("object fields={}", fields.len()),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(_) => "bool".to_string(),
        serde_json::Value::Number(_) => "number".to_string(),
        serde_json::Value::String(value) => format!("string chars={}", value.chars().count()),
    }
}

#[derive(Debug, Serialize)]
struct CryptoEvidenceReport {
    capability: String,
    instrument: String,
    symbol: Option<String>,
    fetched_at_utc: String,
    results: Vec<ProviderEvidence>,
}

#[derive(Debug, Serialize)]
struct ProviderEvidence {
    provider: String,
    ok: bool,
    endpoints: Vec<EndpointEvidence>,
}

#[derive(Debug, Serialize)]
struct EndpointEvidence {
    endpoint: String,
    required: bool,
    ok: bool,
    error: Option<String>,
    payload: Option<serde_json::Value>,
}

impl EndpointEvidence {
    fn error(endpoint: &str, required: bool, error: String) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            required,
            ok: false,
            error: Some(error),
            payload: None,
        }
    }
}
