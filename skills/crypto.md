# agent-finance crypto skill

Use this when crypto markets, Binance spot, Binance USD-M futures, funding, open interest, long/short ratios, taker flow, basis, or 24/7 crypto price discovery matter.

## Start

```bash
agent-finance crypto snapshot BTC/USDT
agent-finance crypto sentiment BTCUSDT
agent-finance price BTC/USDT --asset crypto
agent-finance history BTC/USDT --asset crypto --interval 1h --limit 48
```

## Low-Level Evidence

```bash
agent-finance crypto spot ticker BTCUSDT
agent-finance crypto spot ticker24h BTCUSDT
agent-finance crypto spot book BTCUSDT --limit 20
agent-finance crypto spot trades BTCUSDT --aggregate --limit 20
agent-finance crypto spot klines BTCUSDT --interval 1m --limit 60

agent-finance crypto futures ticker BTCUSDT
agent-finance crypto futures mark BTCUSDT
agent-finance crypto futures funding BTCUSDT --limit 8
agent-finance crypto futures open-interest BTCUSDT
agent-finance crypto futures ratios BTCUSDT --period 5m --limit 30
agent-finance crypto futures flow BTCUSDT --period 5m --limit 30
agent-finance crypto futures basis BTCUSDT --period 5m --limit 30

agent-finance crypto stream BTCUSDT --kind trade --messages 1
agent-finance crypto stream BTCUSDT --market usds-futures --kind mark-price --messages 1
```

## Rules

- Binance is a tier-1 crypto market-data provider in this CLI.
- Binance integration uses self-maintained clients for official public REST and WebSocket paths; do not add the generated Binance SDK unless a future version proves cleaner than these local abstractions.
- Spot WebSocket uses Binance's market-data-only `data-stream.binance.vision` endpoint because this CLI only needs public market data.
- USD-M Futures WebSocket routes streams through Binance's current `/market/ws` and `/public/ws` paths; do not route futures streams through the legacy root `/ws` path.
- Prefer `crypto snapshot` for current observable market state.
- Prefer `crypto sentiment` for futures leverage, funding, open interest, long/short, taker flow, and basis.
- Use `--json` for downstream computation and `--raw` when auditing provider payloads.
- `BINANCE_API_KEY` is only for read-only market-data endpoints. This CLI must not read Binance secrets, sign requests, or use account/trading endpoints.
- Crypto trades 24/7; do not apply equity regular/pre/post/overnight session assumptions.
- USD-M futures and TradFi perps are derivatives. They are useful for price discovery and sentiment, not legal equity or broker-fill prices.
