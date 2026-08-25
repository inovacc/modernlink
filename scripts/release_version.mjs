import {readFileSync, writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';

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
const embeddedBundlePath = 'cli/crates/aihost/src/assets/bundle.rs';
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

function updateEmbeddedBundle(version, write) {
    const absolute = resolve(root, embeddedBundlePath);
    const before = readFileSync(absolute, 'utf8');
    const frontmatter = /(skills\/(?:modernlink-modernize|modernlink-plan|modernlink-verify|modernlink-domains|modernlink-assess|modernlink-prepare)\/SKILL\.md[\s\S]*?^  version:\s*)[^\s]+/gm;
    const manifests = /^  "version": "[^"]+",$/gm;
    const frontmatterMatches = [...before.matchAll(frontmatter)];
    const manifestMatches = [...before.matchAll(manifests)];
    if (frontmatterMatches.length !== 6 || manifestMatches.length !== 2) {
        throw new Error(`${embeddedBundlePath} has an unexpected version-field count.`);
    }
    if (write) {
        const after = before
            .replace(frontmatter, `$1${version}`)
            .replace(manifests, `  "version": "${version}",`);
        writeFileSync(absolute, after);
    } else if (frontmatterMatches.some((match) => match[1] === undefined || match[0].split('version: ')[1] !== version)
        || manifestMatches.some((match) => !match[0].includes(`"version": "${version}",`))) {
        throw new Error(`${embeddedBundlePath} does not match LATEST (${version}).`);
    }
}

function writeVersion(version) {
    assertVersion(version);
    for (const path of cargoFiles) {
        replaceSingle(path, /^version\s*=\s*"[^"]+"\s*$/m, `version = "${version}"`);
    }
    updateEmbeddedBundle(version, true);
    const packagePath = resolve(root, 'npm/modernlink/package.json');
    const pkg = JSON.parse(readFileSync(packagePath, 'utf8'));
    const packageNeedsUpdate = pkg.version !== version
        || Object.values(pkg.optionalDependencies).some((dependencyVersion) => dependencyVersion !== version);
    pkg.version = version;
    for (const name of Object.keys(pkg.optionalDependencies)) pkg.optionalDependencies[name] = version;
    if (packageNeedsUpdate) writeFileSync(packagePath, `${JSON.stringify(pkg, null, 2)}\n`);
    writeFileSync(latestPath, `${version}\n`);
}

function checkVersion(version) {
    for (const path of cargoFiles) checkSingle(path, /^version\s*=\s*"([^"]+)"\s*$/gm, version);
    updateEmbeddedBundle(version, false);
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
