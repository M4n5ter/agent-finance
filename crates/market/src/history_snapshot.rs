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
    pub provider: Provider,
    pub crypto_provider: CryptoProvider,
    pub session: HistorySession,
    pub interval: String,
    pub range: String,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistorySnapshot {
    pub requested_symbol: String,
    pub symbol: String,
    pub provider: String,
    pub session: String,
    pub interval: String,
    pub range: String,
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
    pub close_time: Option<String>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: f64,
    pub volume: Option<f64>,
    pub quote_volume: Option<f64>,
    pub trades: Option<u64>,
    pub repaired: bool,
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
    let session = if asset == AssetClass::Crypto
        || matches!(
            request.provider,
            Provider::BinanceSpot | Provider::BinanceUsdsFutures
        ) {
        "24/7".to_string()
    } else {
        request.session.label().to_string()
    };
    let batch = service::history(
        runtime,
        HistoryRequest {
            symbol: request.symbol.clone(),
            asset,
            instrument: CryptoInstrument::Auto,
            crypto_provider: request.crypto_provider,
            provider: request.provider,
            session: request.session,
            adjustment: HistoryAdjustment::Auto,
            no_actions: false,
            repair: false,
            interval: request.interval,
            range: request.range.clone(),
            limit: request.limit,
            stooq_market: StooqMarket::Us,
            stooq_asset: StooqAsset::Stocks,
        },
    )
    .await?;

    Ok(snapshot_from_batch(
        request.symbol,
        session,
        request.range,
        batch,
        Some(time::now_local(runtime.timezone())),
    ))
}

fn snapshot_from_batch(
    requested_symbol: String,
    session: String,
    range: String,
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
        session,
        interval: batch.interval,
        range,
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
        close_time: bar.close_time,
        open: bar.open,
        high: bar.high,
        low: bar.low,
        close: bar.close,
        volume: bar.volume,
        quote_volume: bar.quote_volume,
        trades: bar.trades,
        repaired: bar.repaired,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_from_batch_maps_latest_bar_and_return() {
        let snapshot = snapshot_from_batch(
            "CRDO".to_string(),
            HistorySession::Regular.label().to_string(),
            "5d".to_string(),
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
        assert_eq!(snapshot.session, "regular");
        assert_eq!(snapshot.range, "5d");
        assert_eq!(snapshot.latest_time.as_deref(), Some("2026-06-25"));
        assert_eq!(snapshot.volume, Some(12_000.0));
        assert_eq!(snapshot.bars[1].open, Some(124.0));
        assert_eq!(snapshot.bars[1].high, Some(130.0));
        assert_eq!(snapshot.bars[1].low, Some(120.0));
        assert_eq!(snapshot.bars[1].quote_volume, Some(1_500_000.0));
        assert_eq!(snapshot.bars[1].trades, Some(42));
        assert!(snapshot.bars[1].repaired);
        assert_eq!(snapshot.return_pct, Some(25.0));
        assert!(snapshot.errors.is_empty());
    }

    fn bar(open_time: &str, close: f64, volume: f64) -> OhlcBar {
        OhlcBar {
            symbol: "CRDO".to_string(),
            provider: "yahoo".to_string(),
            open_time: open_time.to_string(),
            close_time: Some(format!("{open_time} close")),
            open: Some(close - 1.0),
            high: Some(close + 5.0),
            low: Some(close - 5.0),
            close,
            adj_close: None,
            volume: Some(volume),
            quote_volume: Some(volume * close),
            trades: Some(42),
            dividend: None,
            stock_split: None,
            capital_gain: None,
            repaired: true,
        }
    }
}
