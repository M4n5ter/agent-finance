# agent-finance crypto skill

Use this when crypto markets, Binance/Coinbase/OKX/CoinGecko spot data, Binance/OKX derivatives data, funding, open interest, long/short ratios, taker flow, basis, or 24/7 crypto price discovery matter.

## Start

```bash
agent-finance crypto snapshot BTC/USDT
agent-finance crypto sentiment BTCUSDT
agent-finance price BTC/USDT --asset crypto
agent-finance history BTC/USDT --asset crypto --interval 1h --limit 48
agent-finance crypto quote BTC/USDT
agent-finance crypto book BTC/USDT --limit 20
agent-finance crypto candles BTC/USDT --interval 1h --limit 48
```

## Cross-Provider Evidence

Prefer these capability-first commands before forcing provider-specific deep endpoints:

```bash
agent-finance crypto quote BTC/USDT
agent-finance crypto quote BTC-USD --provider coinbase
agent-finance crypto book BTC/USDT --provider okx --limit 20
agent-finance crypto trades BTC/USDT --limit 20
agent-finance crypto candles BTC/USDT --provider coingecko --interval 1d --limit 30
agent-finance crypto funding BTCUSDT --provider auto --instrument swap --limit 8
agent-finance crypto open-interest BTCUSDT --provider okx --instrument swap
agent-finance crypto discover --provider coingecko --kind trending
agent-finance crypto discover --provider coingecko --kind global
agent-finance crypto discover --provider okx --kind instruments --instrument swap
agent-finance crypto discover --provider coinbase --kind volume-summary
```

`--provider auto` only queries providers that support the requested capability and instrument. Force `--provider` when auditing a specific provider.

## Instruments

```bash
agent-finance crypto quote BTC/USDT --instrument spot
agent-finance crypto book BTC/USDT --instrument spot --limit 20
agent-finance crypto candles BTC/USDT --instrument spot --interval 1m --limit 60
agent-finance crypto funding BTCUSDT --instrument swap --limit 8
agent-finance crypto open-interest BTCUSDT --instrument swap
agent-finance crypto stream BTCUSDT --kind trade --messages 1
agent-finance crypto stream BTCUSDT --instrument swap --kind mark-price --messages 1
```

## Rules

- Use capability-first crypto commands for all normal work; provider eligibility is determined by capability plus `--instrument`.
- Binance, Coinbase, OKX, and CoinGecko are tier-1 no-key crypto providers in this CLI, but they answer different questions.
- Binance/OKX are stronger for exchange microstructure and derivatives evidence. Coinbase is a spot exchange cross-check. CoinGecko is stronger for aggregate breadth, trending, metadata, and exchange discovery.
- Binance integration uses self-maintained clients for official public REST and WebSocket paths; do not add the generated Binance SDK unless a future version proves cleaner than these local abstractions.
- Spot WebSocket uses Binance's market-data-only `data-stream.binance.vision` endpoint because this CLI only needs public market data.
- USD-M Futures WebSocket routes streams through Binance's current `/market/ws` and `/public/ws` paths; do not route futures streams through the legacy root `/ws` path.
- Prefer `crypto snapshot` for current observable market state.
- Prefer `crypto sentiment` for futures leverage, funding, open interest, long/short, taker flow, and basis.
- Prefer `crypto quote/book/trades/candles/funding/open-interest/discover --json` when an Agent needs provider evidence for reasoning.
- Use `--json` for downstream computation and `--raw` when auditing provider payloads.
- `BINANCE_API_KEY` is only for read-only market-data endpoints. This CLI must not read Binance secrets, sign requests, or use account/trading endpoints.
- Crypto trades 24/7; do not apply equity regular/pre/post/overnight session assumptions.
- USD-M futures and TradFi perps are derivatives. They are useful for price discovery and sentiment, not legal equity or broker-fill prices.
