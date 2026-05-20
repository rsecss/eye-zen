# feat: SQLite rest statistics trends

## Goal

Persist rest behavior to SQLite and expose daily, weekly, and monthly trend data on the Statistics page with ECharts, so rest history survives app restarts and can be verified through backend aggregation tests and frontend IPC-render tests.

## What I Already Know

* The requested feature spans Rust services, Tauri IPC, generated/typed frontend command wrappers, Svelte Statistics UI, and tests.
* Current docs list `StatService` as planned P2 storage at `app_data_dir/eyezen/data.db`.
* Current `TimerService` transitions include `Alerting -> Resting` and `Resting -> Working`, which are the natural anchors for persisted rest sessions.
* Existing frontend command wrappers live in `src/lib/commands.ts` and use Tauri `invoke` with a timeout.
* `npm run ci` runs version sync, Rust fmt, clippy, Rust tests, svelte-check, Vitest, format check, and frontend build.

## Assumptions

* A rest record represents one completed rest session from entering `resting` until returning to `working`.
* The five-cycle manual acceptance path means five `working -> resting -> working` cycles should produce five persisted sessions and matching trend totals after page reload or app restart.
* Aggregation should bucket by local calendar day/week/month for the timezone requested by the frontend or, if none is supplied, the runtime local timezone.
* Backward compatibility means a user with no existing SQLite database should start successfully and get schema creation automatically.

## Requirements

* Add SQLite-backed persistence for completed rest sessions.
* Auto-create the database file and required tables/indexes on startup when no previous DB exists.
* Record enough timestamps/duration metadata to aggregate accurately across timezone, month-end, and daylight-saving boundaries.
* Expose a typed Tauri IPC command for Statistics trend data.
* Add or complete a Statistics page reachable from the main UI that renders day, week, and month trends with ECharts.
* Keep frontend IPC access behind `src/lib/commands.ts` rather than invoking commands ad hoc in components.
* Preserve existing timer behavior while adding persistence as a side effect of valid rest completion.
* Add Rust tests covering aggregation boundary cases: cross-timezone, month-end, and DST.
* Add Vitest coverage that mocks IPC and verifies Statistics rendering behavior.

## Acceptance Criteria

* [x] After five `working -> resting` cycles and corresponding rest completions, the Statistics page shows correct totals/trends.
* [x] Closing and restarting the app preserves previously recorded rest statistics.
* [x] Starting from an old-version profile with no database file creates the SQLite schema automatically without startup failure.
* [x] Daily, weekly, and monthly aggregation is correct across timezone changes, month boundaries, and daylight-saving transitions.
* [x] `npm run ci` passes.
* [x] Rust tests cover aggregation boundaries.
* [x] Vitest tests mock Tauri IPC and verify Statistics page rendering.

## Definition of Done

* Code changes are scoped to the requested storage, IPC, Statistics UI, and tests.
* New Rust and frontend types remain consistent across IPC boundaries.
* No secrets or user data are exposed in logs or test output.
* Trellis quality and finish steps are followed before marking this goal complete.

## Technical Approach

Implement a `StatService` in Rust that owns SQLite initialization and rest-session persistence, register it in `AppServices`, and call it from the timer/effect path when a rest session completes. Store timestamps in UTC with duration seconds, then aggregate by explicit timezone-aware local buckets for day, ISO-like week, and month trend ranges. Add a Tauri command that returns a compact DTO for Statistics charts, wrap it in `src/lib/commands.ts`, and render it in a Svelte Statistics page using ECharts.

## Decision (ADR-lite)

**Context**: Statistics data must survive restarts, support old profiles without a DB, and remain testable for calendar edge cases.

**Decision**: Use a small backend-owned SQLite service with startup schema creation, UTC persisted timestamps, and deterministic aggregation functions that accept explicit timezone inputs in tests.

**Consequences**: The timer remains the source of behavior truth, while statistics become a persistence side effect. Aggregation logic stays in Rust where date/time boundary tests are strongest; frontend tests focus on IPC and rendering state.

## Out of Scope

* Cloud sync, export/import, or user-editable history.
* Analytics beyond daily, weekly, and monthly rest trends.
* Changing existing timer duration semantics except where needed to record completed rest sessions.

## Technical Notes

* Relevant specs discovered: `.trellis/spec/architecture/index.md`, `.trellis/spec/backend/index.md`, `.trellis/spec/frontend/index.md`, `.trellis/spec/guides/index.md`.
* Storage spec says statistics data should live in SQLite at `app_data_dir/eyezen/data.db`.
* Existing module anchors: `src-tauri/src/services/timer/`, `src-tauri/src/services/mod.rs`, `src-tauri/src/commands/mod.rs`, `src/lib/commands.ts`, `src/pages/main/`.
* `package.json` currently does not list `echarts`; `src-tauri/Cargo.toml` currently does not list SQLite/sqlx dependencies.
