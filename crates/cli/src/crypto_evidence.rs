use anyhow::{Result, anyhow};

use crate::crypto_capability::{CryptoCapability, resolve_instrument};
use crate::crypto_runtime::{
    CryptoEvidenceReport, CryptoEvidenceSources, EvidenceEngine, EvidenceRequest, evidence_report,
};

pub async fn run_quote(
    args: crate::cli::CryptoEvidenceSymbolArgs,
    proxy: Option<&str>,
    no_proxy: bool,
    timeout_seconds: u64,
) -> Result<()> {
    let sources = CryptoEvidenceSources::new(proxy, no_proxy, timeout_seconds)?;
    let capability = CryptoCapability::Quote;
    let instrument = resolve_instrument(args.instrument, capability);
    let request = EvidenceRequest::new(args.provider, instrument, capability);
    let symbol = args.symbol;
    let results = EvidenceEngine::collect(request, |provider| {
        let sources = sources.clone();
        let symbol = symbol.clone();
        async move { sources.quote(provider, instrument, symbol).await }
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
    let sources = CryptoEvidenceSources::new(proxy, no_proxy, timeout_seconds)?;
    let capability = CryptoCapability::Book;
    let instrument = resolve_instrument(args.instrument, capability);
    let request = EvidenceRequest::new(args.provider, instrument, capability);
    let symbol = args.symbol;
    let limit = args.limit;
    let results = EvidenceEngine::collect(request, |provider| {
        let sources = sources.clone();
        let symbol = symbol.clone();
        async move { sources.book(provider, instrument, symbol, limit).await }
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
    let sources = CryptoEvidenceSources::new(proxy, no_proxy, timeout_seconds)?;
    let capability = CryptoCapability::Trades;
    let instrument = resolve_instrument(args.instrument, capability);
    let request = EvidenceRequest::new(args.provider, instrument, capability);
    let symbol = args.symbol;
    let limit = args.limit;
    let aggregate = args.aggregate;
    let results = EvidenceEngine::collect(request, |provider| {
        let sources = sources.clone();
        let symbol = symbol.clone();
        async move {
            sources
                .trades(provider, instrument, symbol, limit, aggregate)
                .await
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
    let sources = CryptoEvidenceSources::new(proxy, no_proxy, timeout_seconds)?;
    let capability = CryptoCapability::Candles;
    let instrument = resolve_instrument(args.instrument, capability);
    let request = EvidenceRequest::new(args.provider, instrument, capability);
    let symbol = args.symbol;
    let interval = args.interval;
    let limit = args.limit;
    let results = EvidenceEngine::collect(request, |provider| {
        let sources = sources.clone();
        let symbol = symbol.clone();
        let interval = interval.clone();
        async move {
            sources
                .candles(provider, instrument, symbol, interval, limit)
                .await
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
    let sources = CryptoEvidenceSources::new(proxy, no_proxy, timeout_seconds)?;
    let capability = CryptoCapability::Funding;
    let instrument = resolve_instrument(args.instrument, capability);
    let request = EvidenceRequest::new(args.provider, instrument, capability);
    let symbol = args.symbol;
    let limit = args.limit;
    let results = EvidenceEngine::collect(request, |provider| {
        let sources = sources.clone();
        let symbol = symbol.clone();
        async move { sources.funding(provider, instrument, symbol, limit).await }
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
    let sources = CryptoEvidenceSources::new(proxy, no_proxy, timeout_seconds)?;
    let capability = CryptoCapability::OpenInterest;
    let instrument = resolve_instrument(args.instrument, capability);
    let request = EvidenceRequest::new(args.provider, instrument, capability);
    let symbol = args.symbol;
    let results = EvidenceEngine::collect(request, |provider| {
        let sources = sources.clone();
        let symbol = symbol.clone();
        async move { sources.open_interest(provider, instrument, symbol).await }
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
    let sources = CryptoEvidenceSources::new(proxy, no_proxy, timeout_seconds)?;
    let capability = CryptoCapability::Discover(args.kind);
    let instrument = resolve_instrument(args.instrument, capability);
    let request = EvidenceRequest::new(args.provider, instrument, capability);
    let kind = args.kind;
    let limit = args.limit;
    let vs_currency = args.vs_currency;
    let results = EvidenceEngine::collect(request, |provider| {
        let sources = sources.clone();
        let vs_currency = vs_currency.clone();
        async move {
            sources
                .discover(provider, instrument, kind, limit, vs_currency)
                .await
        }
    })
    .await;
    print_evidence_report(
        evidence_report(capability, instrument, None, results),
        args.json,
        args.raw,
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
