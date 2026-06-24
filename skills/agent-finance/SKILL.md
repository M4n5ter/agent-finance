---
name: agent-finance
description: agent-finance is an AI-agent-first market intelligence CLI for evidence-driven financial research. Use when Codex or another AI agent needs current quotes, regular/pre/post/overnight session splits, crypto market data, OHLCV history, indicators, prediction-market sentiment, provider capability discovery, no-key research payloads, URL text extraction, polling, or WebSocket streams.
hidden: true
---

# agent-finance

`agent-finance` is a finance and market-data CLI built for AI agents.

## CLI Availability

The npm package name is `agent-finance-cli`; the installed command is `agent-finance`.
If `agent-finance` is not available on `PATH`, install it with:

```bash
npm install -g agent-finance-cli
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
