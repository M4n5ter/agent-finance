use anyhow::Result;
use serde::Serialize;

use crate::args::{CryptoInstrument, CryptoProvider};
use crate::crypto_runtime::CryptoEvidenceReport;
use crate::service::{self, CryptoEvidenceSymbolRequest, MarketRuntime};
use crate::time;

#[derive(Debug, Clone)]
pub struct CryptoQuoteEvidenceSnapshotRequest {
    pub symbol: String,
    pub provider: CryptoProvider,
    pub instrument: CryptoInstrument,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CryptoQuoteEvidenceSnapshot {
    pub requested_symbol: String,
    pub symbol: String,
    pub instrument: String,
    pub fetched_at_local: Option<String>,
    pub ok_providers: usize,
    pub total_providers: usize,
    pub providers: Vec<ProviderQuoteEvidenceSnapshot>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProviderQuoteEvidenceSnapshot {
    pub provider: String,
    pub ok: bool,
    pub ok_endpoints: usize,
    pub total_endpoints: usize,
    pub required_failed: usize,
    pub first_error: Option<String>,
    pub endpoints: Vec<EndpointQuoteEvidenceSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EndpointQuoteEvidenceSnapshot {
    pub endpoint: String,
    pub required: bool,
    pub ok: bool,
    pub error: Option<String>,
}

pub async fn fetch_crypto_quote_evidence_snapshot(
    runtime: &MarketRuntime,
    request: CryptoQuoteEvidenceSnapshotRequest,
) -> Result<CryptoQuoteEvidenceSnapshot> {
    let report = service::crypto_evidence_quote(
        runtime,
        CryptoEvidenceSymbolRequest {
            symbol: request.symbol.clone(),
            provider: request.provider,
            instrument: request.instrument,
        },
    )
    .await?;

    Ok(snapshot_from_report(
        request.symbol,
        report,
        Some(time::now_local(runtime.timezone())),
    ))
}

fn snapshot_from_report(
    requested_symbol: String,
    report: CryptoEvidenceReport,
    fetched_at_local: Option<String>,
) -> CryptoQuoteEvidenceSnapshot {
    let providers = report
        .results
        .into_iter()
        .map(|provider| {
            let endpoints = provider
                .endpoints
                .into_iter()
                .map(|endpoint| EndpointQuoteEvidenceSnapshot {
                    endpoint: endpoint.endpoint,
                    required: endpoint.required,
                    ok: endpoint.ok,
                    error: endpoint.error,
                })
                .collect::<Vec<_>>();
            let ok_endpoints = endpoints.iter().filter(|endpoint| endpoint.ok).count();
            let total_endpoints = endpoints.len();
            let required_failed = endpoints
                .iter()
                .filter(|endpoint| endpoint.required && !endpoint.ok)
                .count();
            let first_error = endpoints.iter().find_map(|endpoint| endpoint.error.clone());
            ProviderQuoteEvidenceSnapshot {
                provider: provider.provider,
                ok: provider.ok,
                ok_endpoints,
                total_endpoints,
                required_failed,
                first_error,
                endpoints,
            }
        })
        .collect::<Vec<_>>();
    let total_providers = providers.len();
    let ok_providers = providers.iter().filter(|provider| provider.ok).count();
    let errors = providers
        .iter()
        .filter_map(|provider| {
            provider
                .first_error
                .as_ref()
                .map(|error| format!("{}: {error}", provider.provider))
        })
        .collect::<Vec<_>>();

    CryptoQuoteEvidenceSnapshot {
        requested_symbol,
        symbol: report.symbol.unwrap_or_default(),
        instrument: report.instrument,
        fetched_at_local,
        ok_providers,
        total_providers,
        providers,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto_runtime::{EndpointEvidence, ProviderEvidence};

    #[test]
    fn snapshot_from_report_summarizes_provider_endpoint_health() {
        let snapshot = snapshot_from_report(
            "BTCUSDT".to_string(),
            CryptoEvidenceReport {
                capability: "quote".to_string(),
                instrument: "spot".to_string(),
                symbol: Some("BTCUSDT".to_string()),
                fetched_at_utc: "2026-06-25T00:00:00Z".to_string(),
                results: vec![
                    ProviderEvidence {
                        provider: "binance".to_string(),
                        ok: true,
                        endpoints: vec![endpoint("quote", true, true, None)],
                    },
                    ProviderEvidence {
                        provider: "okx".to_string(),
                        ok: false,
                        endpoints: vec![endpoint("ticker", true, false, Some("timeout"))],
                    },
                ],
            },
            Some("2026-06-25 10:00:00".to_string()),
        );

        assert_eq!(snapshot.ok_providers, 1);
        assert_eq!(snapshot.total_providers, 2);
        assert_eq!(snapshot.errors, vec!["okx: timeout"]);
        assert_eq!(snapshot.providers[0].ok_endpoints, 1);
        assert_eq!(snapshot.providers[1].required_failed, 1);
        assert_eq!(snapshot.providers[1].endpoints[0].endpoint, "ticker");
        assert_eq!(
            snapshot.providers[1].endpoints[0].error.as_deref(),
            Some("timeout")
        );
    }

    fn endpoint(endpoint: &str, required: bool, ok: bool, error: Option<&str>) -> EndpointEvidence {
        EndpointEvidence {
            endpoint: endpoint.to_string(),
            required,
            ok,
            error: error.map(ToString::to_string),
            payload: None,
        }
    }
}
