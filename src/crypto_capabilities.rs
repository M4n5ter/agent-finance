use crate::cli::{CryptoDiscoverKind, CryptoInstrument, CryptoMarket, CryptoProvider};
use crate::model::{ProviderCapability, ProviderProfile};

#[derive(Clone, Copy, Debug)]
pub enum CryptoCapability {
    Quote,
    Book,
    Trades,
    Candles,
    Funding,
    OpenInterest,
    Discover(CryptoDiscoverKind),
}

impl CryptoCapability {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Quote => "quote",
            Self::Book => "book",
            Self::Trades => "trades",
            Self::Candles => "candles",
            Self::Funding => "funding",
            Self::OpenInterest => "open-interest",
            Self::Discover(_) => "discover",
        }
    }

    pub const fn default_instrument(self) -> CryptoInstrument {
        match self {
            Self::Funding | Self::OpenInterest => CryptoInstrument::Swap,
            _ => CryptoInstrument::Spot,
        }
    }
}

pub fn resolve_instrument(
    instrument: CryptoInstrument,
    capability: CryptoCapability,
) -> CryptoInstrument {
    match instrument {
        CryptoInstrument::Auto => capability.default_instrument(),
        instrument => instrument,
    }
}

pub fn selected_providers(
    provider: CryptoProvider,
    instrument: CryptoInstrument,
    capability: CryptoCapability,
) -> Vec<CryptoProvider> {
    match provider {
        CryptoProvider::Auto => [
            CryptoProvider::Binance,
            CryptoProvider::Coinbase,
            CryptoProvider::Okx,
            CryptoProvider::Coingecko,
        ]
        .into_iter()
        .filter(|candidate| provider_supports(*candidate, instrument, capability))
        .collect(),
        provider => vec![provider],
    }
}

pub fn provider_supports(
    provider: CryptoProvider,
    instrument: CryptoInstrument,
    capability: CryptoCapability,
) -> bool {
    match capability {
        CryptoCapability::Quote | CryptoCapability::Candles => match instrument {
            CryptoInstrument::Spot => matches!(
                provider,
                CryptoProvider::Binance
                    | CryptoProvider::Coinbase
                    | CryptoProvider::Okx
                    | CryptoProvider::Coingecko
            ),
            CryptoInstrument::Swap | CryptoInstrument::Futures => {
                matches!(provider, CryptoProvider::Binance | CryptoProvider::Okx)
            }
            CryptoInstrument::Option => matches!(provider, CryptoProvider::Okx),
            CryptoInstrument::Auto => false,
        },
        CryptoCapability::Book => match instrument {
            CryptoInstrument::Spot => matches!(
                provider,
                CryptoProvider::Binance | CryptoProvider::Coinbase | CryptoProvider::Okx
            ),
            CryptoInstrument::Swap | CryptoInstrument::Futures => {
                matches!(provider, CryptoProvider::Binance | CryptoProvider::Okx)
            }
            CryptoInstrument::Option => matches!(provider, CryptoProvider::Okx),
            CryptoInstrument::Auto => false,
        },
        CryptoCapability::Trades => match instrument {
            CryptoInstrument::Spot => matches!(
                provider,
                CryptoProvider::Binance | CryptoProvider::Coinbase | CryptoProvider::Okx
            ),
            CryptoInstrument::Swap | CryptoInstrument::Futures => {
                matches!(provider, CryptoProvider::Binance | CryptoProvider::Okx)
            }
            CryptoInstrument::Option => matches!(provider, CryptoProvider::Okx),
            CryptoInstrument::Auto => false,
        },
        CryptoCapability::Funding => {
            instrument == CryptoInstrument::Swap
                && matches!(provider, CryptoProvider::Binance | CryptoProvider::Okx)
        }
        CryptoCapability::OpenInterest => match instrument {
            CryptoInstrument::Swap => {
                matches!(provider, CryptoProvider::Binance | CryptoProvider::Okx)
            }
            CryptoInstrument::Futures | CryptoInstrument::Option => {
                matches!(provider, CryptoProvider::Okx)
            }
            CryptoInstrument::Auto | CryptoInstrument::Spot => false,
        },
        CryptoCapability::Discover(kind) => discover_provider_supports(provider, instrument, kind),
    }
}

pub fn binance_market(instrument: CryptoInstrument) -> CryptoMarket {
    match instrument {
        CryptoInstrument::Spot | CryptoInstrument::Auto => CryptoMarket::Spot,
        CryptoInstrument::Swap | CryptoInstrument::Futures => CryptoMarket::UsdsFutures,
        CryptoInstrument::Option => CryptoMarket::Spot,
    }
}

pub fn crypto_provider_profiles() -> Vec<ProviderProfile> {
    vec![
        crypto_profile(
            "coinbase",
            "exchange-api",
            "Coinbase Exchange no-key crypto market data for products, tickers, stats, books, trades, and candles.",
            &[
                crypto_cap("quote", "yes", "ticker plus 24h stats and product metadata"),
                crypto_cap("history", "yes", "candles"),
                crypto_cap("order book", "yes", "level 1/2/3 product book"),
                crypto_cap("trades", "yes", "product trades"),
                crypto_cap("markets", "yes", "products and volume summary"),
                crypto_cap(
                    "funding/open interest",
                    "no",
                    "spot exchange; no perps funding/OI in this provider",
                ),
                crypto_cap("research", "no", "No issuer fundamentals/analysis"),
            ],
            &[
                "Use BTC-USD style symbols when a USD product exists; BTC/USDT may not exist on Coinbase.",
                "Good independent cross-check for spot price, book depth, and recent prints.",
            ],
        ),
        crypto_profile(
            "okx",
            "exchange-api",
            "OKX no-key crypto market data for spot/swap/futures instruments, tickers, books, trades, candles, funding, mark price, and open interest.",
            &[
                crypto_cap("quote", "yes", "ticker last price"),
                crypto_cap("history", "yes", "candles and historical candles"),
                crypto_cap("order book", "yes", "books"),
                crypto_cap("trades", "yes", "recent trades"),
                crypto_cap("markets", "yes", "instruments and tickers by instType"),
                crypto_cap("funding", "yes", "current and historical funding rates"),
                crypto_cap("open interest", "yes", "open interest plus mark price"),
                crypto_cap("research", "no", "No issuer fundamentals/analysis"),
            ],
            &[
                "Use --instrument spot/swap/futures/option on discovery and derivatives commands.",
                "Useful as a non-Binance derivative sentiment and price-discovery cross-check.",
            ],
        ),
        crypto_profile(
            "coingecko",
            "market-aggregator",
            "CoinGecko no-key aggregate crypto data for simple price, coin metadata, markets, tickers, OHLC, market charts, trending, global, exchanges, and derivatives discovery.",
            &[
                crypto_cap(
                    "quote",
                    "yes",
                    "simple price with market cap, volume, 24h change, plus coin metadata",
                ),
                crypto_cap("history", "yes", "OHLC and market chart windows"),
                crypto_cap(
                    "markets",
                    "yes",
                    "coins markets, coins list, and exchange tickers by coin",
                ),
                crypto_cap(
                    "trending/global",
                    "yes",
                    "trending search and global market data",
                ),
                crypto_cap("exchanges", "yes", "spot and derivatives exchange lists"),
                crypto_cap(
                    "funding/open interest",
                    "partial",
                    "derivatives discovery, not normalized per-symbol OI/funding",
                ),
                crypto_cap(
                    "research",
                    "partial",
                    "coin metadata, links, categories, and market aggregates",
                ),
            ],
            &[
                "Aggregator data is useful for breadth, trending, and cross-exchange context; verify execution-sensitive prices against exchange APIs.",
                "Free/no-key endpoints can be rate limited; COINGECKO_API_KEY or COINGECKO_DEMO_API_KEY is honored when present.",
            ],
        ),
    ]
}

fn crypto_profile(
    provider: &str,
    stability: &str,
    best_for: &str,
    capabilities: &[ProviderCapability],
    limitations: &[&str],
) -> ProviderProfile {
    ProviderProfile {
        provider: provider.to_string(),
        requires_api_key: false,
        official_status: "official-public-api".to_string(),
        stability: stability.to_string(),
        best_for: best_for.to_string(),
        large_download: false,
        capabilities: capabilities.to_vec(),
        limitations: limitations.iter().map(|value| value.to_string()).collect(),
    }
}

fn crypto_cap(module: &str, status: &str, note: &str) -> ProviderCapability {
    ProviderCapability {
        module: module.to_string(),
        status: status.to_string(),
        note: note.to_string(),
        implemented: true,
    }
}

fn discover_provider_supports(
    provider: CryptoProvider,
    instrument: CryptoInstrument,
    kind: CryptoDiscoverKind,
) -> bool {
    match kind {
        CryptoDiscoverKind::Markets => match instrument {
            CryptoInstrument::Spot => matches!(
                provider,
                CryptoProvider::Binance
                    | CryptoProvider::Coinbase
                    | CryptoProvider::Okx
                    | CryptoProvider::Coingecko
            ),
            CryptoInstrument::Swap | CryptoInstrument::Futures | CryptoInstrument::Option => {
                matches!(provider, CryptoProvider::Okx)
            }
            CryptoInstrument::Auto => false,
        },
        CryptoDiscoverKind::Instruments => match instrument {
            CryptoInstrument::Spot => matches!(
                provider,
                CryptoProvider::Binance | CryptoProvider::Coinbase | CryptoProvider::Okx
            ),
            CryptoInstrument::Swap | CryptoInstrument::Futures | CryptoInstrument::Option => {
                matches!(provider, CryptoProvider::Okx)
            }
            CryptoInstrument::Auto => false,
        },
        CryptoDiscoverKind::Tickers => matches!(provider, CryptoProvider::Okx),
        CryptoDiscoverKind::VolumeSummary => matches!(provider, CryptoProvider::Coinbase),
        CryptoDiscoverKind::Trending
        | CryptoDiscoverKind::Global
        | CryptoDiscoverKind::Exchanges
        | CryptoDiscoverKind::Derivatives
        | CryptoDiscoverKind::DerivativesExchanges
        | CryptoDiscoverKind::CoinsList => matches!(provider, CryptoProvider::Coingecko),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_quote_uses_exchange_and_aggregator_spot_sources() {
        let providers = selected_providers(
            CryptoProvider::Auto,
            CryptoInstrument::Spot,
            CryptoCapability::Quote,
        );

        assert_eq!(
            providers,
            vec![
                CryptoProvider::Binance,
                CryptoProvider::Coinbase,
                CryptoProvider::Okx,
                CryptoProvider::Coingecko,
            ]
        );
    }

    #[test]
    fn auto_swap_quote_uses_derivatives_exchanges_only() {
        let providers = selected_providers(
            CryptoProvider::Auto,
            CryptoInstrument::Swap,
            CryptoCapability::Quote,
        );

        assert_eq!(
            providers,
            vec![CryptoProvider::Binance, CryptoProvider::Okx]
        );
    }

    #[test]
    fn coingecko_is_not_a_trade_provider() {
        assert!(!provider_supports(
            CryptoProvider::Coingecko,
            CryptoInstrument::Spot,
            CryptoCapability::Trades,
        ));
    }

    #[test]
    fn derivative_market_discovery_uses_okx_only() {
        let providers = selected_providers(
            CryptoProvider::Auto,
            CryptoInstrument::Swap,
            CryptoCapability::Discover(CryptoDiscoverKind::Markets),
        );

        assert_eq!(providers, vec![CryptoProvider::Okx]);
    }

    #[test]
    fn default_instrument_follows_capability_shape() {
        assert_eq!(
            resolve_instrument(CryptoInstrument::Auto, CryptoCapability::Quote),
            CryptoInstrument::Spot
        );
        assert_eq!(
            resolve_instrument(CryptoInstrument::Auto, CryptoCapability::Funding),
            CryptoInstrument::Swap
        );
        assert_eq!(
            resolve_instrument(CryptoInstrument::Auto, CryptoCapability::OpenInterest),
            CryptoInstrument::Swap
        );
    }

    #[test]
    fn provider_profiles_do_not_claim_missing_crypto_runtime_capabilities() {
        for profile in crypto_provider_profiles() {
            let provider = match profile.provider.as_str() {
                "coinbase" => CryptoProvider::Coinbase,
                "okx" => CryptoProvider::Okx,
                "coingecko" => CryptoProvider::Coingecko,
                other => panic!("unmapped crypto provider profile: {other}"),
            };

            for capability in profile
                .capabilities
                .iter()
                .filter(|capability| capability.status == "yes")
            {
                for (instrument, runtime_capability) in profile_runtime_claims(&capability.module)
                    .unwrap_or_else(|| {
                        panic!(
                            "unmapped crypto profile capability {} for {}",
                            capability.module, profile.provider
                        )
                    })
                {
                    assert!(
                        provider_supports(provider, instrument, runtime_capability),
                        "{} claims {} but runtime does not support {:?} {:?}",
                        profile.provider,
                        capability.module,
                        instrument,
                        runtime_capability,
                    );
                }
            }
        }

        assert!(!provider_supports(
            CryptoProvider::Coingecko,
            CryptoInstrument::Spot,
            CryptoCapability::Trades,
        ));
    }

    fn profile_runtime_claims(module: &str) -> Option<Vec<(CryptoInstrument, CryptoCapability)>> {
        Some(match module {
            "quote" => vec![(CryptoInstrument::Spot, CryptoCapability::Quote)],
            "history" => vec![(CryptoInstrument::Spot, CryptoCapability::Candles)],
            "order book" => vec![(CryptoInstrument::Spot, CryptoCapability::Book)],
            "trades" => vec![(CryptoInstrument::Spot, CryptoCapability::Trades)],
            "markets" => vec![(
                CryptoInstrument::Spot,
                CryptoCapability::Discover(CryptoDiscoverKind::Markets),
            )],
            "funding" => vec![(CryptoInstrument::Swap, CryptoCapability::Funding)],
            "open interest" => vec![(CryptoInstrument::Swap, CryptoCapability::OpenInterest)],
            "trending/global" => vec![
                (
                    CryptoInstrument::Spot,
                    CryptoCapability::Discover(CryptoDiscoverKind::Trending),
                ),
                (
                    CryptoInstrument::Spot,
                    CryptoCapability::Discover(CryptoDiscoverKind::Global),
                ),
            ],
            "exchanges" => vec![(
                CryptoInstrument::Spot,
                CryptoCapability::Discover(CryptoDiscoverKind::Exchanges),
            )],
            _ => return None,
        })
    }
}
