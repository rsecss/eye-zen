import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, '..');

function extract(version) {
  const changelog = readFileSync(resolve(repoRoot, 'CHANGELOG.md'), 'utf8');
  const lines = changelog.split('\n');
  const startMarker = `## [${version}]`;
  let start = -1;
  for (let i = 0; i < lines.length; i += 1) {
    if (lines[i].startsWith(startMarker)) {
      start = i;
      break;
    }
  }

  if (start === -1) {
    throw new Error(`Version ${version} not found in CHANGELOG.md (looked for "${startMarker}")`);
  }

  let end = lines.length;
  for (let i = start + 1; i < lines.length; i += 1) {
    if (lines[i].startsWith('## [')) {
      end = i;
      break;
    }
  }

  return lines
    .slice(start + 1, end)
    .join('\n')
    .replace(/^\s+|\s+$/g, '');
}

const version = process.argv[2];
if (!version) {
  console.error('Usage: node scripts/extract-changelog.mjs <version>');
  process.exit(1);
}

try {
  process.stdout.write(extract(version));
  process.stdout.write('\n');
} catch (err) {
  console.error(err.message);
  process.exit(1);
}
