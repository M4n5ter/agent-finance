#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const result = spawnSync("cargo", ["build", "--release", "--locked"], {
  cwd: root,
  stdio: "inherit",
});

if (result.error) {
  if (result.error.code === "ENOENT") {
    console.error("agent-finance-cli requires Rust/Cargo to build during npm install.");
    console.error("Install Rust from https://rustup.rs/ and then reinstall this package.");
    process.exit(127);
  }
  console.error(result.error.message);
  process.exit(1);
}

if (result.signal) {
  process.kill(process.pid, result.signal);
}

process.exit(result.status ?? 1);
