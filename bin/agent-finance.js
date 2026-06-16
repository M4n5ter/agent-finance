#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const path = require("node:path");

const executable = process.platform === "win32" ? "agent-finance.exe" : "agent-finance";
const binary = path.resolve(__dirname, "..", "target", "release", executable);

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  if (result.error.code === "ENOENT") {
    console.error(
      "agent-finance binary was not found. Reinstall the package and ensure Rust/Cargo is available during npm install.",
    );
    process.exit(127);
  }
  console.error(result.error.message);
  process.exit(1);
}

if (result.signal) {
  process.kill(process.pid, result.signal);
}

process.exit(result.status ?? 1);
