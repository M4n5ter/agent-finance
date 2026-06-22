# agent-finance providers skill

## Capability Matrix

Always inspect provider coverage instead of guessing from provider names:

```bash
agent-finance providers
agent-finance providers --json
```

## Provider Rules

- Quotes: use `price SYMBOL` first. Only force a provider when cross-checking.
- Session split: use `sessions SYMBOL`.
- History: use `history --provider auto|yahoo|stooq|robinhood|binance-spot|binance-usds-futures`.
- Research: `fundamentals/events --provider auto` combines useful no-key sources when available.
- SEC EDGAR is official for filings and XBRL facts, not market quotes, options, analyst estimates, or news aggregation.
- Robinhood and CNBC are partial no-key sources; use them as cross-checks, not replacements for official filings or primary disclosures.
- Stooq live can provide no-key daily/weekly/monthly history; intraday bulk data requires explicit imported ZIP cache.
- Binance Spot and USD-M Futures are tier-1 crypto market-data providers. Use `agent-finance skills get crypto`.
- Binance uses local clients against official public REST/WebSocket paths, not the generated Binance SDK.
- Binance USD-M futures / TradFi perps are derivative instruments and proxy price-discovery sources, not legal equity.
- Polymarket is a prediction-market sentiment source. Use `polymarket search` and `polymarket market` for implied probability, orderbook, liquidity, OI, holder preview rows, and probability history; do not use it as an equity quote or primary-source fact.

## Browser Boundary

The CLI uses HTTP requests with browser-like TLS behavior where possible, but it is not a full browser. Dynamic, login-gated, screenshot-sensitive, or noisy pages require a real browser tool. Polymarket uses the official SDK by default; explicit `--proxy` or `--no-proxy` uses public REST fallback through the CLI HTTP stack.
