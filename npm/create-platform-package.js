#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");

const { executableName } = require("./platform");

const options = parseArgs(process.argv.slice(2));
const root = path.resolve(__dirname, "..");
const rootPackage = readJson(path.join(root, "package.json"));
const packageName = required(options, "package");
const os = required(options, "os");
const cpu = required(options, "cpu");
const binary = path.resolve(required(options, "binary"));
const outDir = path.resolve(required(options, "out"));

if (!fs.existsSync(binary)) {
  fail(`binary does not exist: ${binary}`);
}

fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(path.join(outDir, "bin"), { recursive: true });

const targetBinary = path.join(outDir, "bin", executableName(os));
fs.copyFileSync(binary, targetBinary);
fs.chmodSync(targetBinary, 0o755);

for (const file of ["LICENSE-MIT", "LICENSE-APACHE"]) {
  fs.copyFileSync(path.join(root, file), path.join(outDir, file));
}

fs.writeFileSync(
  path.join(outDir, "README.md"),
  `# ${packageName}\n\nPrebuilt ${os}/${cpu} binary package for \`agent-finance-cli\`.\n\nInstall the main package instead:\n\n\`\`\`bash\nnpm install -g agent-finance-cli\n\`\`\`\n`,
);

writeJson(path.join(outDir, "package.json"), {
  name: packageName,
  version: rootPackage.version,
  description: `Prebuilt ${os}/${cpu} binary for agent-finance-cli.`,
  license: rootPackage.license,
  repository: rootPackage.repository,
  homepage: rootPackage.homepage,
  bugs: rootPackage.bugs,
  os: [os],
  cpu: [cpu],
  bin: {
    "agent-finance": `bin/${executableName(os)}`,
  },
  files: ["bin/", "README.md", "LICENSE-MIT", "LICENSE-APACHE"],
  publishConfig: {
    access: "public",
  },
});

console.log(`created ${packageName} in ${outDir}`);

function parseArgs(args) {
  const parsed = {};
  for (let index = 0; index < args.length; index += 2) {
    const key = args[index];
    const value = args[index + 1];
    if (!key?.startsWith("--") || value === undefined) {
      fail(`invalid arguments: ${args.join(" ")}`);
    }
    parsed[key.slice(2)] = value;
  }
  return parsed;
}

function required(options, name) {
  const value = options[name];
  if (!value) {
    fail(`missing --${name}`);
  }
  return value;
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function writeJson(file, value) {
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
