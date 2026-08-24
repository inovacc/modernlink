import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const latestPath = resolve(root, 'LATEST');
const cargoFiles = [
  'cli/Cargo.toml',
  'crates/core/Cargo.toml',
  'crates/http/Cargo.toml',
  'crates/jni/Cargo.toml',
  'crates/messaging/Cargo.toml',
  'crates/tls/Cargo.toml',
  'hacks/messaging-demo/Cargo.toml'
];
const skillFiles = [
  'cli/plugin/skills/modernlink-modernize/SKILL.md',
  'cli/plugin/skills/modernlink-plan/SKILL.md',
  'cli/plugin/skills/modernlink-verify/SKILL.md'
];
const versionPattern = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;

function readLatest() {
  const text = readFileSync(latestPath, 'utf8');
  if (!text.endsWith('\n') || text.trim() !== text.slice(0, -1) || text.trim().includes('\n')) {
    throw new Error('LATEST must contain exactly one newline-terminated semantic version.');
  }
  const version = text.trim();
  if (!versionPattern.test(version)) throw new Error(`LATEST is not a valid semantic version: ${version}`);
  return version;
}

function assertVersion(version) {
  if (!versionPattern.test(version)) throw new Error(`not a valid semantic version: ${version}`);
}

function replaceSingle(path, pattern, replacement) {
  const absolute = resolve(root, path);
  const before = readFileSync(absolute, 'utf8');
  const matches = before.match(pattern);
  if (!matches || matches.length !== 1) throw new Error(`${path} must contain exactly one release version field.`);
  const after = before.replace(pattern, replacement);
  writeFileSync(absolute, after);
}

function checkSingle(path, pattern, version) {
  const text = readFileSync(resolve(root, path), 'utf8');
  const matches = [...text.matchAll(pattern)];
  if (matches.length !== 1 || matches[0][1] !== version) {
    throw new Error(`${path} does not match LATEST (${version}).`);
  }
}

function writeVersion(version) {
  assertVersion(version);
  for (const path of cargoFiles) {
    replaceSingle(path, /^version\s*=\s*"[^"]+"\s*$/m, `version = "${version}"`);
  }
  for (const path of skillFiles) {
    replaceSingle(path, /^  version:\s*[^\s]+\s*$/m, `  version: ${version}`);
  }
  const packagePath = resolve(root, 'npm/modernlink/package.json');
  const pkg = JSON.parse(readFileSync(packagePath, 'utf8'));
  pkg.version = version;
  for (const name of Object.keys(pkg.optionalDependencies)) pkg.optionalDependencies[name] = version;
  writeFileSync(packagePath, `${JSON.stringify(pkg, null, 2)}\n`);
  writeFileSync(latestPath, `${version}\n`);
}

function checkVersion(version) {
  for (const path of cargoFiles) checkSingle(path, /^version\s*=\s*"([^"]+)"\s*$/gm, version);
  for (const path of skillFiles) checkSingle(path, /^  version:\s*([^\s]+)\s*$/gm, version);
  const pkg = JSON.parse(readFileSync(resolve(root, 'npm/modernlink/package.json'), 'utf8'));
  if (pkg.version !== version) throw new Error(`npm/modernlink/package.json does not match LATEST (${version}).`);
  for (const [name, dependencyVersion] of Object.entries(pkg.optionalDependencies)) {
    if (dependencyVersion !== version) throw new Error(`${name} does not match LATEST (${version}).`);
  }
}

const [command, argument] = process.argv.slice(2);
if (command === 'print' && argument === undefined) {
  console.log(`version=${readLatest()}`);
} else if (command === 'check' && argument === undefined) {
  const version = readLatest();
  checkVersion(version);
  console.log(`all release artifacts match ${version}`);
} else if (command === 'set' && argument !== undefined) {
  writeVersion(argument);
  checkVersion(argument);
  console.log(`set all release artifacts to ${argument}`);
} else {
  throw new Error('usage: node scripts/release_version.mjs <print|check|set VERSION>');
}
