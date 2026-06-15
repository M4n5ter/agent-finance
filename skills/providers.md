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
- History: use `history --provider auto|yahoo|stooq|robinhood|binance-futures`.
- Research: `fundamentals/events --provider auto` combines useful no-key sources when available.
- SEC EDGAR is official for filings and XBRL facts, not market quotes, options, analyst estimates, or news aggregation.
- Robinhood and CNBC are partial no-key sources; use them as cross-checks, not replacements for official filings or primary disclosures.
- Stooq live can provide no-key daily/weekly/monthly history; intraday bulk data requires explicit imported ZIP cache.
- Binance futures / TradFi perps are proxy/derivative instruments.

## Browser Boundary

The CLI uses HTTP requests with browser-like TLS behavior where possible, but it is not a full browser. Dynamic, login-gated, screenshot-sensitive, or noisy pages require a real browser tool.
