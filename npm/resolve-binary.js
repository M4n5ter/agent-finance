const fs = require("node:fs");
const path = require("node:path");

const { executableName, localBuildBinary, platformPackageName } = require("./platform");

function resolveBinary(options = {}) {
  const root = path.resolve(__dirname, "..");
  const packageName = platformPackageName();

  if (packageName) {
    try {
      const packageJson = require.resolve(`${packageName}/package.json`, {
        paths: [root],
      });
      const binary = path.join(path.dirname(packageJson), "bin", executableName());
      if (fs.existsSync(binary)) {
        return binary;
      }
    } catch (error) {
      if (!isMissingPackage(error) && !options.silent) {
        throw error;
      }
    }
  }

  const fallback = localBuildBinary(root);
  if (fs.existsSync(fallback)) {
    return fallback;
  }

  return undefined;
}

function isMissingPackage(error) {
  return error && error.code === "MODULE_NOT_FOUND";
}

module.exports = { resolveBinary };
