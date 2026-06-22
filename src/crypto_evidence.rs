use std::future::Future;

use anyhow::{Result, anyhow};
use futures_util::future::{BoxFuture, FutureExt, join_all};
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
    let symbol = args.symbol;
    let results = collect_provider_evidence(args.provider, instrument, capability, |provider| {
        let client = client.clone();
        let config = config.clone();
        let symbol = symbol.clone();
        async move {
            match provider {
                CryptoProvider::Binance => provider_from_endpoints(
                    "binance",
                    vec![required_payload(
                        "quote",
                        binance::fetch_quote(&config, binance_market(instrument), &symbol).await,
                    )],
                ),
                CryptoProvider::Coinbase => provider_from_endpoints(
                    "coinbase",
                    collect_endpoint_evidence(vec![
                        required_endpoint("ticker", {
                            let client = client.clone();
                            let symbol = symbol.clone();
                            async move { coinbase::ticker(&client, &symbol).await }
                        }),
                        supplemental_endpoint("stats", {
                            let client = client.clone();
                            let symbol = symbol.clone();
                            async move { coinbase::stats(&client, &symbol).await }
                        }),
                        supplemental_endpoint("product", {
                            let client = client.clone();
                            let symbol = symbol.clone();
                            async move { coinbase::product(&client, &symbol).await }
                        }),
                    ])
                    .await,
                ),
                CryptoProvider::Okx => provider_from_endpoints(
                    "okx",
                    vec![required_value(
                        "ticker",
                        okx::ticker(&client, &symbol, instrument).await,
                    )],
                ),
                CryptoProvider::Coingecko => provider_from_endpoints(
                    "coingecko",
                    collect_endpoint_evidence(vec![
                        required_endpoint("simple-price", {
                            let client = client.clone();
                            let symbol = symbol.clone();
                            async move { coingecko::simple_price(&client, &symbol).await }
                        }),
                        supplemental_endpoint("coin", {
                            let client = client.clone();
                            let symbol = symbol.clone();
                            async move { coingecko::coin(&client, &symbol).await }
                        }),
                    ])
                    .await,
                ),
                CryptoProvider::Auto => unreachable!(),
            }
        }
    })
    .await;
    print_evidence_report(
        evidence_report(capability, instrument, Some(&symbol), results),
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
    let symbol = args.symbol;
    let limit = args.limit;
    let results = collect_provider_evidence(args.provider, instrument, capability, |provider| {
        let client = client.clone();
        let config = config.clone();
        let symbol = symbol.clone();
        async move {
            match provider {
                CryptoProvider::Binance => provider_from_endpoints(
                    "binance",
                    vec![match instrument {
                        CryptoInstrument::Swap | CryptoInstrument::Futures => required_payload(
                            "book",
                            binance::futures_book(&config, &symbol, limit).await,
                        ),
                        _ => required_payload(
                            "book",
                            binance::spot_book(&config, &symbol, limit).await,
                        ),
                    }],
                ),
                CryptoProvider::Coinbase => provider_from_endpoints(
                    "coinbase",
                    vec![required_value(
                        "book",
                        coinbase::book(&client, &symbol, limit).await,
                    )],
                ),
                CryptoProvider::Okx => provider_from_endpoints(
                    "okx",
                    vec![required_value(
                        "book",
                        okx::book(&client, &symbol, instrument, limit).await,
                    )],
                ),
                CryptoProvider::Coingecko => {
                    unsupported_provider("coingecko", capability, instrument)
                }
                CryptoProvider::Auto => unreachable!(),
            }
        }
    })
    .await;
    print_evidence_report(
        evidence_report(capability, instrument, Some(&symbol), results),
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
    let symbol = args.symbol;
    let limit = args.limit;
    let aggregate = args.aggregate;
    let results = collect_provider_evidence(args.provider, instrument, capability, |provider| {
        let client = client.clone();
        let config = config.clone();
        let symbol = symbol.clone();
        async move {
            match provider {
                CryptoProvider::Binance => provider_from_endpoints(
                    "binance",
                    vec![match instrument {
                        CryptoInstrument::Swap | CryptoInstrument::Futures => required_payload(
                            "trades",
                            binance::futures_trades(&config, &symbol, limit).await,
                        ),
                        _ => required_payload(
                            "trades",
                            binance::spot_trades(&config, &symbol, limit, aggregate).await,
                        ),
                    }],
                ),
                CryptoProvider::Coinbase => provider_from_endpoints(
                    "coinbase",
                    vec![required_value(
                        "trades",
                        coinbase::trades(&client, &symbol, limit).await,
                    )],
                ),
                CryptoProvider::Okx => provider_from_endpoints(
                    "okx",
                    vec![required_value(
                        "trades",
                        okx::trades(&client, &symbol, instrument, limit).await,
                    )],
                ),
                CryptoProvider::Coingecko => unreachable!("unsupported provider handled earlier"),
                CryptoProvider::Auto => unreachable!(),
            }
        }
    })
    .await;
    print_evidence_report(
        evidence_report(capability, instrument, Some(&symbol), results),
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
    let symbol = args.symbol;
    let interval = args.interval;
    let limit = args.limit;
    let results = collect_provider_evidence(args.provider, instrument, capability, |provider| {
        let client = client.clone();
        let config = config.clone();
        let symbol = symbol.clone();
        let interval = interval.clone();
        async move {
            match provider {
                    CryptoProvider::Binance => provider_from_endpoints(
                        "binance",
                        vec![match instrument {
                            CryptoInstrument::Swap | CryptoInstrument::Futures => required_payload(
                                "klines",
                                binance::futures_klines(&config, &symbol, &interval, limit).await,
                            ),
                            _ => required_payload(
                                "klines",
                                binance::spot_klines(&config, &symbol, &interval, limit).await,
                            ),
                        }],
                    ),
                    CryptoProvider::Coinbase => provider_from_endpoints(
                        "coinbase",
                        vec![required_value(
                            "candles",
                            coinbase::candles(&client, &symbol, &interval, limit).await,
                        )],
                    ),
                    CryptoProvider::Okx => provider_from_endpoints(
                        "okx",
                        collect_endpoint_evidence(vec![
                            required_endpoint("candles", {
                                let client = client.clone();
                                let symbol = symbol.clone();
                                let interval = interval.clone();
                                async move {
                                    okx::candles(&client, &symbol, instrument, &interval, limit)
                                        .await
                                }
                            }),
                            supplemental_endpoint("history-candles", {
                                let client = client.clone();
                                let symbol = symbol.clone();
                                let interval = interval.clone();
                                async move {
                                    okx::history_candles(
                                        &client, &symbol, instrument, &interval, limit,
                                    )
                                    .await
                                }
                            }),
                        ])
                        .await,
                    ),
                    CryptoProvider::Coingecko => {
                        provider_from_endpoints(
                            "coingecko",
                            collect_endpoint_evidence(vec![
                                required_endpoint("ohlc", {
                                    let client = client.clone();
                                    let symbol = symbol.clone();
                                    let interval = interval.clone();
                                    async move {
                                        coingecko::ohlc(&client, &symbol, &interval, limit).await
                                    }
                                }),
                                supplemental_endpoint("market-chart", {
                                    let client = client.clone();
                                    let symbol = symbol.clone();
                                    async move {
                                        coingecko::market_chart(&client, &symbol, "1", limit).await
                                    }
                                }),
                            ])
                            .await,
                        )
                    }
                    CryptoProvider::Auto => unreachable!(),
                }
        }
    })
    .await;
    print_evidence_report(
        evidence_report(capability, instrument, Some(&symbol), results),
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
    let symbol = args.symbol;
    let limit = args.limit;
    let results = collect_provider_evidence(args.provider, instrument, capability, |provider| {
        let client = client.clone();
        let config = config.clone();
        let symbol = symbol.clone();
        async move {
            match provider {
                CryptoProvider::Binance => provider_from_endpoints(
                    "binance",
                    vec![required_payload(
                        "funding",
                        binance::futures_funding(&config, &symbol, limit).await,
                    )],
                ),
                CryptoProvider::Okx => provider_from_endpoints(
                    "okx",
                    collect_endpoint_evidence(vec![
                        required_endpoint("funding-rate", {
                            let client = client.clone();
                            let symbol = symbol.clone();
                            async move { okx::funding_rate(&client, &symbol, instrument).await }
                        }),
                        supplemental_endpoint("funding-rate-history", {
                            let client = client.clone();
                            let symbol = symbol.clone();
                            async move {
                                okx::funding_rate_history(&client, &symbol, instrument, limit).await
                            }
                        }),
                    ])
                    .await,
                ),
                provider => unsupported_provider(provider.label(), capability, instrument),
            }
        }
    })
    .await;
    print_evidence_report(
        evidence_report(capability, instrument, Some(&symbol), results),
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
    let symbol = args.symbol;
    let results = collect_provider_evidence(args.provider, instrument, capability, |provider| {
        let client = client.clone();
        let config = config.clone();
        let symbol = symbol.clone();
        async move {
            match provider {
                CryptoProvider::Binance => provider_from_endpoints(
                    "binance",
                    vec![required_payload(
                        "open-interest",
                        binance::futures_open_interest(&config, &symbol).await,
                    )],
                ),
                CryptoProvider::Okx => provider_from_endpoints(
                    "okx",
                    collect_endpoint_evidence(vec![
                        required_endpoint("open-interest", {
                            let client = client.clone();
                            let symbol = symbol.clone();
                            async move { okx::open_interest(&client, &symbol, instrument).await }
                        }),
                        supplemental_endpoint("mark-price", {
                            let client = client.clone();
                            let symbol = symbol.clone();
                            async move { okx::mark_price(&client, &symbol, instrument).await }
                        }),
                    ])
                    .await,
                ),
                provider => unsupported_provider(provider.label(), capability, instrument),
            }
        }
    })
    .await;
    print_evidence_report(
        evidence_report(capability, instrument, Some(&symbol), results),
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
    let kind = args.kind;
    let limit = args.limit;
    let vs_currency = args.vs_currency;
    let results = collect_provider_evidence(args.provider, instrument, capability, |provider| {
        let client = client.clone();
        let config = config.clone();
        let vs_currency = vs_currency.clone();
        async move {
            match (provider, kind) {
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
                (CryptoProvider::Coingecko, CryptoDiscoverKind::Markets) => {
                    provider_from_endpoints(
                        "coingecko",
                        vec![required_value(
                            "markets",
                            coingecko::markets(&client, &vs_currency, limit).await,
                        )],
                    )
                }
                (CryptoProvider::Coingecko, CryptoDiscoverKind::Trending) => {
                    provider_from_endpoints(
                        "coingecko",
                        vec![required_value(
                            "trending",
                            coingecko::trending(&client).await,
                        )],
                    )
                }
                (CryptoProvider::Coingecko, CryptoDiscoverKind::Global) => provider_from_endpoints(
                    "coingecko",
                    vec![required_value("global", coingecko::global(&client).await)],
                ),
                (CryptoProvider::Coingecko, CryptoDiscoverKind::Exchanges) => {
                    provider_from_endpoints(
                        "coingecko",
                        vec![required_value(
                            "exchanges",
                            coingecko::exchanges(&client, limit).await,
                        )],
                    )
                }
                (CryptoProvider::Coingecko, CryptoDiscoverKind::Derivatives) => {
                    provider_from_endpoints(
                        "coingecko",
                        vec![required_value(
                            "derivatives",
                            coingecko::derivatives(&client, limit).await,
                        )],
                    )
                }
                (CryptoProvider::Coingecko, CryptoDiscoverKind::DerivativesExchanges) => {
                    provider_from_endpoints(
                        "coingecko",
                        vec![required_value(
                            "derivatives-exchanges",
                            coingecko::derivatives_exchanges(&client, limit).await,
                        )],
                    )
                }
                (CryptoProvider::Coingecko, CryptoDiscoverKind::CoinsList) => {
                    provider_from_endpoints(
                        "coingecko",
                        vec![required_value(
                            "coins-list",
                            coingecko::coins_list(&client, limit).await,
                        )],
                    )
                }
                (provider, _) => unsupported_provider(provider.label(), capability, instrument),
            }
        }
    })
    .await;
    print_evidence_report(
        evidence_report(capability, instrument, None, results),
        args.json,
        args.raw,
    )
}

async fn collect_provider_evidence<F, Fut>(
    provider: CryptoProvider,
    instrument: CryptoInstrument,
    capability: CryptoCapability,
    fetch: F,
) -> Vec<ProviderEvidence>
where
    F: Fn(CryptoProvider) -> Fut,
    Fut: Future<Output = ProviderEvidence>,
{
    let futures = selected_providers(provider, instrument, capability)
        .into_iter()
        .map(|provider| {
            let fetch = &fetch;
            async move {
                if provider_supports(provider, instrument, capability) {
                    fetch(provider).await
                } else {
                    unsupported_provider(provider.label(), capability, instrument)
                }
            }
        });
    join_all(futures).await
}

async fn collect_endpoint_evidence(
    endpoints: Vec<BoxFuture<'static, EndpointEvidence>>,
) -> Vec<EndpointEvidence> {
    join_all(endpoints).await
}

fn required_endpoint<T, Fut>(
    endpoint: &'static str,
    result: Fut,
) -> BoxFuture<'static, EndpointEvidence>
where
    T: Serialize + Send + 'static,
    Fut: Future<Output = Result<T>> + Send + 'static,
{
    endpoint_future(endpoint, true, result)
}

fn supplemental_endpoint<T, Fut>(
    endpoint: &'static str,
    result: Fut,
) -> BoxFuture<'static, EndpointEvidence>
where
    T: Serialize + Send + 'static,
    Fut: Future<Output = Result<T>> + Send + 'static,
{
    endpoint_future(endpoint, false, result)
}

fn endpoint_future<T, Fut>(
    endpoint: &'static str,
    required: bool,
    result: Fut,
) -> BoxFuture<'static, EndpointEvidence>
where
    T: Serialize + Send + 'static,
    Fut: Future<Output = Result<T>> + Send + 'static,
{
    async move { endpoint_result(endpoint, required, result.await) }.boxed()
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
    endpoint_result(endpoint, true, result)
}

fn required_value(endpoint: &str, result: Result<serde_json::Value>) -> EndpointEvidence {
    endpoint_value(endpoint, true, result)
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

fn endpoint_result<T: Serialize>(
    endpoint: &str,
    required: bool,
    result: Result<T>,
) -> EndpointEvidence {
    match result {
        Ok(payload) => endpoint_value(
            endpoint,
            required,
            serde_json::to_value(payload).map_err(anyhow::Error::from),
        ),
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
