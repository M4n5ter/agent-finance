# agent-finance-cli

AI Agent-first CLI for no-key financial market data and research context.

`agent-finance` is designed for human-operated AI agents: the CLI can print its own task-specific skills, then provide current quotes, regular/pre/post/overnight session splits, OHLCV history, local indicators, proxy futures data, no-key research payloads, URL text extraction, provider capability notes, polling, and Yahoo WebSocket streams.

If you are an AI Agent, start here:

```bash
agent-finance skills get core
agent-finance skills list
```

## Install

From npm:

```bash
npm install -g agent-finance-cli
```

The npm package builds the Rust binary from source during install, so a working Rust toolchain is required.

From GitHub:

```bash
cargo install --git https://github.com/M4n5ter/agent-finance-cli
```

From a checkout:

```bash
cargo run --bin agent-finance -- skills get core
cargo run --bin agent-finance -- price CRDO
```

Future distribution targets include crates.io and Homebrew.

## Common Commands

```bash
# Current observable price + regular-market basis.
agent-finance price CRDO
agent-finance price CRDO MRVL --json

# Precise regular/pre/post/overnight/provider split.
agent-finance sessions CRDO
agent-finance sessions LITE --proxy-symbol LITEUSDT

# History, indicators, futures/proxy data, and streams.
agent-finance history CRDO --range 1mo --interval 1d
agent-finance history CRDO --range 5d --interval 1m --session extended --adjustment raw --no-actions
agent-finance indicators CRDO MRVL --limit 120
agent-finance futures SPCXUSDT
agent-finance stream CRDO --messages 5
agent-finance watch CRDO --interval-seconds 15 --iterations 4

# No-key research and discovery.
agent-finance fundamentals CRDO
agent-finance fundamentals CRDO --provider sec-edgar
agent-finance analysis CRDO
agent-finance options CRDO --provider robinhood --count 80
agent-finance ownership CRDO
agent-finance events CRDO --provider sec-edgar
agent-finance news CRDO
agent-finance read-url "https://www.sec.gov/Archives/edgar/data/0001807794/000162828026014017/crdo-20260131.htm"
agent-finance search "optical interconnect"
agent-finance screen day_gainers
agent-finance providers
```

## Agent Skills

The CLI ships Markdown skills so an AI Agent can learn the exact command surface for the installed version:

```bash
agent-finance skills list
agent-finance skills get core --full
agent-finance skills get price
agent-finance skills get research-data
agent-finance skills get providers
agent-finance skills get history-indicators
agent-finance skills get futures
```

## Data Source Rules

- `price SYMBOL` is the default answer to "what is it trading at now?" It returns the current observable price, session, regular-market basis, and local/UTC timestamps.
- `sessions SYMBOL` is for explicit regular/pre/post/overnight/provider comparisons.
- `history` defaults to adjusted prices and includes corporate actions unless disabled.
- `providers` is the source-of-truth capability matrix. Do not infer coverage from provider names.
- Binance futures / TradFi perps are derivative/proxy prices. They are useful for 24h price discovery, but they are not the legal equity, broker fill, or pre-IPO ownership price.
- `read-url` is an extraction fallback using direct/Jina/Defuddle readers. It is not a login-capable browser.
- Dynamic, login-gated, screenshot-sensitive, or noisy pages should be verified with a real browser tool. `agent-browser` and `opencli` are examples, not dependencies.

## Network Defaults

`agent-finance` respects explicit and environment proxy configuration:

```bash
agent-finance --proxy socks5h://127.0.0.1:7890 price CRDO
agent-finance --no-proxy price CRDO
```

Proxy precedence:

1. `--proxy`
2. `AGENT_FINANCE_PROXY`
3. `ALL_PROXY`
4. `HTTPS_PROXY`
5. `HTTP_PROXY`

No proxy is hardcoded by default.

SEC EDGAR requests use `AGENT_FINANCE_SEC_USER_AGENT` when set, otherwise a project-level user agent.

Disclaimer: this tool is not investment advice; data may be delayed, incomplete, or wrong, and users must verify important facts and follow source terms.
