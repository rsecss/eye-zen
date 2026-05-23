import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, '..');

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error('Usage: node scripts/bump-version.mjs <semver>');
  console.error('Example: node scripts/bump-version.mjs 0.2.0');
  process.exit(1);
}

function patchJson(relPath, mutate) {
  const abs = resolve(repoRoot, relPath);
  const raw = readFileSync(abs, 'utf8');
  const data = JSON.parse(raw);
  mutate(data);
  const trailing = raw.endsWith('\n') ? '\n' : '';
  writeFileSync(abs, `${JSON.stringify(data, null, 2)}${trailing}`);
  console.log(`  updated ${relPath}`);
}

function patchCargoToml(relPath) {
  const abs = resolve(repoRoot, relPath);
  const lines = readFileSync(abs, 'utf8').split('\n');
  let inPackageTable = false;
  let patched = false;
  for (let i = 0; i < lines.length; i += 1) {
    const trimmed = lines[i].trim();
    if (trimmed.startsWith('[')) {
      inPackageTable = trimmed === '[package]';
      continue;
    }
    if (!inPackageTable) continue;
    if (/^version\s*=\s*"[^"]+"/.test(trimmed)) {
      lines[i] = `version = "${version}"`;
      patched = true;
      break;
    }
  }
  if (!patched) {
    throw new Error(`version not found in [package] section of ${relPath}`);
  }
  writeFileSync(abs, lines.join('\n'));
  console.log(`  updated ${relPath}`);
}

function patchCargoLock(relPath, packageName) {
  const abs = resolve(repoRoot, relPath);
  const lines = readFileSync(abs, 'utf8').split('\n');
  let inTargetPackage = false;
  let patched = false;
  for (let i = 0; i < lines.length; i += 1) {
    const trimmed = lines[i].trim();
    if (trimmed === '[[package]]') {
      inTargetPackage = false;
      continue;
    }
    if (trimmed === `name = "${packageName}"`) {
      inTargetPackage = true;
      continue;
    }
    if (inTargetPackage && /^version\s*=\s*"[^"]+"/.test(trimmed)) {
      lines[i] = `version = "${version}"`;
      patched = true;
      break;
    }
  }
  if (!patched) {
    throw new Error(`${packageName} package version not found in ${relPath}`);
  }
  writeFileSync(abs, lines.join('\n'));
  console.log(`  updated ${relPath}`);
}

function prependChangelogSection(relPath) {
  const abs = resolve(repoRoot, relPath);
  const text = readFileSync(abs, 'utf8');
  if (text.includes(`## [${version}]`)) {
    console.log(`  CHANGELOG already has [${version}] — skipped`);
    return;
  }
  const today = new Date().toISOString().slice(0, 10);
  const stub = `## [${version}] - ${today}\n\n### Added\n\n- TODO\n\n### Changed\n\n- TODO\n\n### Fixed\n\n- TODO\n\n`;

  // Anchor on the first existing version heading so the stub lands after the
  // header's description block (categories table + line-format guidance).
  const firstHeading = text.match(/^## \[\d/m);
  if (firstHeading) {
    const insertAt = firstHeading.index;
    writeFileSync(abs, `${text.slice(0, insertAt)}${stub}${text.slice(insertAt)}`);
    console.log(`  prepended ## [${version}] section to ${relPath}`);
    return;
  }

  // Greenfield CHANGELOG: fall back to inserting after the header marker.
  const marker = '\nand this project adheres to [Semantic Versioning](https://semver.org/).\n';
  const idx = text.indexOf(marker);
  if (idx === -1) {
    throw new Error('CHANGELOG header not found; insert section manually');
  }
  const insertAt = idx + marker.length;
  writeFileSync(abs, `${text.slice(0, insertAt)}\n${stub}${text.slice(insertAt)}`);
  console.log(`  prepended ## [${version}] section to ${relPath}`);
}

console.log(`Bumping version to ${version}...`);
patchJson('package.json', (data) => {
  data.version = version;
});
patchCargoToml('src-tauri/Cargo.toml');
patchCargoLock('src-tauri/Cargo.lock', 'eyezen');
patchJson('src-tauri/tauri.conf.json', (data) => {
  data.version = version;
});
prependChangelogSection('CHANGELOG.md');

console.log('\nRemaining manual updates:');
console.log('  - README.md / README.zh-CN.md badge if it shows the version explicitly');
console.log('  - CHANGELOG.md: fill in the TODO bullets');
console.log('\nVerify with: node scripts/check-version-sync.mjs');
