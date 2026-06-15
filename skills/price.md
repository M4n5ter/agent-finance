# agent-finance price skill

## Default Price

Use `price` to answer "what is it trading at now?":

```bash
agent-finance price CRDO
agent-finance price CRDO --json
```

The default output includes current observable price, session, provider, local timestamp, UTC fields in JSON, change from regular-market previous close, and regular-market open/high/low/volume when available.

## Session Split

Use `sessions` when the task asks about premarket, postmarket, overnight, BOATS, platform 24h prices, or provider disagreement:

```bash
agent-finance sessions CRDO
agent-finance sessions LITE --proxy-symbol LITEUSDT
```

## Proxy Prices

If a Binance TradFi / futures contract exists, add `--proxy-symbol`:

```bash
agent-finance sessions SPCX --proxy-symbol SPCXUSDT
```

Proxy prices are price-discovery and sentiment signals. They are not the legal equity, pre-IPO ownership, or broker-fill price.

## Streaming

```bash
agent-finance stream CRDO --messages 5
agent-finance watch CRDO --interval-seconds 15 --iterations 4
```

Use `watch` when WebSocket streaming is blocked by the local network.
