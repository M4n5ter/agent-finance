#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const path = require("node:path");

const { localBuildBinary, platformKey, platformPackageName } = require("./platform");
const { resolveBinary } = require("./resolve-binary");

const root = path.resolve(__dirname, "..");

if (resolveBinary({ silent: true })) {
  process.exit(0);
}

const packageName = platformPackageName();
if (!packageName) {
  console.error(`agent-finance-cli does not publish a prebuilt binary for ${platformKey()}.`);
} else {
  console.error(`Prebuilt package ${packageName} was not installed.`);
}
console.error("Falling back to local Rust build.");

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

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

if (!resolveBinary({ silent: true }) && !require("node:fs").existsSync(localBuildBinary(root))) {
  console.error("Cargo build completed but agent-finance binary was not found.");
  process.exit(1);
}

process.exit(0);
