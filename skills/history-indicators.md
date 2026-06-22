# agent-finance history and indicators skill

## History

```bash
agent-finance history LITE --provider auto --interval 1d --range 1mo --limit 30
agent-finance history LITE --interval 1m --range 5d --session extended --adjustment raw --no-actions --limit 200
agent-finance history LITE --interval 1d --range 1y --adjustment auto --repair --limit 252
agent-finance history AAPL --provider robinhood --interval 5m --range 1d --session extended --limit 80
agent-finance history BTC/USDT --asset crypto --provider binance-spot --interval 1h --limit 48
agent-finance history BTCUSDT --asset crypto --provider binance-usds-futures --interval 1d --limit 30
```

## Intervals

- Yahoo / Yahoo extended: `1m`, `2m`, `5m`, `15m`, `30m`, `60m`, `90m`, `1h`, `1d`, `5d`, `1wk`, `1mo`, `3mo`.
- Robinhood: `5m`, `10m`, `1h`, `1d`, `1w`.
- Stooq live: `1d`, `1w`, `1mo`.
- Stooq bulk cache: `5m`, `1h` after explicit import.
- Binance spot / USD-M futures: `1m`, `3m`, `5m`, `15m`, `30m`, `1h`, `2h`, `4h`, `6h`, `8h`, `12h`, `1d`, `3d`, `1w`, `1M`.

When unsure:

```bash
agent-finance history --help
agent-finance stooq sync --help
```

## Adjustments

- `--adjustment auto`: adjust OHLC and close using adjusted close.
- `--adjustment back`: adjust OHLC but keep raw close.
- `--adjustment raw`: keep raw OHLC and expose adjusted close separately.
- `--repair`: repair obvious 100x Yahoo price errors and mark repaired bars.

## Indicators

```bash
agent-finance indicators LITE AAOI --provider auto --limit 120
agent-finance indicators CRDO MRVL --session extended --interval 1m --range 5d --limit 200
```

Indicators are summaries. For fill quality, limit-order decisions, or intraday exits, inspect daily and minute bars directly.
