use std::collections::BTreeMap;

use anyhow::Result;

use crate::model::{
    DerivedIndicator, FuturesStats, HistoryBatch, PricePoint, PriceSummary, ProviderProfile,
    ResearchReport, SearchReport, StreamQuote,
};
use crate::page_read::PageReadReport;

pub fn print_price_summary(summary: &PriceSummary, show_all: bool) {
    println!(
        "{} price summary  fetched={}  tz={}",
        summary.symbol, summary.fetched_at_local, summary.timezone
    );
    if let Some(current) = summary.current.as_ref() {
        println!(
            "Current: {} {}  session={}  source={}  change={}  time={}",
            currency(current.currency.as_deref()),
            money_value(current.price),
            current.session.as_deref().unwrap_or("-"),
            current.provider,
            pct_value(current.change_pct),
            current.market_time_local.as_deref().unwrap_or("-")
        );
    } else {
        println!("Current: no quote available");
    }
    println!(
        "Regular basis: prev_close={} open={} high={} low={} volume={}",
        money_value(summary.regular_basis.previous_close),
        money_value(summary.regular_basis.open),
        money_value(summary.regular_basis.high),
        money_value(summary.regular_basis.low),
        number_value(summary.regular_basis.volume.map(|value| value as f64))
    );
    if let Some(proxy) = summary.proxy.as_ref() {
        println!(
            "Proxy: {} {} via {} time={} note={}",
            currency(proxy.currency.as_deref()),
            money_value(proxy.price),
            proxy.provider,
            proxy.market_time_local.as_deref().unwrap_or("-"),
            proxy.note.as_deref().unwrap_or("-")
        );
    }
    if show_all {
        println!();
        println!("Session / provider split");
        let headers = [
            "label", "price", "chg%", "session", "provider", "time", "open", "high", "low",
            "volume",
        ];
        let rows = summary
            .sessions
            .iter()
            .map(price_point_row)
            .collect::<Vec<_>>();
        print_table(&headers, &rows);
    } else if summary.sessions.len() > 1 {
        println!(
            "Note: fetched {} session/provider rows; use sessions to inspect the split.",
            summary.sessions.len()
        );
    }
    if !summary.errors.is_empty() {
        println!();
        println!("Quote errors");
        for (provider, error) in &summary.errors {
            println!("{provider}: {error}");
        }
    }
}

pub fn print_history_table(history: &HistoryBatch) {
    println!(
        "{} history via {} interval={} adjustment={} actions={} repair_requested={} repair_applied={}",
        history.symbol,
        history.provider,
        history.interval,
        history.adjustment,
        history.actions_included,
        history.repair_requested,
        history.repair_applied
    );
    let headers = [
        "time",
        "open",
        "high",
        "low",
        "close",
        "adj_close",
        "volume",
        "dividend",
        "split",
        "gain",
        "repair",
    ];
    let rows = history
        .bars
        .iter()
        .map(|bar| {
            vec![
                bar.open_time.clone(),
                money_value(bar.open),
                money_value(bar.high),
                money_value(bar.low),
                money_value(Some(bar.close)),
                money_value(bar.adj_close),
                number_value(bar.volume),
                money_value(bar.dividend),
                number_value(bar.stock_split),
                money_value(bar.capital_gain),
                if bar.repaired { "yes" } else { "-" }.to_string(),
            ]
        })
        .collect::<Vec<_>>();
    print_table(&headers, &rows);
}

pub fn print_indicator_table(indicators: &[DerivedIndicator], errors: &BTreeMap<String, String>) {
    let headers = [
        "symbol", "close", "1bar", "5bar", "20bar", "sma20", "sma50", "hi20", "lo20", "rv20",
    ];
    let mut rows = indicators
        .iter()
        .map(|indicator| {
            vec![
                indicator.symbol.clone(),
                money_value(indicator.latest_close),
                pct_value(indicator.return_1_bar_pct),
                pct_value(indicator.return_5_bar_pct),
                pct_value(indicator.return_20_bar_pct),
                money_value(indicator.sma_20),
                money_value(indicator.sma_50),
                money_value(indicator.high_20),
                money_value(indicator.low_20),
                pct_value(indicator.realized_vol_20_annualized_pct),
            ]
        })
        .collect::<Vec<_>>();

    for (symbol, error) in errors {
        rows.push(vec![
            symbol.clone(),
            "ERROR".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            error.clone(),
        ]);
    }
    print_table(&headers, &rows);
}

pub fn print_futures_stats(stats: &FuturesStats) {
    println!(
        "{} futures stats via {} fetched_at={}",
        stats.symbol, stats.provider, stats.fetched_at_utc
    );
    if let Some(ticker) = stats.ticker_24h.as_ref() {
        println!(
            "24h: last={} change={} high={} low={} quote_volume={} trades={}",
            money_value(ticker.last_price),
            pct_value(ticker.price_change_pct),
            money_value(ticker.high_price),
            money_value(ticker.low_price),
            number_value(ticker.quote_volume),
            ticker
                .count
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string())
        );
    }
    if let Some(mark) = stats.mark_price.as_ref() {
        println!(
            "mark: mark={} index={} funding={} next_funding={}",
            money_value(mark.mark_price),
            money_value(mark.index_price),
            pct_value(mark.last_funding_rate.map(|value| value * 100.0)),
            mark.next_funding_time.as_deref().unwrap_or("-")
        );
    }
    if let Some(open_interest) = stats.open_interest.as_ref() {
        println!(
            "open_interest: {} time={}",
            number_value(open_interest.open_interest),
            open_interest.time.as_deref().unwrap_or("-")
        );
    }
    if !stats.funding_rates.is_empty() {
        println!("recent funding:");
        let headers = ["time", "rate", "mark"];
        let rows = stats
            .funding_rates
            .iter()
            .map(|row| {
                vec![
                    row.funding_time.clone().unwrap_or_else(|| "-".to_string()),
                    pct_value(row.funding_rate.map(|value| value * 100.0)),
                    money_value(row.mark_price),
                ]
            })
            .collect::<Vec<_>>();
        print_table(&headers, &rows);
    }
    if !stats.errors.is_empty() {
        println!("errors:");
        for (name, error) in &stats.errors {
            println!("{name}: {error}");
        }
    }
}

pub fn print_page_read_report(report: &PageReadReport) {
    println!(
        "URL reader via {} fetched={} words={} chars={} truncated={}",
        report.provider,
        report.fetched_at_utc,
        report.word_count,
        report.char_count,
        if report.truncated { "yes" } else { "no" }
    );
    println!("url: {}", report.url);
    println!("source_url: {}", report.source_url);
    if let Some(title) = report.title.as_deref() {
        println!("title: {title}");
    }
    if !report.errors.is_empty() {
        println!();
        println!("Fallback errors:");
        for error in &report.errors {
            println!("{}: {}", error.provider, error.error);
        }
    }
    println!();
    println!("{}", report.content);
}

pub fn print_research_report(report: &ResearchReport, raw: bool) -> Result<()> {
    println!(
        "{} {} fetched={}",
        report.symbol, report.category, report.fetched_at_local
    );
    if !report.sources.is_empty() {
        println!(
            "sources: {}",
            report
                .sources
                .iter()
                .map(|source| format!("{}:{}", source.provider, source.cache_status))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if !report.modules.is_empty() {
        println!(
            "modules: {}",
            report
                .modules
                .iter()
                .map(|module| format!("{}:{}:{}", module.provider, module.name, module.status))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if report.highlights.is_empty() {
        println!("No highlights extracted; use --json to inspect the raw payload.");
    } else {
        let headers = ["source", "module", "field", "value"];
        let rows = report
            .highlights
            .iter()
            .map(|row| {
                vec![
                    row.provider.clone(),
                    row.module.clone(),
                    row.label.clone(),
                    row.value.clone(),
                ]
            })
            .collect::<Vec<_>>();
        print_table(&headers, &rows);
    }
    if !report.coverage_gaps.is_empty() {
        println!();
        println!("Coverage Gaps");
        println!("-------------");
        for gap in &report.coverage_gaps {
            println!("{}: {}", gap.module, gap.reason);
        }
    }
    if raw {
        println!();
        println!("{}", serde_json::to_string_pretty(&report.payload)?);
    }
    Ok(())
}

pub fn print_search_report(report: &SearchReport, raw: bool) -> Result<()> {
    println!(
        "{} {} via {} fetched={} cache={}",
        report.category,
        report.query,
        report.provider,
        report.fetched_at_local,
        report.cache_status
    );
    if report.highlights.is_empty() {
        println!("No highlights extracted; use --json to inspect the raw payload.");
    } else {
        let headers = ["source", "item", "value"];
        let rows = report
            .highlights
            .iter()
            .map(|row| vec![row.provider.clone(), row.label.clone(), row.value.clone()])
            .collect::<Vec<_>>();
        print_table(&headers, &rows);
    }
    if raw {
        println!();
        println!("{}", serde_json::to_string_pretty(&report.payload)?);
    }
    Ok(())
}

pub fn print_provider_profiles(profiles: &[ProviderProfile]) {
    let headers = [
        "provider",
        "key",
        "official",
        "stability",
        "large",
        "best_for",
    ];
    let rows = profiles
        .iter()
        .map(|profile| {
            vec![
                profile.provider.clone(),
                if profile.requires_api_key {
                    "required".to_string()
                } else {
                    "no".to_string()
                },
                profile.official_status.clone(),
                profile.stability.clone(),
                profile.large_download.to_string(),
                profile.best_for.clone(),
            ]
        })
        .collect::<Vec<_>>();
    print_table(&headers, &rows);

    println!();
    println!("Capabilities");
    println!("------------");
    let headers = ["provider", "module", "status", "implemented", "note"];
    let rows = profiles
        .iter()
        .flat_map(|profile| {
            profile.capabilities.iter().map(|capability| {
                vec![
                    profile.provider.clone(),
                    capability.module.clone(),
                    capability.status.clone(),
                    capability.implemented.to_string(),
                    capability.note.clone(),
                ]
            })
        })
        .collect::<Vec<_>>();
    print_table(&headers, &rows);
}

pub fn print_stooq_catalog(catalog: &crate::model::StooqCatalog) {
    println!(
        "Stooq bulk catalog fetched={} source={}",
        catalog.fetched_at_utc, catalog.source_url
    );
    let headers = [
        "frequency",
        "market",
        "asset",
        "size_mb",
        "cached",
        "cache_key",
        "label",
    ];
    let rows = catalog
        .entries
        .iter()
        .map(|entry| {
            vec![
                entry.frequency.clone(),
                entry.market.clone(),
                entry.asset.clone(),
                number_value(entry.approx_size_mb),
                entry
                    .cached_zip_path
                    .clone()
                    .unwrap_or_else(|| "no".to_string()),
                entry.cache_key.clone(),
                entry.label.clone(),
            ]
        })
        .collect::<Vec<_>>();
    print_table(&headers, &rows);
    println!();
    println!(
        "Download note: Stooq bulk download links are captcha-authorized. Use `stooq sync --zip-path <file>` or `stooq sync --url <authorized-url>`."
    );
}

pub fn print_stooq_sync_report(report: &crate::model::StooqSyncReport) {
    println!(
        "Stooq synced {} {} {} bytes={} path={}",
        report.frequency, report.market, report.asset, report.bytes, report.zip_path
    );
    println!("source: {}", report.source);
    println!("imported_at_utc: {}", report.imported_at_utc);
}

pub fn print_stream_quotes(updates: &[StreamQuote]) {
    let headers = [
        "symbol",
        "price",
        "chg%",
        "market_hours",
        "time",
        "exchange",
        "volume",
        "name",
    ];
    let rows = updates
        .iter()
        .map(|quote| {
            vec![
                quote.symbol.clone(),
                money_value(Some(quote.price)),
                pct_value(quote.change_pct),
                quote
                    .market_hours
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                quote.time_local.clone().unwrap_or_else(|| "-".to_string()),
                quote.exchange.clone().unwrap_or_else(|| "-".to_string()),
                quote
                    .day_volume
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                quote.short_name.clone().unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect::<Vec<_>>();
    print_table(&headers, &rows);
}

fn price_point_row(point: &PricePoint) -> Vec<String> {
    vec![
        point.label.clone(),
        money_value(point.price),
        pct_value(point.change_pct),
        point.session.clone().unwrap_or_else(|| "-".to_string()),
        point.provider.clone(),
        point
            .market_time_local
            .clone()
            .unwrap_or_else(|| "-".to_string()),
        money_value(point.open),
        money_value(point.high),
        money_value(point.low),
        number_value(point.volume.map(|value| value as f64)),
    ]
}

fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    let mut widths = headers
        .iter()
        .map(|header| header.len())
        .collect::<Vec<_>>();
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }

    println!("{}", table_row(headers.iter().copied(), &widths));
    println!(
        "{}",
        table_row(widths.iter().map(|width| "-".repeat(*width)), &widths)
    );
    for row in rows {
        println!("{}", table_row(row.iter().map(String::as_str), &widths));
    }
}

fn table_row<I, S>(values: I, widths: &[usize]) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    values
        .into_iter()
        .zip(widths.iter())
        .map(|(value, width)| format!("{:<width$}", value.as_ref()))
        .collect::<Vec<_>>()
        .join("  ")
}

fn money_value(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("${value:.2}"),
        None => "-".to_string(),
    }
}

fn currency(value: Option<&str>) -> &str {
    value.unwrap_or("USD")
}

fn number_value(value: Option<f64>) -> String {
    match value {
        Some(value) => {
            let formatted = format!("{value:.4}");
            formatted
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        }
        None => "-".to_string(),
    }
}

fn pct_value(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{value:+.2}%"),
        None => "-".to_string(),
    }
}
