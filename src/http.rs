use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{SecondsFormat, TimeZone, Utc};
use wreq::{Client, Proxy};
use wreq_util::{Emulation, EmulationOS, EmulationOption};

pub fn http_client(timeout_seconds: u64, proxy: Option<&str>, no_proxy: bool) -> Result<Client> {
    let emulation = EmulationOption::builder()
        .emulation(Emulation::Chrome137)
        .emulation_os(EmulationOS::Linux)
        .build();
    let mut builder = Client::builder()
        .emulation(emulation)
        .timeout(Duration::from_secs(timeout_seconds))
        .cookie_store(true);

    if let Some(proxy) = selected_proxy(proxy, no_proxy) {
        builder = builder
            .proxy(Proxy::all(&proxy).with_context(|| format!("invalid proxy URL: {proxy}"))?);
    } else if no_proxy {
        builder = builder.no_proxy();
    }

    builder.build().context("failed to build HTTP client")
}

pub fn selected_proxy(proxy: Option<&str>, no_proxy: bool) -> Option<String> {
    if no_proxy {
        return None;
    }
    proxy
        .map(str::to_string)
        .or_else(|| std::env::var("AGENT_FINANCE_PROXY").ok())
        .or_else(|| std::env::var("ALL_PROXY").ok())
        .or_else(|| std::env::var("HTTPS_PROXY").ok())
        .or_else(|| std::env::var("HTTP_PROXY").ok())
}

pub fn utc_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn timestamp_ms_to_utc(timestamp: i64) -> Option<String> {
    Utc.timestamp_millis_opt(timestamp)
        .single()
        .map(|datetime| datetime.to_rfc3339_opts(SecondsFormat::Secs, true))
}

pub fn timestamp_sec_to_utc(timestamp: i64) -> Option<String> {
    Utc.timestamp_opt(timestamp, 0)
        .single()
        .map(|datetime| datetime.to_rfc3339_opts(SecondsFormat::Secs, true))
}

pub fn change_pct(price: f64, previous_close: Option<f64>) -> Option<f64> {
    let previous_close = previous_close?;
    if previous_close == 0.0 {
        None
    } else {
        Some((price - previous_close) / previous_close * 100.0)
    }
}

pub fn parse_optional_f64(value: Option<&str>) -> Option<f64> {
    let value = clean_text(value)?;
    value.parse::<f64>().ok()
}

pub fn parse_optional_u64(value: Option<&str>) -> Option<u64> {
    let value = clean_text(value)?;
    value.parse::<f64>().ok().map(|number| number as u64)
}

pub fn clean_text(value: Option<&str>) -> Option<&str> {
    match value.map(str::trim) {
        Some("") | Some("N/D") | None => None,
        Some(value) => Some(value),
    }
}
