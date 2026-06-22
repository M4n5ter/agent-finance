use crate::cli::{Provider, ResearchProvider};
use crate::model::{ProviderCapability, ProviderProfile};

pub fn profiles() -> Vec<ProviderProfile> {
    vec![
        profile(
            ResearchProvider::Auto.label(),
            false,
            "composite",
            "composite",
            "Default router; selects the most useful no-key source by module.",
            &[
                cap(
                    "quote",
                    "yes",
                    "Yahoo BOATS/extended/regular, then Stooq fallback",
                ),
                cap("history", "yes", "Yahoo, then Stooq fallback"),
                cap(
                    "fundamentals",
                    "partial",
                    "Yahoo quoteSummary + SEC companyfacts/submissions + Robinhood/CNBC cross-check",
                ),
                cap(
                    "events",
                    "partial",
                    "Yahoo calendar/secFilings + SEC submissions + Robinhood splits/market hours",
                ),
                cap(
                    "analysis",
                    "yahoo-only",
                    "No stable no-key replacement is currently implemented",
                ),
                cap(
                    "options",
                    "partial",
                    "Yahoo optionChain + Robinhood chain/contract metadata",
                ),
                cap(
                    "ownership",
                    "yahoo-only",
                    "No stable no-key replacement is currently implemented",
                ),
                cap(
                    "news",
                    "yahoo-only",
                    "CNBC/Nasdaq public pages are useful browser research targets, but not stable CLI providers yet",
                ),
                cap("search", "yahoo-only", "Yahoo Finance search"),
                cap("screen", "yahoo-only", "Yahoo predefined screeners"),
            ],
            &["Composite source; always inspect provider/source fields per module."],
        ),
        profile(
            ResearchProvider::Yahoo.label(),
            false,
            "unofficial-public-endpoint",
            "unofficial",
            "Broadest no-key research data source.",
            &[
                cap("quote", "yes", "chart/v7 quote"),
                cap("history", "yes", "chart OHLCV"),
                cap("extended sessions", "yes", "includePrePost and BOATS quote"),
                cap("fundamentals", "yes", "quoteSummary modules"),
                cap(
                    "analysis",
                    "yes",
                    "analyst targets/recommendations/estimates",
                ),
                cap("options", "yes", "option expiries and chains"),
                cap("ownership", "yes", "holders and insider modules"),
                cap("events", "yes", "calendarEvents/secFilings/earnings"),
                cap("news", "yes", "finance search news"),
                cap("search", "yes", "finance search"),
                cap("screen", "yes", "predefined screeners"),
            ],
            &[
                "Not an official stable API; verify key facts against company releases, SEC filings, and primary text.",
            ],
        ),
        profile(
            ResearchProvider::SecEdgar.label(),
            false,
            "official-api",
            "official",
            "Official filings, submissions, and XBRL companyfacts.",
            &[
                cap("filings", "yes", "submissions API"),
                cap("companyfacts", "yes", "XBRL companyfacts API"),
                cap(
                    "fundamentals",
                    "partial",
                    "Official disclosure facts; no valuation or analyst data",
                ),
                cap(
                    "events",
                    "partial",
                    "Recent filings; no Yahoo earnings-calendar estimates",
                ),
                cap("quote", "no", "SEC does not provide market quotes"),
                cap("history", "no", "SEC does not provide OHLCV history"),
                cap("analysis", "no", "SEC does not provide analyst targets"),
                cap("options", "no", "SEC does not provide option chains"),
                cap("news", "no", "SEC does not provide news aggregation"),
                cap(
                    "search",
                    "no",
                    "Only ticker-to-CIK and company filings are implemented here",
                ),
                cap("screen", "no", "SEC does not provide stock screeners"),
            ],
            &[
                "Fields come from filed XBRL and differ from Yahoo financialData definitions; preserve provenance.",
            ],
        ),
        profile(
            Provider::Stooq.label(),
            false,
            "official-public-download",
            "public-html/csv",
            "Delayed quotes, no-key HTML history tables, and explicitly imported bulk OHLCV.",
            &[
                cap("quote", "yes", "delayed CSV"),
                cap(
                    "history",
                    "yes",
                    "daily HTML table; weekly/monthly are aggregated from daily rows; CSV can use STOOQ_API_KEY",
                ),
                cap("catalog", "yes", "Official daily/hourly/5min bulk catalog"),
                cap(
                    "bulk history",
                    "yes",
                    "Read hourly/5min after explicit sync from captcha-authorized URL or local ZIP",
                ),
                cap("research", "no", "No fundamentals/analysis/options/news"),
            ],
            &[
                "CSV downloads require a captcha-issued API key; no-key live history uses web tables.",
                "Web tables can hit Stooq daily site limits; use STOOQ_API_KEY or bulk sync for stable batch history.",
                "Useful as price backup and historical-data base, not as a research-data provider.",
            ],
        ),
        profile(
            Provider::CnbcExtended.label(),
            false,
            "unofficial-public-endpoint",
            "public-endpoint",
            "pre/post extended quote cross-check.",
            &[
                cap("quote", "yes", "ExtendedMktQuote"),
                cap("history", "no", "History is not currently implemented"),
                cap(
                    "research",
                    "no",
                    "Use cnbc for research data; page evidence still belongs in a browser",
                ),
            ],
            &["Use for extended-price cross-checking, not as a complete research source."],
        ),
        profile(
            ResearchProvider::Cnbc.label(),
            false,
            "unofficial-public-endpoint",
            "public-endpoint",
            "CNBC quote payload fundamentals-lite valuation cross-check.",
            &[
                cap(
                    "fundamentals-lite",
                    "yes",
                    "PE, forward PE, market cap, beta, TTM revenue, margins, and other quote payload fields",
                ),
                cap(
                    "quote",
                    "partial",
                    "payload includes quote fields; extended-hours quote command uses cnbc-extended",
                ),
                cap("history", "no", "History is not currently implemented"),
                cap(
                    "research",
                    "partial",
                    "CLI provider covers fundamentals-lite; full news and page evidence still require a browser",
                ),
            ],
            &["Use for fundamentals-lite cross-checking, not as a complete research source."],
        ),
        profile(
            Provider::Robinhood.label(),
            false,
            "unofficial-public-endpoint",
            "public-endpoint",
            "extended-hours quote, instrument/fundamentals, minute history, and option metadata cross-check.",
            &[
                cap("quote", "yes", "public quote"),
                cap("history", "yes", "public historicals endpoint"),
                cap(
                    "fundamentals",
                    "partial",
                    "instrument profile + fundamentals endpoint",
                ),
                cap("events", "partial", "splits and market hours"),
                cap(
                    "options",
                    "partial",
                    "chain expirations and contract metadata",
                ),
                available_cap(
                    "option quotes",
                    "auth-limited",
                    "marketdata options endpoints may require auth; expose as coverage gap when blocked",
                ),
            ],
            &["Use for extended-hours price checks."],
        ),
        profile(
            "polymarket",
            false,
            "official-sdk",
            "official-public-api",
            "Prediction-market sentiment, implied probability, orderbook, liquidity, open interest, holder previews, and probability history.",
            &[
                cap(
                    "prediction search",
                    "yes",
                    "Gamma public relevance search for events and markets",
                ),
                cap(
                    "market detail",
                    "yes",
                    "Gamma market metadata and outcome probabilities",
                ),
                cap(
                    "orderbook",
                    "yes",
                    "CLOB public best bid/ask and depth by outcome token",
                ),
                cap(
                    "probability history",
                    "yes",
                    "CLOB prices-history by outcome token",
                ),
                cap(
                    "open interest",
                    "yes",
                    "Data API open interest by condition id",
                ),
                cap(
                    "holders",
                    "preview",
                    "Data API top holder preview rows by condition id; not total holder count",
                ),
                cap("quote", "no", "Prediction prices are not equity quotes"),
                cap(
                    "fundamentals",
                    "no",
                    "Does not replace SEC, IR, or company filings",
                ),
            ],
            &[
                "Use as quantifiable sentiment and event-probability evidence only.",
                "Default transport uses the official SDK; explicit --proxy/--no-proxy uses public REST fallback through the CLI HTTP stack.",
            ],
        ),
        profile(
            Provider::BinanceFutures.label(),
            false,
            "official-api",
            "exchange-api",
            "TradFi / futures / pre-IPO proxy contract price discovery.",
            &[
                cap("quote", "yes", "USD-M futures ticker/mark price"),
                cap("history", "yes", "klines"),
                cap(
                    "proxy metrics",
                    "yes",
                    "24h ticker、mark、open interest、funding",
                ),
                cap("research", "no", "No company fundamentals/analysis"),
            ],
            &["Proxy price is not the stock or pre-IPO legal-equity price."],
        ),
    ]
}

fn profile(
    provider: &str,
    requires_api_key: bool,
    official_status: &str,
    stability: &str,
    best_for: &str,
    capabilities: &[ProviderCapability],
    limitations: &[&str],
) -> ProviderProfile {
    ProviderProfile {
        provider: provider.to_string(),
        requires_api_key,
        official_status: official_status.to_string(),
        stability: stability.to_string(),
        best_for: best_for.to_string(),
        large_download: provider == Provider::Stooq.label(),
        capabilities: capabilities.to_vec(),
        limitations: limitations.iter().map(|value| value.to_string()).collect(),
    }
}

fn cap(module: &str, status: &str, note: &str) -> ProviderCapability {
    implemented_cap(module, status, note, true)
}

fn available_cap(module: &str, status: &str, note: &str) -> ProviderCapability {
    implemented_cap(module, status, note, false)
}

fn implemented_cap(
    module: &str,
    status: &str,
    note: &str,
    implemented: bool,
) -> ProviderCapability {
    ProviderCapability {
        module: module.to_string(),
        status: status.to_string(),
        note: note.to_string(),
        implemented,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sec_edgar_is_official_but_not_a_market_or_options_provider() {
        let profiles = profiles();
        let sec = profiles
            .iter()
            .find(|profile| profile.provider == "sec-edgar")
            .expect("sec-edgar profile");

        assert!(!sec.requires_api_key);
        assert_eq!(sec.stability, "official");
        assert!(sec.capabilities.iter().any(|capability| {
            capability.module == "companyfacts" && capability.status == "yes"
        }));
        assert!(
            sec.capabilities
                .iter()
                .any(|capability| { capability.module == "quote" && capability.status == "no" })
        );
        assert!(
            sec.capabilities
                .iter()
                .any(|capability| { capability.module == "options" && capability.status == "no" })
        );
    }

    #[test]
    fn auto_marks_research_breadth_as_partial_not_full_replacement() {
        let profiles = profiles();
        let auto = profiles
            .iter()
            .find(|profile| profile.provider == "auto")
            .expect("auto profile");

        assert!(auto.capabilities.iter().any(|capability| {
            capability.module == "fundamentals" && capability.status == "partial"
        }));
        assert!(auto.capabilities.iter().any(|capability| {
            capability.module == "analysis" && capability.status == "yahoo-only"
        }));
    }
}
