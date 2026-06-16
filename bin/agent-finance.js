#!/usr/bin/env node

const { spawnSync } = require("node:child_process");

const { resolveBinary } = require("../npm/resolve-binary");

const binary = resolveBinary();

if (!binary) {
  console.error(
    "agent-finance binary was not found. Reinstall agent-finance-cli, or install Rust/Cargo so the npm fallback build can run.",
  );
  process.exit(127);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

if (result.signal) {
  process.kill(process.pid, result.signal);
}

process.exit(result.status ?? 1);
