import { spawnSync } from 'node:child_process';

const steps = [
  ['Version sync check', 'node', ['scripts/check-version-sync.mjs']],
  [
    'Rust format check',
    'cargo',
    ['fmt', '--all', '--manifest-path', 'src-tauri/Cargo.toml', '--check'],
  ],
  [
    'Rust clippy',
    'cargo',
    ['clippy', '--manifest-path', 'src-tauri/Cargo.toml', '--all-targets', '--', '-D', 'warnings'],
  ],
  [
    'Rust tests',
    'cargo',
    ['test', '--manifest-path', 'src-tauri/Cargo.toml', '--no-default-features'],
  ],
  ['Frontend type check', 'npx', ['svelte-check', '--tsconfig', './tsconfig.json']],
  ['Frontend tests', 'npm', ['test', '--', '--run']],
  ['Frontend format check', 'npm', ['run', 'format:check']],
  ['Build frontend', 'npm', ['run', 'build']],
];

for (const [index, [label, command, args]] of steps.entries()) {
  console.log(`[${index + 1}/${steps.length}] ${label}`);
  const result = spawnSync(command, args, {
    shell: process.platform === 'win32' && (command === 'npm' || command === 'npx'),
    stdio: 'inherit',
  });

  if (result.error) {
    console.error(result.error.message);
    process.exit(1);
  }

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

console.log('All local CI checks passed.');
