# PRD: Health Analysis (v0.6, Hybrid)

- **Task**: `.trellis/tasks/05-23-health-analysis`
- **Date**: 2026-05-23
- **Branch**: `feat/health-analysis` (base: `main`)
- **Selected route**: Hybrid — P2-lite (adherence/suppression) + honest-mode ESRI + P5 streak cards.
- **Research**: `.trellis/tasks/05-23-health-analysis/research/` (5 files + INDEX)

## Background

Eyezen v0.5.0 ships a SQLite stat service that records only completed rest
sessions (`activity_segments` with `state='resting'`). The Statistics page
renders raw counters and a daily/weekly/monthly ECharts trend — no story,
no cause, no goal. Independent research (this task) + codex review (read-only)
converged on the conclusion that Eyezen's real differentiator is the
**`SkipFlags { fullscreen, schedule, afk, process_whitelisted }`** taxonomy
it already computes but discards. Persisting the *why* of every cycle is a
unique unlock no competitor (Stretchly, Workrave, Blink Eye, ProjectEye)
currently ships. v0.6 turns that latent signal into the headline feature.

## Goals

1. **Explain the "why"** — every rest cycle (taken / skipped / suppressed) is
   persisted with a typed reason; user can answer "why didn't I get more
   rests today?".
2. **Honest summary** — a single Beta-labelled Eye-Care Index (0–100) over
   only the components we can defend in v0.6 (adherence + longest-session);
   the `skip_rate` component is deferred to v0.7 once P2-lite data has had
   real-world settling time.
3. **Rhythm awareness** — current streak + best streak cards anchored to the
   user's own median, no fitness-style streak-loss alarm.

## Non-goals

- Foreground process / window-title heatmap (P3) — postponed.
- Symptom check-in modal (P4) — postponed.
- Cloud, telemetry, account, peer comparison — never.
- Replacing the existing trends chart — kept as-is; new content is additive.
- Backfilling pre-v0.6 rows with synthetic outcome data — legacy rows surface
  as "taken" only, no inference.
- Recording arbitrary foreground process names — only the whitelist-hit
  basename is optionally captured (already opted-in via whitelist enrollment).

## Approach

### 1. Data model — schema v2 (additive + one-shot backfill)

New table:

```sql
CREATE TABLE rest_cycle_events (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    occurred_at   TEXT NOT NULL,        -- RFC3339 UTC
    outcome       TEXT NOT NULL,        -- 'taken' | 'skipped' | 'suppressed'
    reason        TEXT,                 -- NULL | 'fullscreen' | 'schedule' | 'afk' | 'process_whitelisted'
    process_hint  TEXT,                 -- NULL unless reason='process_whitelisted' AND user opted in
    duration_secs INTEGER,              -- non-null only when outcome='taken'
    mode          TEXT,                 -- NULL for legacy backfilled rows; 'twenty_twenty_twenty' | 'pomodoro' for v0.6+
    is_long_break INTEGER NOT NULL DEFAULT 0  -- 0/1; only meaningful in pomodoro
);
CREATE INDEX idx_rest_cycle_events_occurred_at ON rest_cycle_events(occurred_at);
CREATE INDEX idx_rest_cycle_events_outcome ON rest_cycle_events(outcome);
PRAGMA user_version = 2;
```

**Migration v1→v2 (one-shot, idempotent)**:

```sql
-- backfill all legacy 'resting' segments as taken cycles with NULL mode
INSERT INTO rest_cycle_events
       (occurred_at, outcome, reason, process_hint, duration_secs, mode, is_long_break)
SELECT  started_at, 'taken', NULL,   NULL,         duration_secs, NULL, 0
FROM   activity_segments
WHERE  state = 'resting';
```

After v0.6 cutover, **all reads (trends + cycle_outcomes) go to
`rest_cycle_events`**. `activity_segments` continues to receive a row per
taken rest (dual-write) purely for VACUUM-INTO export backward-compat —
users with existing tooling reading `activity_segments` won't break.
Removing `activity_segments` writes is a v0.7 decision after we confirm no
downstream tooling depends on it.

### 2. Event recording

Extend `Effect` enum (`src-tauri/src/services/timer/effect.rs`) with:

```rust
RecordCycleEvent(CycleEventDraft),
```

`CycleEventDraft { occurred_at_utc, outcome, reason, process_hint, duration_secs, mode, is_long_break }`.

Emission points in `timer/machine.rs`:

| Transition | Outcome | Reason source |
|---|---|---|
| `Resting → Working` (natural completion) | `taken` | NULL; `duration_secs` set |
| `Alerting → Working` via `UserEvent::Skip` | `skipped` | NULL |
| `Working → Working` blocked by `SkipFlags.fullscreen_active` at PreAlert tip | `suppressed` | `fullscreen` |
| same blocked by `SkipFlags.schedule_inactive` | `suppressed` | `schedule` |
| same blocked by `SkipFlags.afk_active` | `suppressed` | `afk` |
| same blocked by `SkipFlags.process_whitelisted` | `suppressed` | `process_whitelisted`; `process_hint = $whitelist_basename_hit` if user opted in |

`effect_executor.rs` routes new variant to `StatService::record_cycle_event()`.

### 3. Query path

New `StatService::cycle_outcomes(range, timezone) -> CycleOutcomesPayload`:

```rust
struct CycleOutcomesPayload {
    today_taken: u32,
    today_skipped: u32,
    today_suppressed: u32,
    today_adherence_rate: Option<f32>,   // taken / (taken+skipped), None if denominator=0
    today_reason_breakdown: ReasonBreakdown,  // { fullscreen, schedule, afk, process_whitelisted }
    last_24h_ribbon: Vec<RibbonEntry>,    // { occurred_at, outcome, reason }
    eye_care_index: EyeCareIndex,         // see §5
    rhythm: RhythmPayload,                // see §6
    is_beta: bool,                        // always true in v0.6
}
```

Existing `statistics_trends()` is **rewritten** to read from
`rest_cycle_events WHERE outcome='taken'`. Aggregation (daily/weekly/monthly
buckets) keeps the same shape; legacy rows backfilled at migration time
preserve historical chart continuity. Two reads now share one source of
truth.

### 4. Frontend — Statistics page additive layout

Existing layout (Hero + 3 Metric cards + Range-toggle Chart) is preserved.
New content added **above** the chart:

1. **Eye-Care Index Hero (Beta)** — large 0–100 number, accent gradient ring,
   tooltip explaining components, "Beta · v0.6" badge.
2. **Today's Cycles row** — three small stat tiles: Taken / Skipped /
   Suppressed (with reason-breakdown popover).
3. **Adherence ribbon** — 24h horizontal ribbon, color-coded by outcome
   (green=taken, amber=skipped, slate=suppressed-with-reason); hover shows
   timestamp + reason.
4. **Rhythm cards** (two side-by-side under Index): Current streak / Best
   streak with threshold caption.
5. Existing range-toggle chart stays below (unchanged).

### 5. Eye-Care Index (Beta) — formula

v0.6 deliberately omits the `skip_rate` component (would be hot-air without
P2-lite settling time; deferred to v0.7).

```
adherence_p      = clamp((taken / (taken + skipped)) * 100, 0, 100)
                   IF (taken + skipped) == 0 then index = "warming up"
longest_session_p = clamp(100 - max(0, (longest_work_secs - target_work_secs) / 60), 0, 100)
ECI v0.6          = round(0.7 * adherence_p + 0.3 * longest_session_p)
```

- `target_work_secs` = `timer.work_minutes * 60` (or pomodoro focus_minutes).
- "Beta" badge + tooltip text explicitly says: *"v0.6 ECI is provisional;
  suppression-rate weighting arrives in v0.7."*
- Thresholds: ≥80 good (green), 60–79 okay (amber), <60 needs attention
  (red). Source: judgment + AOA "follow 20-20-20" anchoring; not RCT.
- Schedule-aware: if today's weekday is NOT in `schedule.active_days`,
  the Hero shows "Rest day" instead of a score (avoids weekend penalty).

### 6. Streak / Best-streak cards

- **Threshold N** = median of last 30 daily `taken` counts (Rust-side
  computation). First-week fallback: `floor(expected_rests_per_day * 0.6)`.
- **Current streak** = consecutive days (ending today) where daily taken ≥ N.
- **Best streak** = longest such run in the available history.
- Caption: *"Based on your 30-day rhythm of {N} rests/day"*. No streak-loss
  alarm; no notifications.

### 7. i18n

All copy lives in `src/lib/i18n/dict-{en,zh-CN}.ts`. New keys (TBD ~25):

- ECI: `index.hero.title`, `index.hero.beta`, `index.hero.tooltip.title`,
  `index.hero.tooltip.adherence`, `index.hero.tooltip.longest`,
  `index.hero.tooltip.deferred`, `index.hero.threshold.good/okay/attention`,
  `index.hero.warming_up`, `index.hero.rest_day`
- Today: `today.taken`, `today.skipped`, `today.suppressed`,
  `today.reason.fullscreen`, `today.reason.schedule`, `today.reason.afk`,
  `today.reason.process_whitelist`
- Ribbon: `ribbon.title`, `ribbon.legend.*`, `ribbon.empty`
- Rhythm: `rhythm.current.title`, `rhythm.best.title`, `rhythm.threshold`

Copy constraint: **no medical claims**. Allowed: "break adherence", "rest
rhythm", "follow 20-20-20", "Eye-Care Index". Forbidden: "reduce eye strain",
"risk", "diagnosis", "fatigue level".

### 8. Privacy & data constraints

- v0.6 records `process_hint` ONLY when `reason='process_whitelisted'` AND
  user has whitelisted that process (opt-in by construction). Other reasons
  never carry a process name.
- DB export (existing `VACUUM INTO`) includes `rest_cycle_events` as-is; if
  the user has `process_hint` recorded, they exported it themselves — no
  opt-out needed (matches existing whitelist export behavior).
- No telemetry, no cloud, no remote calls.

## Verification

- `cargo test --manifest-path src-tauri/Cargo.toml`: new tests for
  `record_cycle_event()`, schema v1→v2 migration, `cycle_outcomes()`
  aggregation, ECI formula edge cases (warming-up, rest-day, denominator=0).
- `npm run ci` 8/8 green locally.
- Manual smoke: trigger each suppress reason (fullscreen YouTube, weekend
  toggle, AFK >threshold, whitelisted app) and confirm a row lands in
  `rest_cycle_events` with the right reason.
- Manual UI: Statistics page renders new sections without layout regression
  on 880×560 main window; ECharts trend chart unaffected.
- Migration smoke: open app once on schema v1 DB, verify `PRAGMA
  user_version` becomes 2 and table exists.

## Out-of-scope risk acknowledgement

- ECI threshold (80/60) is judgment-based, not validated. We can iterate in
  v0.7 once we have a week of real ECI distribution data from dogfood.
- Streak threshold based on 30-day median means new users see "warming up"
  on the streak card for first week — acceptable.
- Pomodoro long-break vs short-break is recorded (`is_long_break`) but not
  exposed in v0.6 UI; v0.7 will surface it if user demand emerges.

## Deliverables

- `src-tauri/src/services/stat.rs` — schema v2 migration, new
  `record_cycle_event()`, `cycle_outcomes()` aggregation + ECI compute +
  streak compute.
- `src-tauri/src/services/timer/effect.rs` + `effect_executor.rs` — new
  `RecordCycleEvent` variant + wiring.
- `src-tauri/src/services/timer/machine.rs` — emission at the 5+1 transitions
  in §2.
- `src-tauri/src/models/statistics.rs` — new ts-rs types
  (`CycleEventDraft`, `CycleOutcomesPayload`, `EyeCareIndex`, `RibbonEntry`,
  `RhythmPayload`).
- `src-tauri/src/commands/statistics.rs` — new `statistics_cycle_outcomes`
  command.
- `src/lib/bindings/` — auto-regenerated.
- `src/pages/main/StatisticsPage.svelte` — new ECI hero, Today tiles,
  ribbon, rhythm cards.
- `src/lib/i18n/dict-en.ts`, `dict-zh-CN.ts` — ~25 new keys.
- `CHANGELOG.md` — v0.6 unreleased section.
- `docs/devlog.md` — entry summarising the decision.

## Success criteria

- [ ] Schema v2 migration runs cleanly on a v0.5.0 DB (manual + test).
- [ ] All 4 suppression reasons land in `rest_cycle_events` during smoke test.
- [ ] ECI Beta renders with "Beta" badge + tooltip explaining components.
- [ ] Streak cards reflect 30-day median threshold correctly.
- [ ] Statistics page passes svelte-check + lint + tests with no regression.
- [ ] No medical-claim copy survives review.

## Open questions — resolved

| # | Decision |
|---|---|
| Q1 ECI formula weights | **0.7 adherence + 0.3 longest-session** |
| Q2 "warming up" trigger | First cycle outcome of the day (default) |
| Q3 Ribbon time window | 24h rolling (default) |
| Q4 Rest-day display | **Hide ECI hero entirely on schedule-inactive days** |
| Q5 Pomodoro long-break visibility | Silent in v0.6 (recorded but not shown) |
| Q6 `process_hint` default | **Enabled by default** (rides on existing whitelist opt-in) |
| Q7 Trends-chart cutover | **v0.6 one-shot migration** — backfill at v1→v2 + dual-write to `activity_segments` for export back-compat |
