---
name: profile
description: Configure agent-finance trading profiles, Binance HMAC env references, risk policy, intent-first live writes, audit logs, and safe AI Agent workflows.
---

# agent-finance profile skill

Use this skill before any `account`, `order`, `transfer`, `risk`, or `audit` command.
Also use it before USD-M futures `state` changes.

## Model

- A profile is a TOML file in the user config directory.
- The profile stores environment variable names for Binance HMAC keys, not secrets.
- The default HMAC secret env is `BINANCE_PRIVATE_KEY`; in Binance HMAC mode this is the API Secret string, not an RSA or Ed25519 private key.
- Live writes require all of these: profile `allow_live = true`, matching `[permissions]` declarations, the relevant order/transfer/futures-state whitelist, matching Binance API key permissions, intent id, and `--live`.
- Live market orders are blocked until risk notional can be derived from fresh exchange data instead of user-supplied `valuation_price`.
- USD-M futures leverage, margin type, and Binance futures account position mode changes require explicit `risk.allowed_futures_state_changes` policy and use separate `state` intents.
- Binance position mode changes every symbol; UM/CM share `dualSidePosition`, and Binance rejects the change when either side has open orders or open positions.
- Order, cancel, transfer, and futures state writes are intent-first. Create the intent, inspect it, run `risk check`, then submit.
- Audit logging is append-only JSONL in the user data directory.

## Profile Permissions

`[permissions]` declares what this profile is allowed to attempt before API-key probing:

```toml
[permissions]
spot_trading = true
usds_futures = true
universal_transfer = false
```

- `spot_trading`: required for Spot order and cancel intents.
- `usds_futures`: required for USD-M order/cancel intents and futures state changes.
- `universal_transfer`: required for Spot `<->` USD-M internal transfers.
- `profile doctor` reports both profile/risk consistency and live Binance API-key permission checks when HMAC env vars are set.
- `risk check` and submit block an intent when the matching profile permission is `false`, even if the risk whitelist would otherwise allow it.
- Profiles that omit `[permissions]` or omit individual fields parse with those permissions defaulting to `false`; this is fail-closed and `profile doctor` will report which declarations are missing for the risk policy.

## Setup

```bash
agent-finance profile path --profile default
agent-finance profile template --profile default
agent-finance profile doctor --profile default
agent-finance profile explain --profile default
agent-finance risk explain --profile default
agent-finance account permissions --profile default --json
agent-finance account balances --profile default --json
agent-finance account positions --profile default --json
```

Signed read JSON output is a `SignedReadSnapshot` envelope:

- `profile`, `provider`, `environment`, `kind`
- `request`: typed read request and scope
- `payload`: raw provider response for the requested signed read

| Command | `kind` | Common payload path |
| --- | --- | --- |
| `account permissions` | `api-permissions` | `payload` |
| `account balances` | `spot-balances` | `payload.balances` |
| `account positions` | `usds-futures-positions` | `payload.assets`, `payload.positions` |
| `order query` | `order-query` | `payload` |
| `order open` | `open-orders` | `payload` |
| `transfer history` | `transfer-history` | `payload.rows` |

## Order Flow

```bash
agent-finance order create BTCUSDT --profile default --market spot --side buy --kind limit --quantity 0.001 --price 50000 --time-in-force gtc
agent-finance order create BTCUSDT --profile default --market spot --side buy --kind limit-maker --quantity 0.001 --price 50000
agent-finance order create BTCUSDT --profile default --market spot --side buy --kind market --quantity 0.001 --valuation-price 50000
agent-finance risk check INTENT_ID --profile default
agent-finance order submit INTENT_ID --profile default
agent-finance order submit INTENT_ID --profile default --test
agent-finance order submit INTENT_ID --profile default --live
agent-finance order query BTCUSDT --profile default --market spot --client-order-id CLIENT_ORDER_ID --json
agent-finance order open --profile default --market spot --symbol BTCUSDT --json
```

Signed submit JSON output is a `SubmitSnapshot` envelope:

- `profile`, `provider`, `environment`, `intent_id`, `intent_kind`, `mode`, `risk`
- `execution.kind`: `plan`, `order-submit`, `cancel`, `transfer`, or `futures-state`
- `execution.payload`: dry-run request plan, exchange rule check, or raw provider execution payload

## Cancel Flow

`order cancel` creates a cancel intent; it does not cancel an exchange order until the intent is checked and submitted.

```bash
agent-finance order cancel BTCUSDT --profile default --market spot --client-order-id CLIENT_ORDER_ID
agent-finance risk check CANCEL_INTENT_ID --profile default
agent-finance order submit CANCEL_INTENT_ID --profile default
agent-finance order submit CANCEL_INTENT_ID --profile default --live
```

## Futures State Flow

Add Binance futures account position mode policy manually before using `--kind position-mode`; it is not included in the default profile template:

```toml
[[risk.allowed_futures_state_changes]]
kind = "position-mode"
mode = "hedge"
```

```bash
agent-finance state create --profile default --kind leverage --symbol BTCUSDT --leverage 2
agent-finance state create --profile default --kind margin-type --symbol BTCUSDT --margin-type isolated
agent-finance state create --profile default --kind position-mode --position-mode hedge
agent-finance risk check INTENT_ID --profile default --live
agent-finance state submit INTENT_ID --profile default
agent-finance state submit INTENT_ID --profile default --live
```

## Transfer Flow

```bash
agent-finance transfer create USDT --profile default --direction spot-to-usds-futures --amount 10
agent-finance risk check INTENT_ID --profile default
agent-finance transfer submit INTENT_ID --profile default
agent-finance transfer submit INTENT_ID --profile default --live
agent-finance transfer history --profile live --direction spot-to-usds-futures --size 20 --json
```

## Audit Flow

```bash
agent-finance audit tail --limit 20
agent-finance audit export --json
```

## Guardrails

- Never put API secrets in TOML, Markdown, command history, audit logs, or prompts.
- Use Binance testnet profiles first.
- For live profiles, keep whitelist and notional limits small.
- `profile doctor` first checks that `[permissions]` covers the risk policy, then reads Binance API restrictions when HMAC env vars are set and reports specific permission checks for spot trading, USD-M futures, and universal transfer.
- Missing `[permissions]` fields default to `false`; add explicit declarations instead of assuming older profiles are live-write capable.
- Live submit checks the required Binance API permissions before claiming the intent, so a permission failure does not consume the intent.
- `max_daily_order_notional_usdt` is enforced from the local append-only audit log for `risk check --live` and live order submit. Matching live-submit events with missing notional data fail closed.
- `order submit` without flags is an offline dry-run; `--test` calls an exchange test endpoint where available but does not consume the intent; only `--live` consumes the intent.
- `order submit --test` and `order submit --live` fetch Binance `exchangeInfo` and block orders that violate locally checkable symbol status, price tick, lot size, or notional filters. Dry-run is offline and prints the `exchangeInfo` request that will be checked later.
- Limit orders use `--price` as the exchange price. Spot `limit-maker` orders map to Binance `LIMIT_MAKER`, do not accept `--time-in-force`, and rely on the exchange to reject orders that would immediately take liquidity. Market orders use `--valuation-price` for risk notional checks and never send an exchange `price` parameter; exchange notional for market orders is reported as not locally checked because it depends on execution price.
- Live universal transfers require explicit `[[risk.allowed_transfers]]` entries with direction, asset, and max amount.
- Live futures state changes require explicit `[[risk.allowed_futures_state_changes]]` entries. Order submit does not change leverage, margin type, or position mode implicitly.
- Review `risk check` findings before live position-mode submit; the CLI warns that Binance applies it account-wide across every symbol and that UM/CM share the setting.
- Transfer history reads Binance SAPI live account data and requires a reviewed live profile.
- Do not use this CLI for withdrawals, margin, COIN-M, options, earn, or external transfers.
