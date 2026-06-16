#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const binary = path.resolve(process.argv[2] || "");
if (!binary || !fs.existsSync(binary)) {
  fail(`binary does not exist: ${binary || "<missing>"}`);
}

const platform = process.platform;

if (platform === "linux") {
  run("file", [binary]);
  run("ldd", [binary]);
  run("readelf", ["-d", binary]);
} else if (platform === "darwin") {
  run("file", [binary]);
  run("otool", ["-L", binary]);
} else if (platform === "win32") {
  run(binary, ["--help"]);
  const dumpbin = spawnSync("where", ["dumpbin"], { encoding: "utf8" });
  if (dumpbin.status === 0) {
    run("dumpbin", ["/DEPENDENTS", binary]);
  } else {
    console.log("dumpbin is unavailable; executable smoke test completed");
  }
} else {
  run(binary, ["--help"]);
  console.log(`no platform-specific dynamic-link checker for ${platform}`);
}

function run(command, args) {
  const result = spawnSync(command, args, { encoding: "utf8" });
  if (result.stdout) {
    process.stdout.write(result.stdout);
  }
  if (result.stderr) {
    process.stderr.write(result.stderr);
  }
  if (result.status !== 0) {
    fail(`${command} ${args.join(" ")} failed with status ${result.status}`);
  }
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
