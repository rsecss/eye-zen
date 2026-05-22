# Stage 1 Notes — Health Analysis Backend (2026-05-23)

## Verification

- `cargo fmt --all --manifest-path src-tauri/Cargo.toml` — clean.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` — clean.
- `cargo test --manifest-path src-tauri/Cargo.toml` — 175 passed / 0 failed.
- `npx svelte-check --tsconfig ./tsconfig.json` — 0 errors, 0 warnings, 0 files with problems.

## Schema migration smoke

`fresh_init_reports_schema_version_two` asserts `PRAGMA user_version == 2` plus
`rest_cycle_events` existing in the freshly initialised in-memory DB. The
companion `v1_to_v2_migration_backfills_taken_cycles` manually seeds a
schema-v1 database with 3 legacy `state='resting'` rows, runs `migrate()`, and
asserts:

1. `PRAGMA user_version` becomes 2.
2. Backfill produces 3 `outcome='taken'` rows in `rest_cycle_events`.
3. Re-running `migrate()` is a no-op (rows stay at 3, not 6).

## Architectural notes

### Where each cycle event is emitted

- **Taken** (`Resting -> Working` timeout) and **Skipped** (`Alerting ->
  Working` user-skip) — emitted by `cycle_event_from_transition` inside
  `apply_transition_and_collect_effects` in `timer/machine.rs`. Snapshots
  pre-transition `inner.is_long_break` / `inner.mode`.
- **Suppressed** (`Working -> Working` skip-flag reset) — emitted by the
  timer loop in `services/context.rs::spawn_timer_loop`. The pure machine
  doesn't see WHICH flag fired (only `any_active()`), so the priority order
  (fullscreen > schedule > afk > process_whitelisted, per PRD §2) lives in
  `suppression_event()` outside the lock.

Justification for this split: keeps `machine.rs` free of OS / detector
dependencies (`PlatformApi::get_foreground_process_name`), matches how
`RecordRestSession` is already layered, and is unit-testable via the new
`working_to_working_skip_does_not_record_cycle_event_from_machine` test.

### Dual-write retention

PRD §3 Q7 demands that `activity_segments` keeps receiving rows for legacy
export tooling. Implementation: `Effect::RecordRestSession` (existing) still
writes to `activity_segments` for taken rests; `Effect::RecordCycleEvent`
(new) writes to `rest_cycle_events`. Both effects are emitted for every
taken rest, so no logic in `record_cycle_event` knows about
`activity_segments`. Tests use a helper `record_taken(service, draft)` that
calls both write paths in sequence to mirror production.

### Source of truth for trends

`statistics_trends` was rewritten to read `SELECT occurred_at, duration_secs
FROM rest_cycle_events WHERE outcome = 'taken'`. Backfill from the v1→v2
migration preserves historical bucket continuity, so the chart on day-of-
upgrade looks the same before and after migration.

### Whitelist match capture

`DetectorService::foreground_whitelist_match(&whitelist) -> Option<String>`
is the new public-on-crate API. Returns the matched basename when a
`process_whitelisted` suppression fires. Empty list / no foreground process
/ no match → `None`.

## Open items for Stage 2 (frontend)

1. Statistics page should call `statisticsCycleOutcomes()` alongside the
   existing `getStatisticsTrends()` call.
2. New i18n keys (per PRD §7) span:
   - ECI hero: title, beta-badge, tooltip (adherence / longest / deferred
     `skip_rate`), threshold labels (good/okay/attention), warming_up,
     rest_day.
   - Today tiles: taken / skipped / suppressed + 4 reason labels.
   - Ribbon: title, legend, empty state.
   - Rhythm: current/best/threshold.
3. ECharts trend chart is unaffected — its `StatisticsTrendPayload` contract
   didn't change.
4. New bindings to consume:
   `CycleOutcome`, `CycleReason`, `CycleOutcomesPayload`, `RibbonEntry`,
   `ReasonBreakdown`, `EyeCareIndex`, `EyeCareComponents`, `RhythmPayload`.

## Risks acknowledged

- `longest_work_secs_today` uses gap-between-taken-rests as a proxy for
  longest unbroken work segment. Skipped / suppressed cycles collapse into
  the surrounding span, which is the PRD-stated v0.6 approximation.
- ECI threshold (80 / 60) and weights (0.7 / 0.3) are judgment-anchored;
  PRD §"Out-of-scope risk acknowledgement" says we iterate in v0.7.
- `process_hint` is recorded only when reason is `process_whitelisted` —
  no other foreground process names leave the boundary.
