use crate::providers::binance;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum MarketSymbolKind {
    Equity,
    Crypto,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct MarketSymbol {
    pub(crate) input: String,
    pub(crate) canonical_key: String,
    kind: MarketSymbolKind,
}

impl MarketSymbol {
    pub(crate) fn new(input: String) -> Self {
        let canonical_key = canonical_lookup_key(&input);
        let kind = if is_crypto_pair(&input, &canonical_key) {
            MarketSymbolKind::Crypto
        } else {
            MarketSymbolKind::Equity
        };
        Self {
            input,
            canonical_key,
            kind,
        }
    }

    pub(crate) fn is_crypto(&self) -> bool {
        self.kind == MarketSymbolKind::Crypto
    }
}

pub(crate) fn canonical_lookup_key(symbol: &str) -> String {
    binance::normalize_symbol(symbol).unwrap_or_else(|_| {
        symbol
            .trim()
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .collect::<String>()
            .to_uppercase()
    })
}

fn is_crypto_pair(input: &str, canonical_key: &str) -> bool {
    let has_explicit_pair_separator = input
        .trim()
        .chars()
        .any(|character| matches!(character, '/' | '-' | '_' | ':'));
    for quote in ["USDT", "USDC", "USD", "EUR", "GBP", "BTC", "ETH"] {
        if canonical_key
            .strip_suffix(quote)
            .filter(|base| !base.is_empty())
            .filter(|base| has_explicit_pair_separator || is_common_crypto_base(base))
            .is_some()
        {
            return true;
        }
    }
    false
}

fn is_common_crypto_base(base: &str) -> bool {
    matches!(
        base,
        "BTC"
            | "ETH"
            | "BNB"
            | "SOL"
            | "XRP"
            | "ADA"
            | "DOGE"
            | "AVAX"
            | "LINK"
            | "LTC"
            | "BCH"
            | "DOT"
            | "MATIC"
            | "TON"
            | "TRX"
            | "SHIB"
            | "PEPE"
            | "UNI"
            | "AAVE"
            | "NEAR"
            | "ATOM"
            | "ETC"
            | "FIL"
            | "INJ"
            | "OP"
            | "ARB"
            | "SUI"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_crypto_pair_spellings_route_to_crypto_symbol_kind() {
        assert!(MarketSymbol::new("BTC/USDT".to_string()).is_crypto());
        assert!(MarketSymbol::new("eth-usdc".to_string()).is_crypto());
        assert!(MarketSymbol::new("SOLUSD".to_string()).is_crypto());
        assert!(MarketSymbol::new("BTC_EUR".to_string()).is_crypto());
        assert!(MarketSymbol::new("ETH:BTC".to_string()).is_crypto());
        assert!(!MarketSymbol::new("AAPL".to_string()).is_crypto());
        assert!(!MarketSymbol::new("GBTC".to_string()).is_crypto());
        assert!(!MarketSymbol::new("ETHE".to_string()).is_crypto());
        assert!(!MarketSymbol::new("USD".to_string()).is_crypto());
    }
}
