#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { spawnSync } = require('node:child_process');

const wrapper = require('../package.json');
const platform = process.platform;
const arch = process.arch;
const suffix = {
  'win32-x64': 'win32-x64',
  'linux-x64': 'linux-x64',
  'linux-arm64': 'linux-arm64',
  'darwin-x64': 'darwin-x64',
  'darwin-arm64': 'darwin-arm64'
}[`${platform}-${arch}`];

if (!suffix) {
  fail(`ModernLink has no native package for ${platform}-${arch}.`);
}

const packageName = `@inovacc/modernlink-${suffix}`;
let packageJson;
try {
  packageJson = require.resolve(`${packageName}/package.json`);
} catch {
  fail(`${packageName}@${wrapper.version} is not installed. Reinstall this exact ModernLink version; offline use requires the platform package in the local npm/Bun cache.`);
}

const platformPackage = JSON.parse(fs.readFileSync(packageJson, 'utf8'));
if (platformPackage.version !== wrapper.version) {
  fail(`${packageName} version ${platformPackage.version} does not match wrapper version ${wrapper.version}.`);
}

const packageRoot = fs.realpathSync(path.dirname(packageJson));
const manifestPath = resolveContained(packageRoot, 'modernlink-native.json');
if (!fs.existsSync(manifestPath)) fail(`${packageName}@${wrapper.version} has no native integrity manifest.`);

let manifest;
try {
  manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
} catch {
  fail(`${packageName}@${wrapper.version} has an unreadable native integrity manifest.`);
}
const expectedBinaryPath = `bin/${platform === 'win32' ? 'modernlink.exe' : 'modernlink'}`;
if (
  manifest.schema_version !== 'modernlink.native-integrity/v1' ||
  manifest.package !== packageName ||
  manifest.version !== wrapper.version ||
  manifest.os !== platform ||
  manifest.arch !== arch ||
  manifest.binary_path !== expectedBinaryPath ||
  !/^sha256:[a-f0-9]{64}$/.test(manifest.binary_sha256 || '')
) {
  fail(`${packageName}@${wrapper.version} native integrity manifest does not match this launcher platform.`);
}

const executable = resolveContained(packageRoot, manifest.binary_path);
const stat = fs.statSync(executable, { throwIfNoEntry: false });
if (!stat || !stat.isFile()) fail(`${packageName}@${wrapper.version} does not contain its expected Rust binary.`);
const actualHash = `sha256:${crypto.createHash('sha256').update(fs.readFileSync(executable)).digest('hex')}`;
if (!crypto.timingSafeEqual(Buffer.from(actualHash), Buffer.from(manifest.binary_sha256))) {
  fail(`${packageName}@${wrapper.version} Rust binary SHA-256 does not match its release manifest.`);
}

const result = spawnSync(executable, process.argv.slice(2), { stdio: 'inherit' });
if (result.error) fail(`cannot launch ModernLink Rust binary: ${result.error.message}`);
process.exit(result.status === null ? 1 : result.status);

function fail(message) {
  process.stderr.write(`modernlink: ${message}\n`);
  process.exit(1);
}

function resolveContained(root, relativePath) {
  if (typeof relativePath !== 'string' || path.isAbsolute(relativePath)) fail('native integrity manifest has an unsafe path.');
  const candidate = path.resolve(root, relativePath);
  const relative = path.relative(root, candidate);
  if (relative === '' || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    fail('native integrity manifest path escapes its platform package.');
  }
  return candidate;
}
