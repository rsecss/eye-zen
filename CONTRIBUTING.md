# Contributing

Thanks for helping Eyezen.

## Rules

- Code, commits, and PR titles use English.
- Issues and discussions may use English or Chinese.
- Work on short-lived `feat/<scope>`, `fix/<scope>`, or `chore/<scope>` branches off `main`.
- `main` receives changes only through PR.
- Keep each PR focused.
- Contributions are licensed as GPL-3.0-or-later, inbound = outbound.
- Follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Setup

```bash
git clone https://github.com/rsecss/eye-zen.git
cd eye-zen
npm install
npm run tauri dev
```

Linux needs Tauri system dependencies:

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libasound2-dev
```

## Quality Gate

Run before pushing:

```bash
npm run ci
```

This covers Rust format, clippy, Rust tests, Svelte type check, Vitest, Prettier, Rust check, and frontend build.

## Commits

Use Conventional Commits:

```text
feat(timer): add timed pause
fix(config): preserve invalid config backup
docs: update release workflow
ci: align local and cloud checks
```

Constraints:

- One logical change per commit.
- Subject is imperative English, lowercase, no period.
- Scope is optional but useful: `timer`, `config`, `ui`, `tray`, `platform`, `i18n`.

## PR Checklist

- [ ] Target branch is `main`.
- [ ] Branch name follows `<type>/<scope>` (e.g. `feat/timer-pause`, `fix/config-backup`).
- [ ] `npm run ci` passes.
- [ ] New behavior has tests.
- [ ] New Tauri command has matching capability permissions.
- [ ] IPC or config schema changes are documented and backward-compatible.
- [ ] PR description includes summary, test plan, and risk.

PR descriptions should be concise and in English.

## Contribution Areas

| Area | Path |
|------|------|
| Rust services | `src-tauri/src/services/` |
| Platform code | `src-tauri/src/platform/` |
| Frontend pages | `src/pages/` |
| IPC bindings | `src/lib/commands.ts`, `src-tauri/src/commands/` |
| Docs | `README*.md`, `docs/`, `.trellis/spec/` |

## Project Rules

Canonical engineering rules live in `.trellis/spec/`.

Start here:

- Architecture: `.trellis/spec/architecture/index.md`
- Backend: `.trellis/spec/backend/index.md`
- Frontend: `.trellis/spec/frontend/index.md`
- Quality gate: `.trellis/spec/architecture/testing-quality.md`

## Reports

For bugs, include:

- OS and Eyezen version
- Reproduction steps
- Expected vs actual behavior
- Screenshots or logs if available

Log paths:

- Windows: `%APPDATA%\com.eyezen.app\logs\`
- macOS: `~/Library/Application Support/com.eyezen.app/logs/`
- Linux: `~/.config/com.eyezen.app/logs/`
