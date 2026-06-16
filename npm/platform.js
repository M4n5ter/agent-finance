const path = require("node:path");

const PACKAGES = {
  "darwin-arm64": "agent-finance-cli-darwin-arm64",
  "darwin-x64": "agent-finance-cli-darwin-x64",
  "linux-arm64": "agent-finance-cli-linux-arm64",
  "linux-x64": "agent-finance-cli-linux-x64",
  "win32-x64": "agent-finance-cli-win32-x64",
};

function platformKey(platform = process.platform, arch = process.arch) {
  return `${platform}-${arch}`;
}

function platformPackageName(platform = process.platform, arch = process.arch) {
  return PACKAGES[platformKey(platform, arch)];
}

function executableName(platform = process.platform) {
  return platform === "win32" ? "agent-finance.exe" : "agent-finance";
}

function localBuildBinary(root, platform = process.platform) {
  return path.join(root, "target", "release", executableName(platform));
}

module.exports = {
  PACKAGES,
  executableName,
  localBuildBinary,
  platformKey,
  platformPackageName,
};
