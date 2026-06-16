use std::collections::BTreeMap;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::http::{parse_optional_f64, timestamp_ms_to_utc, utc_now};
use crate::model::{
    FuturesFundingRate, FuturesMarkPrice, FuturesOpenInterest, FuturesStats, FuturesTicker24h,
    HistoryBatch, OhlcBar, Quote, SESSION_24H_PROXY,
};

const BASE_URL: &str = "https://fapi.binance.com";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PriceTicker {
    symbol: String,
    price: String,
    time: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Ticker24h {
    last_price: Option<String>,
    price_change: Option<String>,
    price_change_percent: Option<String>,
    weighted_avg_price: Option<String>,
    open_price: Option<String>,
    high_price: Option<String>,
    low_price: Option<String>,
    volume: Option<String>,
    quote_volume: Option<String>,
    open_time: Option<i64>,
    close_time: Option<i64>,
    count: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PremiumIndex {
    mark_price: Option<String>,
    index_price: Option<String>,
    estimated_settle_price: Option<String>,
    last_funding_rate: Option<String>,
    interest_rate: Option<String>,
    next_funding_time: Option<i64>,
    time: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenInterest {
    open_interest: Option<String>,
    time: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FundingRate {
    funding_rate: Option<String>,
    funding_time: Option<i64>,
    mark_price: Option<String>,
}

pub async fn fetch_quote(client: &Client, symbol: &str) -> Result<Quote> {
    let provider_symbol = symbol.to_uppercase();
    let url = format!(
        "{}/fapi/v2/ticker/price?symbol={provider_symbol}",
        base_url()
    );
    let ticker: PriceTicker = client
        .get(url)
        .send()
        .await
        .context("Binance futures price request failed")?
        .error_for_status()
        .context("Binance futures price returned HTTP error")?
        .json()
        .await
        .context("Binance futures price JSON parse failed")?;
    let price = ticker
        .price
        .parse::<f64>()
        .context("Binance futures price is not numeric")?;

    Ok(Quote {
        symbol: ticker.symbol,
        price,
        currency: Some("USDT".to_string()),
        provider: "binance-futures".to_string(),
        session: Some(SESSION_24H_PROXY.to_string()),
        fetched_at_utc: utc_now(),
        market_time: ticker.time.and_then(timestamp_ms_to_utc),
        previous_close: None,
        open: None,
        high: None,
        low: None,
        volume: None,
        exchange: Some("Binance USDⓈ-M Futures".to_string()),
        provider_symbol: Some(provider_symbol),
        change_pct: None,
    })
}

pub async fn fetch_history(
    client: &Client,
    symbol: &str,
    interval: &str,
    limit: usize,
) -> Result<HistoryBatch> {
    let provider_symbol = symbol.to_uppercase();
    let limit = limit.clamp(1, 1500);
    let url = format!(
        "{}/fapi/v1/klines?symbol={provider_symbol}&interval={interval}&limit={limit}",
        base_url()
    );
    let rows: Vec<Vec<Value>> = client
        .get(url)
        .send()
        .await
        .context("Binance futures klines request failed")?
        .error_for_status()
        .context("Binance futures klines returned HTTP error")?
        .json()
        .await
        .context("Binance futures klines JSON parse failed")?;
    let bars = rows
        .into_iter()
        .filter_map(|row| kline_row_to_bar(symbol, row))
        .collect::<Vec<_>>();

    Ok(HistoryBatch {
        symbol: symbol.to_uppercase(),
        provider: "binance-futures".to_string(),
        interval: interval.to_string(),
        adjustment: "raw".to_string(),
        actions_included: false,
        repair_requested: false,
        repair_applied: false,
        bars,
    })
}

pub async fn fetch_futures_stats(
    client: &Client,
    symbol: &str,
    funding_limit: usize,
) -> FuturesStats {
    let normalized = symbol.to_uppercase();
    let mut errors = BTreeMap::new();

    let (ticker_24h_result, mark_price_result, open_interest_result, funding_rates_result) = tokio::join!(
        fetch_24h(client, &normalized),
        fetch_mark_price(client, &normalized),
        fetch_open_interest(client, &normalized),
        fetch_funding_rates(client, &normalized, funding_limit)
    );

    let ticker_24h = match ticker_24h_result {
        Ok(value) => Some(value),
        Err(error) => {
            errors.insert("ticker_24h".to_string(), format!("{error:#}"));
            None
        }
    };
    let mark_price = match mark_price_result {
        Ok(value) => Some(value),
        Err(error) => {
            errors.insert("mark_price".to_string(), format!("{error:#}"));
            None
        }
    };
    let open_interest = match open_interest_result {
        Ok(value) => Some(value),
        Err(error) => {
            errors.insert("open_interest".to_string(), format!("{error:#}"));
            None
        }
    };
    let funding_rates = match funding_rates_result {
        Ok(value) => value,
        Err(error) => {
            errors.insert("funding_rates".to_string(), format!("{error:#}"));
            Vec::new()
        }
    };

    FuturesStats {
        symbol: normalized,
        provider: "binance-futures".to_string(),
        fetched_at_utc: utc_now(),
        ticker_24h,
        mark_price,
        open_interest,
        funding_rates,
        errors,
    }
}

async fn fetch_24h(client: &Client, symbol: &str) -> Result<FuturesTicker24h> {
    let url = format!("{}/fapi/v1/ticker/24hr?symbol={symbol}", base_url());
    let ticker: Ticker24h = client
        .get(url)
        .send()
        .await
        .context("Binance futures 24h ticker request failed")?
        .error_for_status()
        .context("Binance futures 24h ticker returned HTTP error")?
        .json()
        .await
        .context("Binance futures 24h ticker JSON parse failed")?;
    Ok(FuturesTicker24h {
        last_price: parse_optional_f64(ticker.last_price.as_deref()),
        price_change: parse_optional_f64(ticker.price_change.as_deref()),
        price_change_pct: parse_optional_f64(ticker.price_change_percent.as_deref()),
        weighted_avg_price: parse_optional_f64(ticker.weighted_avg_price.as_deref()),
        open_price: parse_optional_f64(ticker.open_price.as_deref()),
        high_price: parse_optional_f64(ticker.high_price.as_deref()),
        low_price: parse_optional_f64(ticker.low_price.as_deref()),
        volume: parse_optional_f64(ticker.volume.as_deref()),
        quote_volume: parse_optional_f64(ticker.quote_volume.as_deref()),
        count: ticker.count,
        open_time: ticker.open_time.and_then(timestamp_ms_to_utc),
        close_time: ticker.close_time.and_then(timestamp_ms_to_utc),
    })
}

async fn fetch_mark_price(client: &Client, symbol: &str) -> Result<FuturesMarkPrice> {
    let url = format!("{}/fapi/v1/premiumIndex?symbol={symbol}", base_url());
    let mark: PremiumIndex = client
        .get(url)
        .send()
        .await
        .context("Binance futures mark price request failed")?
        .error_for_status()
        .context("Binance futures mark price returned HTTP error")?
        .json()
        .await
        .context("Binance futures mark price JSON parse failed")?;
    Ok(FuturesMarkPrice {
        mark_price: parse_optional_f64(mark.mark_price.as_deref()),
        index_price: parse_optional_f64(mark.index_price.as_deref()),
        estimated_settle_price: parse_optional_f64(mark.estimated_settle_price.as_deref()),
        last_funding_rate: parse_optional_f64(mark.last_funding_rate.as_deref()),
        interest_rate: parse_optional_f64(mark.interest_rate.as_deref()),
        next_funding_time: mark.next_funding_time.and_then(timestamp_ms_to_utc),
        time: mark.time.and_then(timestamp_ms_to_utc),
    })
}

async fn fetch_open_interest(client: &Client, symbol: &str) -> Result<FuturesOpenInterest> {
    let url = format!("{}/fapi/v1/openInterest?symbol={symbol}", base_url());
    let open_interest: OpenInterest = client
        .get(url)
        .send()
        .await
        .context("Binance futures open interest request failed")?
        .error_for_status()
        .context("Binance futures open interest returned HTTP error")?
        .json()
        .await
        .context("Binance futures open interest JSON parse failed")?;
    Ok(FuturesOpenInterest {
        open_interest: parse_optional_f64(open_interest.open_interest.as_deref()),
        time: open_interest.time.and_then(timestamp_ms_to_utc),
    })
}

async fn fetch_funding_rates(
    client: &Client,
    symbol: &str,
    funding_limit: usize,
) -> Result<Vec<FuturesFundingRate>> {
    let limit = funding_limit.clamp(1, 1000);
    let url = format!(
        "{}/fapi/v1/fundingRate?symbol={symbol}&limit={limit}",
        base_url()
    );
    let rows: Vec<FundingRate> = client
        .get(url)
        .send()
        .await
        .context("Binance futures funding rate request failed")?
        .error_for_status()
        .context("Binance futures funding rate returned HTTP error")?
        .json()
        .await
        .context("Binance futures funding rate JSON parse failed")?;
    Ok(rows
        .into_iter()
        .map(|row| FuturesFundingRate {
            funding_rate: parse_optional_f64(row.funding_rate.as_deref()),
            funding_time: row.funding_time.and_then(timestamp_ms_to_utc),
            mark_price: parse_optional_f64(row.mark_price.as_deref()),
        })
        .collect())
}

fn kline_row_to_bar(symbol: &str, row: Vec<Value>) -> Option<OhlcBar> {
    let open_time = row.first()?.as_i64()?;
    let open = parse_value_f64(row.get(1));
    let high = parse_value_f64(row.get(2));
    let low = parse_value_f64(row.get(3));
    let close = parse_value_f64(row.get(4))?;
    let volume = parse_value_f64(row.get(5));
    let close_time = row.get(6).and_then(Value::as_i64);
    let quote_volume = parse_value_f64(row.get(7));
    let trades = row.get(8).and_then(Value::as_u64);
    Some(OhlcBar {
        symbol: symbol.to_uppercase(),
        provider: "binance-futures".to_string(),
        open_time: timestamp_ms_to_utc(open_time)?,
        close_time: close_time.and_then(timestamp_ms_to_utc),
        open,
        high,
        low,
        close,
        volume,
        quote_volume,
        trades,
        adj_close: None,
        dividend: None,
        stock_split: None,
        capital_gain: None,
        repaired: false,
    })
}

fn parse_value_f64(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::String(value) => parse_optional_f64(Some(value)),
        Value::Number(value) => value.as_f64(),
        _ => None,
    }
}

fn base_url() -> String {
    std::env::var("BINANCE_FUTURES_BASE_URL").unwrap_or_else(|_| BASE_URL.to_string())
}
