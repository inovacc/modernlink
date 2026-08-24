import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { basename, join } from 'node:path';

const [platform, arch, version, source, output] = process.argv.slice(2);
if (![platform, arch, version, source, output].every(Boolean)) {
  throw new Error('usage: package_npm_platform.mjs <platform> <arch> <version> <source-binary> <output-directory>');
}
const packageName = `@inovacc/modernlink-${platform}-${arch}`;
const binaryName = platform === 'win32' ? 'modernlink.exe' : 'modernlink';
const binary = readFileSync(source);
mkdirSync(join(output, 'bin'), { recursive: true });
copyFileSync(source, join(output, 'bin', binaryName));
writeFileSync(join(output, 'package.json'), `${JSON.stringify({
  name: packageName,
  version,
  private: true,
  repository: {
    type: 'git',
    url: 'git+https://github.com/inovacc/modernlink.git'
  },
  publishConfig: {
    registry: 'https://npm.pkg.github.com'
  },
  os: [platform],
  cpu: [arch],
  files: ['bin/', 'modernlink-native.json']
}, null, 2)}\n`);
writeFileSync(join(output, 'modernlink-native.json'), `${JSON.stringify({
  schema_version: 'modernlink.native-integrity/v1',
  package: packageName,
  version,
  os: platform,
  arch,
  binary_path: `bin/${binaryName}`,
  binary_sha256: `sha256:${createHash('sha256').update(binary).digest('hex')}`
}, null, 2)}\n`);
