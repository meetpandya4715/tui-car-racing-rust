#!/usr/bin/env node

"use strict";

const crypto = require("node:crypto");
const fs = require("node:fs");
const https = require("node:https");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const packageJson = require("../package.json");

const repository = "meetpandya4715/tui-car-racing-rust";

const targets = new Map([
  ["win32-x64", "ascii-apex-windows-x64.exe"],
  ["linux-x64", "ascii-apex-linux-x64"],
  ["darwin-x64", "ascii-apex-darwin-x64"],
  ["darwin-arm64", "ascii-apex-darwin-arm64"],
]);

function targetFor(platform, arch) {
  const asset = targets.get(`${platform}-${arch}`);
  if (!asset) {
    throw new Error(
      `Unsupported platform: ${platform}-${arch}. ` +
        "Supported targets are Windows x64, Linux x64, and macOS x64/arm64.",
    );
  }
  return asset;
}

function releaseUrl(fileName) {
  return `https://github.com/${repository}/releases/download/v${packageJson.version}/${fileName}`;
}

function download(url, redirectsLeft = 5) {
  return new Promise((resolve, reject) => {
    const request = https.get(
      url,
      { headers: { "User-Agent": `ascii-apex/${packageJson.version}` } },
      (response) => {
        const status = response.statusCode ?? 0;
        const location = response.headers.location;

        if (status >= 300 && status < 400 && location) {
          response.resume();
          if (redirectsLeft === 0) {
            reject(new Error(`Too many redirects while downloading ${url}`));
            return;
          }
          resolve(download(new URL(location, url).toString(), redirectsLeft - 1));
          return;
        }

        if (status !== 200) {
          response.resume();
          reject(new Error(`Download failed with HTTP ${status}: ${url}`));
          return;
        }

        const chunks = [];
        response.on("data", (chunk) => chunks.push(chunk));
        response.on("end", () => resolve(Buffer.concat(chunks)));
        response.on("error", reject);
      },
    );
    request.on("error", reject);
  });
}

function expectedHash(checksums, asset) {
  for (const line of checksums.split(/\r?\n/u)) {
    const match = line.match(/^([a-fA-F0-9]{64})\s+\*?(.+)$/u);
    if (match && match[2] === asset) {
      return match[1].toLowerCase();
    }
  }
  throw new Error(`SHA256SUMS does not contain ${asset}`);
}

async function installBinary(asset, binaryPath) {
  process.stderr.write(`Downloading ASCII Apex ${packageJson.version} for ${process.platform}-${process.arch}...\n`);
  const [checksumsBuffer, binary] = await Promise.all([
    download(releaseUrl("SHA256SUMS")),
    download(releaseUrl(asset)),
  ]);

  const wanted = expectedHash(checksumsBuffer.toString("utf8"), asset);
  const received = crypto.createHash("sha256").update(binary).digest("hex");
  if (wanted !== received) {
    throw new Error(`Checksum verification failed for ${asset}`);
  }

  fs.mkdirSync(path.dirname(binaryPath), { recursive: true });
  const temporaryPath = `${binaryPath}.${process.pid}.tmp`;
  fs.writeFileSync(temporaryPath, binary, { mode: 0o755 });
  fs.renameSync(temporaryPath, binaryPath);
  if (process.platform !== "win32") {
    fs.chmodSync(binaryPath, 0o755);
  }
}

async function main() {
  const asset = targetFor(process.platform, process.arch);
  const cacheRoot =
    process.env.ASCII_APEX_CACHE_DIR ||
    path.join(os.homedir(), ".ascii-apex", packageJson.version);
  const binaryPath = path.join(cacheRoot, asset);

  if (!fs.existsSync(binaryPath)) {
    await installBinary(asset, binaryPath);
  }

  const result = spawnSync(binaryPath, process.argv.slice(2), { stdio: "inherit" });
  if (result.error) {
    throw result.error;
  }
  if (result.signal) {
    throw new Error(`ASCII Apex exited after receiving ${result.signal}`);
  }
  return result.status ?? 1;
}

if (require.main === module) {
  main()
    .then((status) => process.exit(status))
    .catch((error) => {
      process.stderr.write(`ascii-apex: ${error.message}\n`);
      process.exit(1);
    });
}

module.exports = { expectedHash, releaseUrl, targetFor };
