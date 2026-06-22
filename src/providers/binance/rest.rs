use anyhow::{Context, Result, anyhow};
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use url::Url;
use wreq::Client;

use super::config::BinanceConfig;
use super::types::{CryptoEndpointReport, RestMarket, market_name, report_from_value};

pub(super) async fn endpoint_report(
    config: &BinanceConfig,
    market: RestMarket,
    endpoint: &str,
    path: &str,
    symbol: Option<String>,
    params: Vec<(&'static str, String)>,
) -> Result<CryptoEndpointReport> {
    let payload = rest_get(config, market, path, params).await?;
    Ok(report_from_value(
        market,
        endpoint,
        symbol,
        200,
        Vec::new(),
        payload,
    ))
}

pub(super) async fn rest_get(
    config: &BinanceConfig,
    market: RestMarket,
    path: &str,
    params: Vec<(&'static str, String)>,
) -> Result<Value> {
    let client = config.client()?;
    let mut errors = Vec::new();
    for base_url in config.base_urls(market) {
        match rest_get_with_retries(&client, config, base_url, path, &params).await {
            Ok(value) => return Ok(value),
            Err(error) => errors.push(format!("{base_url}: {error:#}")),
        }
    }
    Err(anyhow!(
        "all Binance {} endpoints failed for {path}: {}",
        market_name(market),
        errors.join(" | ")
    ))
}

async fn rest_get_with_retries(
    client: &Client,
    config: &BinanceConfig,
    base_url: &str,
    path: &str,
    params: &[(&'static str, String)],
) -> Result<Value> {
    let mut last_error = None;
    for attempt in 1..=3 {
        match rest_get_once(client, config, base_url, path, params).await {
            Ok(value) => return Ok(value),
            Err(error) => {
                let message = format!("{error:#}");
                if !is_transient_network_error(&message) || attempt == 3 {
                    return Err(error);
                }
                last_error = Some(message);
                sleep(Duration::from_millis(150 * attempt)).await;
            }
        }
    }
    Err(anyhow!(
        "Binance request failed after retries: {}",
        last_error.unwrap_or_else(|| "unknown error".to_string())
    ))
}

async fn rest_get_once(
    client: &Client,
    config: &BinanceConfig,
    base_url: &str,
    path: &str,
    params: &[(&'static str, String)],
) -> Result<Value> {
    let mut url = Url::parse(base_url)
        .with_context(|| format!("invalid Binance base URL: {base_url}"))?
        .join(path.trim_start_matches('/'))
        .with_context(|| format!("invalid Binance API path: {path}"))?;
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in params {
            query.append_pair(key, value);
        }
    }
    let mut request = client.get(url.as_str());
    if let Some(api_key) = config.api_key.as_deref() {
        request = request.header("X-MBX-APIKEY", api_key);
    }
    let response = request
        .send()
        .await
        .with_context(|| format!("Binance request failed: {url}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("Binance response body read failed: {url}"))?;
    if !status.is_success() {
        return Err(anyhow!(
            "Binance request failed status={} body={body}",
            status.as_u16()
        ));
    }
    serde_json::from_str(&body)
        .with_context(|| format!("Binance response JSON decode failed: {url}"))
}

fn is_transient_network_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    [
        "unexpected eof",
        "timed out",
        "timeout",
        "connection reset",
        "connection closed",
        "connect",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}
