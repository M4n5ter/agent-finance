use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Result, anyhow};
use serde::Serialize;

use crate::cli::{
    CryptoArgs, CryptoCommand, CryptoFuturesCommand, CryptoSpotCommand, HistoryArgs,
    IndicatorsArgs, PriceArgs, Provider, WatchArgs,
};
use crate::indicators::compute_indicator;
use crate::model::{DerivedIndicator, PricePoint};
use crate::output;
use crate::price;
use crate::providers::binance;

pub async fn run(
    args: CryptoArgs,
    proxy: Option<&str>,
    no_proxy: bool,
    timeout_seconds: u64,
    timezone: &str,
) -> Result<()> {
    let config = binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy);
    match args.command {
        CryptoCommand::Snapshot(args) => {
            let report = binance::snapshot(&config, &args.symbol).await;
            let status = report.ensure_complete();
            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                output::print_crypto_snapshot(&report, timezone, args.raw)?;
            }
            status
        }
        CryptoCommand::Sentiment(args) => {
            let report = binance::sentiment(&config, &args.symbol).await;
            let status = report.ensure_complete();
            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                output::print_crypto_sentiment(&report, timezone, args.raw)?;
            }
            status
        }
        CryptoCommand::Spot(args) => run_spot(args, &config, timezone).await,
        CryptoCommand::Futures(args) => run_futures(args, &config, timezone).await,
        CryptoCommand::Stream(args) => {
            let report = binance::stream_messages(
                &config,
                args.market,
                args.kind,
                &args.symbol,
                &args.interval,
                args.messages,
            )
            .await?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                output::print_crypto_stream(&report)?;
            }
            Ok(())
        }
    }
}

pub async fn run_price(
    args: PriceArgs,
    proxy: Option<&str>,
    no_proxy: bool,
    timeout_seconds: u64,
    timezone: &str,
) -> Result<()> {
    let batch = fetch_price_batch(
        &binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy),
        args.crypto_market,
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
    let market = match args.provider {
        Provider::BinanceSpot => crate::cli::CryptoMarket::Spot,
        Provider::BinanceUsdsFutures => crate::cli::CryptoMarket::UsdsFutures,
        _ => args.crypto_market,
    };
    let config = binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy);
    let history =
        binance::fetch_history(&config, market, &args.symbol, &args.interval, args.limit).await?;
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
    let config = binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy);
    let market = match args.provider {
        Provider::BinanceSpot => crate::cli::CryptoMarket::Spot,
        Provider::BinanceUsdsFutures => crate::cli::CryptoMarket::UsdsFutures,
        _ => args.crypto_market,
    };
    let mut indicators = Vec::new();
    let mut errors = BTreeMap::new();
    for symbol in args.symbols {
        match binance::fetch_history(&config, market, &symbol, &args.interval, args.limit).await {
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
    let config = binance::BinanceConfig::from_env(timeout_seconds, proxy, no_proxy);
    let mut iteration = 0usize;
    let mut had_errors = false;
    loop {
        iteration += 1;
        let batch =
            fetch_price_batch(&config, args.crypto_market, args.symbols.clone(), timezone).await;
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
    config: &binance::BinanceConfig,
    market: crate::cli::CryptoMarket,
    symbols: Vec<String>,
    timezone: &str,
) -> CryptoPriceBatch {
    let mut points = Vec::new();
    let mut errors = BTreeMap::new();
    for symbol in symbols {
        match binance::fetch_quote(config, market, &symbol).await {
            Ok(quote) => points.push(price::quote_to_point(
                quote,
                "Binance crypto price",
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

async fn run_spot(
    args: crate::cli::CryptoSpotArgs,
    config: &binance::BinanceConfig,
    timezone: &str,
) -> Result<()> {
    match args.command {
        CryptoSpotCommand::ExchangeInfo(args) => {
            let report = binance::spot_exchange_info(config, args.symbol.as_deref()).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoSpotCommand::Ticker(args) => {
            let report = binance::spot_ticker(config, &args.symbol).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoSpotCommand::Ticker24h(args) => {
            let report = binance::spot_24h(config, &args.symbol).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoSpotCommand::AvgPrice(args) => {
            let report = binance::spot_avg_price(config, &args.symbol).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoSpotCommand::Book(args) => {
            let report = binance::spot_book(config, &args.symbol, args.limit).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoSpotCommand::Trades(args) => {
            let report =
                binance::spot_trades(config, &args.symbol, args.limit, args.aggregate).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoSpotCommand::Klines(args) => {
            let report =
                binance::spot_klines(config, &args.symbol, &args.interval, args.limit).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
    }
}

async fn run_futures(
    args: crate::cli::CryptoFuturesArgs,
    config: &binance::BinanceConfig,
    timezone: &str,
) -> Result<()> {
    match args.command {
        CryptoFuturesCommand::ExchangeInfo(args) => {
            let report = binance::futures_exchange_info(config).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoFuturesCommand::Ticker(args) => {
            let report = binance::futures_ticker(config, &args.symbol).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoFuturesCommand::Ticker24h(args) => {
            let report = binance::futures_24h(config, &args.symbol).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoFuturesCommand::Book(args) => {
            let report = binance::futures_book(config, &args.symbol, args.limit).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoFuturesCommand::Trades(args) => {
            let report = binance::futures_trades(config, &args.symbol, args.limit).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoFuturesCommand::Klines(args) => {
            let report =
                binance::futures_klines(config, &args.symbol, &args.interval, args.limit).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoFuturesCommand::Mark(args) => {
            let report = binance::futures_mark(config, &args.symbol).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoFuturesCommand::Funding(args) => {
            let report = binance::futures_funding(config, &args.symbol, args.limit).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoFuturesCommand::OpenInterest(args) => {
            let report = binance::futures_open_interest(config, &args.symbol).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoFuturesCommand::Ratios(args) => {
            let report =
                binance::futures_ratios(config, &args.symbol, args.period, args.limit).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoFuturesCommand::Flow(args) => {
            let report =
                binance::futures_flow(config, &args.symbol, args.period, args.limit).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
        CryptoFuturesCommand::Basis(args) => {
            let report =
                binance::futures_basis(config, &args.symbol, args.period, args.limit).await?;
            print_endpoint(report, args.json, args.raw, timezone)
        }
    }
}

fn print_endpoint(
    report: binance::CryptoEndpointReport,
    json: bool,
    raw: bool,
    timezone: &str,
) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        output::print_crypto_endpoint(&report, timezone, raw)?;
    }
    Ok(())
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
