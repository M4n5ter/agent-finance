---
name: profile
description: Configure agent-finance trading profiles, Binance HMAC env references, risk policy, intent-first live writes, audit logs, and safe AI Agent workflows.
---

# agent-finance profile skill

Use this skill before any `account`, `order`, `transfer`, `risk`, or `audit` command.

## Model

- A profile is a TOML file in the user config directory.
- The profile stores environment variable names for Binance HMAC keys, not secrets.
- Live writes require all of these: profile `allow_live = true`, symbol/order/transfer whitelist, intent id, and `--live`.
- Order, cancel, and transfer writes are intent-first. Create the intent, inspect it, run `risk check`, then submit.
- Audit logging is append-only JSONL in the user data directory.

## Setup

```bash
agent-finance profile path --profile default
agent-finance profile template --profile default
agent-finance profile doctor --profile default
agent-finance profile explain --profile default
```

## Order Flow

```bash
agent-finance order intent BTCUSDT --profile default --market spot --side buy --kind limit --quantity 0.001 --price 50000 --time-in-force gtc
agent-finance order intent BTCUSDT --profile default --market spot --side buy --kind market --quantity 0.001 --valuation-price 50000
agent-finance risk check INTENT_ID --profile default
agent-finance order submit INTENT_ID --profile default
agent-finance order submit INTENT_ID --profile default --test
agent-finance order submit INTENT_ID --profile default --live
agent-finance order cancel-intent BTCUSDT --profile default --market spot --client-order-id CLIENT_ORDER_ID
```

## Transfer Flow

```bash
agent-finance transfer intent USDT --profile default --direction spot-to-usds-futures --amount 10
agent-finance risk check INTENT_ID --profile default
agent-finance transfer submit INTENT_ID --profile default
agent-finance transfer submit INTENT_ID --profile default --live
```

## Guardrails

- Never put API secrets in TOML, Markdown, command history, audit logs, or prompts.
- Use Binance testnet profiles first.
- For live profiles, keep whitelist and notional limits small.
- `order submit` without flags is an offline dry-run; `--test` calls an exchange test endpoint where available but does not consume the intent; only `--live` consumes the intent.
- Limit orders use `--price` as the exchange price. Market orders use `--valuation-price` for risk notional checks and never send an exchange `price` parameter.
- Live universal transfers require explicit `[[risk.allowed_transfers]]` entries with direction, asset, and max amount.
- Do not use this CLI for withdrawals, margin, COIN-M, options, earn, or external transfers.
