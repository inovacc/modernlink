#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');
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

const executable = path.join(path.dirname(packageJson), 'bin', platform === 'win32' ? 'modernlink.exe' : 'modernlink');
if (!fs.existsSync(executable)) {
  fail(`${packageName}@${wrapper.version} does not contain its expected Rust binary.`);
}

const result = spawnSync(executable, process.argv.slice(2), { stdio: 'inherit' });
if (result.error) fail(`cannot launch ModernLink Rust binary: ${result.error.message}`);
process.exit(result.status === null ? 1 : result.status);

function fail(message) {
  process.stderr.write(`modernlink: ${message}\n`);
  process.exit(1);
}
