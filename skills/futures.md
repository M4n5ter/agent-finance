# agent-finance futures and proxy skill

Use this for Binance USD-M futures / TradFi perps such as `SPCXUSDT` and `LITEUSDT`.

```bash
agent-finance futures SPCXUSDT --funding-limit 8
agent-finance futures LITEUSDT --funding-limit 8 --json
```

Fields:

- `last_price`: latest 24h ticker trade price.
- `mark_price`: risk and funding reference price.
- `index_price`: index reference price.
- `funding`: funding rate.
- `open_interest`: open interest.

Risk rule: proxy/perpetual contracts are derivatives. They are not the legal equity, pre-IPO ownership, or broker-fill price.
