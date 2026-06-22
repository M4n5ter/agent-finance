use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Result, anyhow};
use serde::Serialize;

use crate::cli::{
    CryptoInstrument, CryptoProvider, HistoryArgs, IndicatorsArgs, PriceArgs, Provider, WatchArgs,
};
use crate::crypto_capabilities::{
    CryptoCapability, binance_market, provider_supports, resolve_instrument, selected_providers,
};
use crate::http::http_client;
use crate::indicators::compute_indicator;
use crate::model::{DerivedIndicator, HistoryBatch, PricePoint, Quote};
use crate::output;
use crate::price;
use crate::providers::{binance, coinbase, coingecko, okx};

pub async fn run_price(
    args: PriceArgs,
    proxy: Option<&str>,
    no_proxy: bool,
    timeout_seconds: u64,
    timezone: &str,
) -> Result<()> {
    let client = http_client(timeout_seconds, proxy, no_proxy)?;
    let config = binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy);
    let batch = fetch_price_batch(
        &client,
        &config,
        args.crypto_provider,
        resolve_instrument(args.instrument, CryptoCapability::Quote),
        args.symbols,
        timezone,
    )
    .await;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&batch)?);
    } else {
        output::print_crypto_price_points(&batch.points, &batch.errors);
    }
    if batch.errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow!("one or more crypto price quotes failed"))
    }
}

pub async fn run_history(
    args: HistoryArgs,
    proxy: Option<&str>,
    no_proxy: bool,
    timeout_seconds: u64,
    timezone: &str,
) -> Result<()> {
    let client = http_client(timeout_seconds, proxy, no_proxy)?;
    let config = binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy);
    let history = fetch_history(
        &client,
        &config,
        provider_crypto_provider(args.provider, args.crypto_provider),
        provider_instrument(args.provider, args.instrument),
        &args.symbol,
        &args.interval,
        args.limit,
    )
    .await?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&history)?);
    } else {
        output::print_history_table(&history, timezone);
    }
    Ok(())
}

pub async fn run_indicators(
    args: IndicatorsArgs,
    proxy: Option<&str>,
    no_proxy: bool,
    timeout_seconds: u64,
) -> Result<()> {
    let client = http_client(timeout_seconds, proxy, no_proxy)?;
    let config = binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy);
    let provider = provider_crypto_provider(args.provider, args.crypto_provider);
    let instrument = provider_instrument(args.provider, args.instrument);
    let mut indicators = Vec::new();
    let mut errors = BTreeMap::new();
    for symbol in args.symbols {
        match fetch_history(
            &client,
            &config,
            provider,
            instrument,
            &symbol,
            &args.interval,
            args.limit,
        )
        .await
        {
            Ok(history) => indicators.push(compute_indicator(&history)),
            Err(error) => {
                errors.insert(symbol, format!("{error:#}"));
            }
        }
    }
    let batch = IndicatorBatch { indicators, errors };
    if args.json {
        println!("{}", serde_json::to_string_pretty(&batch)?);
    } else {
        output::print_indicator_table(&batch.indicators, &batch.errors);
    }
    if batch.errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow!("one or more crypto indicators failed"))
    }
}

pub async fn run_watch(
    args: WatchArgs,
    proxy: Option<&str>,
    no_proxy: bool,
    timeout_seconds: u64,
    timezone: &str,
) -> Result<()> {
    let client = http_client(timeout_seconds, proxy, no_proxy)?;
    let config = binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy);
    let mut iteration = 0usize;
    let mut had_errors = false;
    loop {
        iteration += 1;
        let batch = fetch_price_batch(
            &client,
            &config,
            args.crypto_provider,
            resolve_instrument(args.instrument, CryptoCapability::Quote),
            args.symbols.clone(),
            timezone,
        )
        .await;
        had_errors |= !batch.errors.is_empty();
        if args.json {
            println!("{}", serde_json::to_string_pretty(&batch)?);
        } else {
            output::print_crypto_price_points(&batch.points, &batch.errors);
            println!();
        }
        if args.iterations != 0 && iteration >= args.iterations {
            break;
        }
        tokio::time::sleep(Duration::from_secs(args.interval_seconds.max(1))).await;
    }
    if had_errors {
        Err(anyhow!("one or more crypto watch quotes failed"))
    } else {
        Ok(())
    }
}

async fn fetch_price_batch(
    client: &wreq::Client,
    config: &binance::BinanceConfig,
    provider: CryptoProvider,
    instrument: CryptoInstrument,
    symbols: Vec<String>,
    timezone: &str,
) -> CryptoPriceBatch {
    let mut points = Vec::new();
    let mut errors = BTreeMap::new();
    for symbol in symbols {
        match fetch_quote(client, config, provider, instrument, &symbol).await {
            Ok(quote) => points.push(price::quote_to_point(
                quote,
                "Crypto price",
                timezone,
                Some("Crypto markets trade 24/7; this is not an equity session quote".to_string()),
            )),
            Err(error) => {
                errors.insert(symbol, format!("{error:#}"));
            }
        }
    }
    CryptoPriceBatch { points, errors }
}

async fn fetch_quote(
    client: &wreq::Client,
    config: &binance::BinanceConfig,
    provider: CryptoProvider,
    instrument: CryptoInstrument,
    symbol: &str,
) -> Result<Quote> {
    if !provider_supports(provider, instrument, CryptoCapability::Quote)
        && provider != CryptoProvider::Auto
    {
        return Err(anyhow!(
            "provider {} does not support capability=quote instrument={}",
            provider.label(),
            instrument.label()
        ));
    }
    match provider {
        CryptoProvider::Auto => {
            let mut errors = Vec::new();
            for provider in selected_providers(provider, instrument, CryptoCapability::Quote) {
                match fetch_quote_one(client, config, provider, instrument, symbol).await {
                    Ok(quote) => return Ok(quote),
                    Err(error) => errors.push(format!("{}: {error:#}", provider.label())),
                }
            }
            Err(anyhow!("{}", errors.join("; ")))
        }
        provider => fetch_quote_one(client, config, provider, instrument, symbol).await,
    }
}

async fn fetch_quote_one(
    client: &wreq::Client,
    config: &binance::BinanceConfig,
    provider: CryptoProvider,
    instrument: CryptoInstrument,
    symbol: &str,
) -> Result<Quote> {
    match provider {
        CryptoProvider::Binance => {
            binance::fetch_quote(config, binance_market(instrument), symbol).await
        }
        CryptoProvider::Coinbase => coinbase::fetch_quote(client, symbol).await,
        CryptoProvider::Okx => okx::fetch_quote(client, symbol, instrument).await,
        CryptoProvider::Coingecko => coingecko::fetch_quote(client, symbol).await,
        CryptoProvider::Auto => unreachable!("auto must be expanded before provider dispatch"),
    }
}

async fn fetch_history(
    client: &wreq::Client,
    config: &binance::BinanceConfig,
    provider: CryptoProvider,
    instrument: CryptoInstrument,
    symbol: &str,
    interval: &str,
    limit: usize,
) -> Result<HistoryBatch> {
    if !provider_supports(provider, instrument, CryptoCapability::Candles)
        && provider != CryptoProvider::Auto
    {
        return Err(anyhow!(
            "provider {} does not support capability=candles instrument={}",
            provider.label(),
            instrument.label()
        ));
    }
    match provider {
        CryptoProvider::Auto => {
            let mut errors = Vec::new();
            for provider in selected_providers(provider, instrument, CryptoCapability::Candles) {
                match fetch_history_one(
                    client, config, provider, instrument, symbol, interval, limit,
                )
                .await
                {
                    Ok(history) => return Ok(history),
                    Err(error) => errors.push(format!("{}: {error:#}", provider.label())),
                }
            }
            Err(anyhow!("{}", errors.join("; ")))
        }
        provider => {
            fetch_history_one(
                client, config, provider, instrument, symbol, interval, limit,
            )
            .await
        }
    }
}

async fn fetch_history_one(
    client: &wreq::Client,
    config: &binance::BinanceConfig,
    provider: CryptoProvider,
    instrument: CryptoInstrument,
    symbol: &str,
    interval: &str,
    limit: usize,
) -> Result<HistoryBatch> {
    match provider {
        CryptoProvider::Binance => {
            binance::fetch_history(config, binance_market(instrument), symbol, interval, limit)
                .await
        }
        CryptoProvider::Coinbase => coinbase::fetch_history(client, symbol, interval, limit).await,
        CryptoProvider::Okx => {
            okx::fetch_history(client, symbol, instrument, interval, limit).await
        }
        CryptoProvider::Coingecko => {
            coingecko::fetch_history(client, symbol, interval, limit).await
        }
        CryptoProvider::Auto => unreachable!("auto must be expanded before provider dispatch"),
    }
}

fn provider_instrument(provider: Provider, instrument: CryptoInstrument) -> CryptoInstrument {
    match provider {
        Provider::BinanceSpot => CryptoInstrument::Spot,
        Provider::BinanceUsdsFutures => CryptoInstrument::Swap,
        _ => resolve_instrument(instrument, CryptoCapability::Candles),
    }
}

fn provider_crypto_provider(provider: Provider, crypto_provider: CryptoProvider) -> CryptoProvider {
    match provider {
        Provider::BinanceSpot | Provider::BinanceUsdsFutures => CryptoProvider::Binance,
        _ => crypto_provider,
    }
}

#[derive(Debug, Serialize)]
struct CryptoPriceBatch {
    points: Vec<PricePoint>,
    errors: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
struct IndicatorBatch {
    indicators: Vec<DerivedIndicator>,
    errors: BTreeMap<String, String>,
}
