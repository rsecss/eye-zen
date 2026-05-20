import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, '..');

function readPkg() {
  const pkg = JSON.parse(readFileSync(resolve(repoRoot, 'package.json'), 'utf8'));
  return { source: 'package.json', version: pkg.version };
}

function readCargo() {
  const text = readFileSync(resolve(repoRoot, 'src-tauri/Cargo.toml'), 'utf8');
  const lines = text.split('\n');
  let inPackageTable = false;
  for (const raw of lines) {
    const line = raw.trim();
    if (line.startsWith('[')) {
      inPackageTable = line === '[package]';
      continue;
    }
    if (!inPackageTable) continue;
    const match = line.match(/^version\s*=\s*"([^"]+)"/);
    if (match) return { source: 'src-tauri/Cargo.toml', version: match[1] };
  }
  throw new Error('version not found in [package] section of src-tauri/Cargo.toml');
}

function readTauriConf() {
  const conf = JSON.parse(readFileSync(resolve(repoRoot, 'src-tauri/tauri.conf.json'), 'utf8'));
  if (!conf.version) throw new Error('version not found in src-tauri/tauri.conf.json');
  return { source: 'src-tauri/tauri.conf.json', version: conf.version };
}

function ensureChangelogHas(version) {
  const text = readFileSync(resolve(repoRoot, 'CHANGELOG.md'), 'utf8');
  if (!text.includes(`## [${version}]`)) {
    throw new Error(`CHANGELOG.md missing section "## [${version}]"`);
  }
}

const sources = [readPkg(), readCargo(), readTauriConf()];
const versions = new Set(sources.map((s) => s.version));

if (versions.size !== 1) {
  console.error('Version mismatch across sources:');
  for (const { source, version } of sources) {
    console.error(`  - ${source}: ${version}`);
  }
  console.error('\nRun `node scripts/bump-version.mjs <version>` to sync.');
  process.exit(1);
}

const [version] = versions;

try {
  ensureChangelogHas(version);
} catch (err) {
  console.error(err.message);
  console.error(`Add a "## [${version}] - YYYY-MM-DD" section to CHANGELOG.md before releasing.`);
  process.exit(1);
}

console.log(`Version ${version} is in sync across ${sources.length} source(s) and CHANGELOG.md.`);
