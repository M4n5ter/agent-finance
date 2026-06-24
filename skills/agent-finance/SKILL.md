---
name: agent-finance
description: Use agent-finance for evidence-driven financial research that needs current quotes, regular/pre/post/overnight session splits, crypto market data, OHLCV history, indicators, prediction-market sentiment, provider capability discovery, no-key research payloads, URL text extraction, polling, or WebSocket streams.
---

# agent-finance

`agent-finance` is a finance and market-data CLI for evidence-driven market work.

## CLI Availability

The npm package name is `agent-finance-cli`; the installed command is `agent-finance`.
If `agent-finance` is missing or stale, install or update the npm package:

```bash
npm install -g agent-finance-cli@latest
agent-finance --version
```

This file is a discovery stub. Before data collection, load the runtime guide from the installed CLI:

```bash
agent-finance skills get core
agent-finance skills get core --full
agent-finance skills list
```

## Task Skills

Load a narrower skill for the task:

```bash
agent-finance skills get price
agent-finance skills get history-indicators
agent-finance skills get crypto
agent-finance skills get research-data
agent-finance skills get providers
agent-finance skills get prediction-markets
agent-finance skills get profile
```

## Operating Rules

- Use `market price` for the default current observable price.
- Use `market sessions` when regular, premarket, postmarket, overnight, provider differences, or proxy prices matter.
- Inspect daily and minute history before trading, order-quality, stop-loss, or take-profit conclusions.
- Use `--json` when another agent or script will consume the output.
- Treat crypto and prediction-market data as market evidence, not primary company facts.
- Load `skills get profile` before signed account, order, transfer, futures state, risk, or audit workflows.
- Use a real browser tool for login-gated, dynamic, screenshot-sensitive, X/Reddit, brokerage, or extraction-suspicious pages.
