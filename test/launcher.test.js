"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { expectedHash, releaseUrl, targetFor } = require("../bin/ascii-apex.js");

test("maps supported platforms to release assets", () => {
  assert.equal(targetFor("win32", "x64"), "ascii-apex-windows-x64.exe");
  assert.equal(targetFor("linux", "x64"), "ascii-apex-linux-x64");
  assert.equal(targetFor("darwin", "x64"), "ascii-apex-darwin-x64");
  assert.equal(targetFor("darwin", "arm64"), "ascii-apex-darwin-arm64");
});

test("rejects unsupported platforms clearly", () => {
  assert.throws(() => targetFor("linux", "arm64"), /Unsupported platform/u);
});

test("builds a versioned GitHub release URL", () => {
  assert.equal(
    releaseUrl("ascii-apex-linux-x64"),
    "https://github.com/meetpandya4715/tui-car-racing-rust/releases/download/v0.1.0/ascii-apex-linux-x64",
  );
});

test("reads an asset hash from SHA256SUMS", () => {
  const hash = "a".repeat(64);
  assert.equal(expectedHash(`${hash}  ascii-apex-linux-x64\n`, "ascii-apex-linux-x64"), hash);
});

test("fails when an asset is absent from SHA256SUMS", () => {
  assert.throws(() => expectedHash("", "missing"), /does not contain/u);
});
