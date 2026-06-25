use anyhow::Result;
use serde::Serialize;

use crate::args::{
    AssetClass, CryptoInstrument, CryptoProvider, HistoryAdjustment, HistorySession, Provider,
    StooqAsset, StooqMarket,
};
use crate::market_symbol::MarketSymbol;
use crate::model::{HistoryBatch, OhlcBar};
use crate::service::{self, HistoryRequest, MarketRuntime};
use crate::time;

#[derive(Debug, Clone)]
pub struct HistorySnapshotRequest {
    pub symbol: String,
    pub interval: String,
    pub range: String,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistorySnapshot {
    pub requested_symbol: String,
    pub symbol: String,
    pub provider: String,
    pub interval: String,
    pub fetched_at_local: Option<String>,
    pub latest_close: Option<f64>,
    pub latest_time: Option<String>,
    pub return_pct: Option<f64>,
    pub volume: Option<f64>,
    pub bars: Vec<HistoryBarSnapshot>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoryBarSnapshot {
    pub open_time: String,
    pub close: f64,
    pub volume: Option<f64>,
}

pub async fn fetch_history_snapshot(
    runtime: &MarketRuntime,
    request: HistorySnapshotRequest,
) -> Result<HistorySnapshot> {
    let market_symbol = MarketSymbol::new(request.symbol.clone());
    let asset = if market_symbol.is_crypto() {
        AssetClass::Crypto
    } else {
        AssetClass::Auto
    };
    let batch = service::history(
        runtime,
        HistoryRequest {
            symbol: request.symbol.clone(),
            asset,
            instrument: CryptoInstrument::Auto,
            crypto_provider: CryptoProvider::Auto,
            provider: Provider::Auto,
            session: HistorySession::Regular,
            adjustment: HistoryAdjustment::Auto,
            no_actions: false,
            repair: false,
            interval: request.interval,
            range: request.range,
            limit: request.limit,
            stooq_market: StooqMarket::Us,
            stooq_asset: StooqAsset::Stocks,
        },
    )
    .await?;

    Ok(snapshot_from_batch(
        request.symbol,
        batch,
        Some(time::now_local(runtime.timezone())),
    ))
}

fn snapshot_from_batch(
    requested_symbol: String,
    batch: HistoryBatch,
    fetched_at_local: Option<String>,
) -> HistorySnapshot {
    let bars = batch.bars.into_iter().map(bar_snapshot).collect::<Vec<_>>();
    let latest = bars.last();
    let first_close = bars.first().map(|bar| bar.close);
    let return_pct = first_close
        .zip(latest.map(|bar| bar.close))
        .filter(|(first, _)| *first != 0.0)
        .map(|(first, last)| (last / first - 1.0) * 100.0);
    let errors = if bars.is_empty() {
        vec![format!("{} history returned no bars", batch.symbol)]
    } else {
        Vec::new()
    };

    HistorySnapshot {
        requested_symbol,
        symbol: batch.symbol,
        provider: batch.provider,
        interval: batch.interval,
        fetched_at_local,
        latest_close: latest.map(|bar| bar.close),
        latest_time: latest.map(|bar| bar.open_time.clone()),
        return_pct,
        volume: latest.and_then(|bar| bar.volume),
        bars,
        errors,
    }
}

fn bar_snapshot(bar: OhlcBar) -> HistoryBarSnapshot {
    HistoryBarSnapshot {
        open_time: bar.open_time,
        close: bar.close,
        volume: bar.volume,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_from_batch_maps_latest_bar_and_return() {
        let snapshot = snapshot_from_batch(
            "CRDO".to_string(),
            HistoryBatch {
                symbol: "CRDO".to_string(),
                provider: "yahoo".to_string(),
                interval: "1d".to_string(),
                adjustment: "auto".to_string(),
                actions_included: true,
                repair_requested: false,
                repair_applied: false,
                bars: vec![
                    bar("2026-06-24", 100.0, 10_000.0),
                    bar("2026-06-25", 125.0, 12_000.0),
                ],
            },
            Some("2026-06-25 10:00:00".to_string()),
        );

        assert_eq!(snapshot.latest_close, Some(125.0));
        assert_eq!(snapshot.latest_time.as_deref(), Some("2026-06-25"));
        assert_eq!(snapshot.volume, Some(12_000.0));
        assert_eq!(snapshot.return_pct, Some(25.0));
        assert!(snapshot.errors.is_empty());
    }

    fn bar(open_time: &str, close: f64, volume: f64) -> OhlcBar {
        OhlcBar {
            symbol: "CRDO".to_string(),
            provider: "yahoo".to_string(),
            open_time: open_time.to_string(),
            close_time: None,
            open: Some(close),
            high: Some(close),
            low: Some(close),
            close,
            adj_close: None,
            volume: Some(volume),
            quote_volume: None,
            trades: None,
            dividend: None,
            stock_split: None,
            capital_gain: None,
            repaired: false,
        }
    }
}
