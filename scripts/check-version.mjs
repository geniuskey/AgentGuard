import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const expectedVersion = '0.1.0';
const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');

function readJson(relativePath) {
  return JSON.parse(readFileSync(resolve(repoRoot, relativePath), 'utf8'));
}

function workspaceCargoVersion() {
  const cargo = readFileSync(resolve(repoRoot, 'Cargo.toml'), 'utf8');
  const marker = '[workspace.package]';
  const sectionStart = cargo.indexOf(marker);
  if (sectionStart === -1) {
    throw new Error('Could not find [workspace.package] in Cargo.toml');
  }

  const afterMarker = cargo.slice(sectionStart + marker.length);
  const nextSection = afterMarker.search(/^\s*\[/m);
  const workspacePackage = nextSection === -1 ? afterMarker : afterMarker.slice(0, nextSection);
  const version = workspacePackage?.match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];
  if (!version) throw new Error('Could not read workspace.package.version from Cargo.toml');
  return version;
}

const packageLock = readJson('package-lock.json');
const versions = new Map([
  ['package.json', readJson('package.json').version],
  ['package-lock.json', packageLock.version],
  ['package-lock.json root package', packageLock.packages?.['']?.version],
  ['Cargo.toml', workspaceCargoVersion()],
  ['src-tauri/tauri.conf.json', readJson('src-tauri/tauri.conf.json').version]
]);

const mismatches = [...versions].filter(([, version]) => version !== expectedVersion);
if (mismatches.length > 0) {
  const details = [...versions]
    .map(([source, version]) => `${source}: ${version ?? '<missing>'}`)
    .join('\n');
  throw new Error(`Expected every project version to be ${expectedVersion}:\n${details}`);
}

console.log(`All project versions are ${expectedVersion}`);
