# agent-finance full core skill

Read this when you need the full command map for `agent-finance`.

## Command Map

```bash
agent-finance skills list
agent-finance skills get core
agent-finance skills get price
agent-finance skills get research-data
agent-finance skills get providers
agent-finance skills get crypto
agent-finance skills get prediction-markets
agent-finance skills get history-indicators
```

## Price and Sessions

```bash
agent-finance price CRDO
agent-finance price CRDO MRVL --json
agent-finance sessions CRDO
agent-finance sessions LITE --proxy-symbol LITEUSDT
```

`price` answers the default current-price question. `sessions` compares regular/pre/post/overnight/provider/proxy sources.

## History and Indicators

```bash
agent-finance history CRDO --range 1mo --interval 1d
agent-finance history CRDO --range 5d --interval 1m --session extended --adjustment raw --no-actions
agent-finance history CRDO --range 1y --interval 1d --adjustment auto --repair
agent-finance indicators CRDO MRVL --limit 120
```

Use history before making order, fill, stop-loss, take-profit, or intraday trend judgments. Indicators are summaries; they do not replace the bar path.

## Research Data

```bash
agent-finance fundamentals CRDO
agent-finance fundamentals CRDO --provider sec-edgar
agent-finance fundamentals CRDO --provider robinhood
agent-finance fundamentals CRDO --provider cnbc
agent-finance analysis CRDO
agent-finance options CRDO
agent-finance options CRDO --provider robinhood --count 80
agent-finance ownership CRDO
agent-finance events CRDO --provider sec-edgar
agent-finance news CRDO
agent-finance read-url "https://www.sec.gov/Archives/edgar/data/0001807794/000162828026014017/crdo-20260131.htm"
agent-finance search "optical interconnect"
agent-finance screen day_gainers
```

Research reports include sources, modules, coverage gaps, highlights, and raw payloads in JSON mode.

## Providers and Proxy Data

```bash
agent-finance providers
agent-finance providers --json
agent-finance crypto snapshot BTC/USDT
agent-finance crypto sentiment BTCUSDT
agent-finance price BTC/USDT --asset crypto
```

Use `providers` as the source-of-truth coverage matrix. Binance crypto is tier-1 market data; USD-M futures / TradFi perps are derivative/proxy prices, not legal equity or broker-fill prices.

## Prediction Markets

```bash
agent-finance polymarket search "spacex ipo" --limit 5
agent-finance polymarket search "spcex" --limit 5
agent-finance polymarket market MARKET_ID_OR_SLUG --json
agent-finance skills get prediction-markets
```

Use Polymarket for quantifiable sentiment and event-probability signals. It does not replace SEC/IR/company releases, verified news, or equity quotes.

## Network and Browser Boundaries

The CLI respects `--proxy`, `AGENT_FINANCE_PROXY`, and standard proxy environment variables. It does not hardcode a local proxy.

Polymarket uses the official SDK by default. When `--proxy` or `--no-proxy` is explicit, it uses public REST fallback through the CLI HTTP stack so those network controls are honored.

`read-url` is a text extraction fallback. For dynamic, login-gated, screenshot-sensitive, or noisy pages, open the original page with an available real browser tool such as agent-browser or opencli.
