# Coverage Baseline – v0.4.0 Pre-release

Captured 2026-05-22 on `feat/process-whitelist` after process whitelist implementation. Used as the reference point for future quality gates.

## Frontend (vitest --coverage, v8 provider)

```
File                          | % Stmts | % Branch | % Funcs | % Lines
------------------------------|---------|----------|---------|---------
All files                     |   59.51 |    87.72 |   46.26 |   59.51
 lib/i18n                     |  100.00 |   100.00 |  100.00 |  100.00
 lib/stores/config.svelte.ts  |   60.31 |   100.00 |   40.00 |   60.31
 lib/stores/timer.svelte.ts   |   91.42 |    83.33 |  100.00 |   91.42
 lib/commands.ts              |    0.00 |     0.00 |    0.00 |    0.00
 lib/events.ts                |    0.00 |     0.00 |    0.00 |    0.00
 pages/main/SettingsPage      |   69.08 |    92.18 |   21.87 |   69.08
 pages/main/StatisticsPage    |   98.59 |    88.63 |   87.50 |   98.59
 pages/main/components/*      |  100.00 |    81.81 |  100.00 |  100.00
 pages/main/AboutPage.svelte  |    0.00 |     0.00 |    0.00 |    0.00
 pages/main/MainApp.svelte    |    0.00 |     0.00 |    0.00 |    0.00
 pages/tip/TipApp.svelte      |    0.00 |     0.00 |    0.00 |    0.00
 pages/tip-minimal/*          |    0.00 |     0.00 |    0.00 |    0.00
 pages/tray/TrayApp.svelte    |    0.00 |     0.00 |    0.00 |    0.00
```

### Interpretation

- **High-coverage core (≥90%)**: i18n, timer store, StatisticsPage, all reusable components — these are pure-logic units, fully unit-testable.
- **Mid-coverage (60–70%)**: SettingsPage, configStore — UI components with logic; tests cover the most-impactful disabled-state and capability-routing paths.
- **Zero-coverage by design**: `commands.ts` and `events.ts` are thin Tauri IPC wrappers (mocking them in tests means they ARE the mock target; running them against a real Tauri runtime needs e2e); window-level entrypoints (MainApp/TipApp/TrayApp/TipMinimalApp/AboutPage) are integration surfaces that need a real Tauri webview to render meaningfully. User explicitly opted out of e2e for v0.4.0.

## Backend (Rust)

`cargo llvm-cov` not yet integrated (requires `cargo install cargo-llvm-cov` on each runner). All 131 backend tests pass via standard `cargo test`. Coverage spread by layer (qualitative):

- `models/config.rs::sanitize_process_whitelist`: 3 dedicated tests cover the 4 normalization rules + truncation
- `services/detector.rs::is_foreground_in_whitelist`: 4 dedicated tests cover empty / hit / miss / None-platform
- `services/timer/state.rs::SkipFlags::any_active`: existing tests cover OR aggregation for all 4 flags
- `platform/{windows,macos,linux}.rs::get_foreground_process_name`: NOT unit-tested (requires real OS APIs and a windowed environment; covered manually via cargo build per platform and the AC verification checklist in PRD)

## Quality gate status for v0.4.0

- No hard coverage threshold enforced this PR (would block on legitimate UI integration code).
- Baseline captured to `research/coverage-baseline.md` for comparison in v0.5.0.
- Future enforcement candidates: lib/i18n ≥ 95%, lib/stores ≥ 70%, components/* ≥ 90% — these are achievable today and stable across refactors.

## How to re-run

```
npm run test:coverage
```

Outputs HTML report to `coverage/` (gitignored).
