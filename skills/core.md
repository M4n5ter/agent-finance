# agent-finance core skill

This skill is printed by the `agent-finance` CLI. It is the first thing an AI Agent should read before using the tool.

## Start Here

```bash
agent-finance skills list
agent-finance skills get core --full
agent-finance providers
agent-finance skills get crypto
```

## Default Workflow

1. Current observable price:

```bash
agent-finance price CRDO
agent-finance price CRDO --json
```

2. Precise session/provider split:

```bash
agent-finance sessions CRDO
agent-finance sessions LITE --proxy-symbol LITEUSDT
```

3. History before a trading or order-quality conclusion:

```bash
agent-finance history LITE --interval 1d --range 1mo --adjustment auto --limit 30
agent-finance history LITE --interval 1m --range 5d --session extended --adjustment raw --no-actions --limit 120
```

4. Research data:

```bash
agent-finance fundamentals CRDO
agent-finance fundamentals CRDO --provider sec-edgar
agent-finance analysis CRDO
agent-finance options CRDO
agent-finance ownership CRDO
agent-finance events CRDO --provider sec-edgar
agent-finance news CRDO
agent-finance read-url "https://www.sec.gov/Archives/edgar/data/0001807794/000162828026014017/crdo-20260131.htm"
agent-finance search "optical interconnect"
agent-finance screen day_gainers
```

5. Prediction-market sentiment:

```bash
agent-finance polymarket search "spacex ipo" --limit 5
agent-finance polymarket market MARKET_ID_OR_SLUG
agent-finance skills get prediction-markets
```

6. Crypto market data:

```bash
agent-finance crypto snapshot BTC/USDT
agent-finance crypto sentiment BTCUSDT
agent-finance price BTC/USDT --asset crypto
agent-finance history BTC/USDT --asset crypto --interval 1h --limit 48
```

## Rules

- Use `price` for the default "what is the current price?" answer.
- Use `sessions` when premarket, postmarket, overnight, BOATS, provider differences, or proxy prices matter.
- Use both daily and minute history before judging fills, limit-order quality, stop placement, or intraday action.
- Use `providers --json` when an Agent needs a machine-readable capability matrix.
- Treat Binance crypto as a tier-1 24/7 market-data source. Spot is crypto spot; USD-M futures / TradFi perps are derivatives and proxy instruments.
- Treat Polymarket as quantifiable prediction-market sentiment and event-probability evidence only; it is not an equity quote or primary-source fact.
- `read-url` is a text extraction fallback, not a real browser. For dynamic, login-gated, screenshot-sensitive, or noisy pages, use an available browser tool such as agent-browser or opencli.
- JSON output preserves structured fields for downstream computation. Human output is for quick inspection.
