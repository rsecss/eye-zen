# Eyezen Current State — What We Already Collect, Aggregate, and Render

- **Date**: 2026-05-23
- **All claims here are `[verified-in-code]` — sourced directly from the repo.**
- **Branch read**: `fix/bump-stub-and-windows-sound-flake` (post v0.5.0, includes Pomodoro + VACUUM INTO export).

This file is the **factual ground truth** the innovation proposals must build on. If a proposal needs data Eyezen doesn't capture, that's called out explicitly.

---

## 1. Persistent storage

### 1.1 SQLite — schema v1

File: `src-tauri/src/services/stat.rs` lines 213–251.

```
table activity_segments (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    state         TEXT NOT NULL,    -- only ever 'resting' today (see §1.2)
    started_at    TEXT NOT NULL,    -- RFC3339 UTC
    ended_at      TEXT NOT NULL,    -- RFC3339 UTC
    duration_secs INTEGER NOT NULL,
    date          TEXT NOT NULL     -- 'YYYY-MM-DD' in UTC (NOT local)
)
indexes:
    idx_activity_segments_date          (date)
    idx_activity_segments_state_started_at (state, started_at)
PRAGMA user_version = 1
```

### 1.2 What is actually written

`src-tauri/src/services/stat.rs:48–67`

```rust
pub(crate) async fn record_rest_session(&self, session: RestSessionDraft) -> Result<()> {
    // INSERT with state='resting' (hardcoded)
}
```

Only `RestSessionDraft` (started_at_utc, ended_at_utc, duration_secs) is persisted, and only when a Resting state successfully completes (or whatever the effect_executor triggers on Resting completion — see `services/timer/effect_executor.rs`).

**This means:**

- ✗ We do **not** persist Working segments.
- ✗ We do **not** persist Skip events (`UserEvent::Skip` from `Alerting`).
- ✗ We do **not** persist Pause/Resume events.
- ✗ We do **not** persist *why* a rest was suppressed (`SkipFlags { fullscreen, schedule, afk, process_whitelisted }` is computed but discarded).
- ✗ We do **not** persist mode (20-20-20 vs Pomodoro) per session.
- ✗ We do **not** persist long-break vs short-break distinction in Pomodoro mode.
- ✗ We do **not** persist foreground process name (even though `process_whitelist` knows the API).

The `state` column being TEXT with only one value today is **schema headroom** — the schema was designed to admit other states later.

### 1.3 TOML config

Captured but not "logged over time":

- `timer.work_minutes` / `rest_seconds` / `pre_alert_seconds` / `alert_timeout_seconds` / `mode`
- `pomodoro.focus_minutes` / `short_break_minutes` / `long_break_minutes` / `cycles_per_long`
- `behavior.sound_enabled` / `fullscreen_skip` / `afk_skip_enabled` / `afk_threshold_minutes` / `auto_start` / `process_whitelist_enabled` / `process_whitelist`
- `schedule.enabled` / `active_days[7]` (Mon–Sun)
- `display.language` / `theme`
- `hotkeys` — full keybindings

There is **no history** of config changes. Changing `afk_threshold_minutes` mid-day silently changes the meaning of existing data.

---

## 2. Aggregation (live, no caching)

`src-tauri/src/services/stat.rs:269–309` (`aggregate_sessions`):

For each rest session:

1. Convert UTC → user's IANA tz (or override).
2. Drop into 3 bucket maps:
   - daily: key = `YYYY-MM-DD` in local tz
   - weekly: key = `YYYY-Www` (ISO week)
   - monthly: key = `YYYY-MM`
3. Each bucket accumulates `rest_sessions` (count) and `total_rest_secs`.

Returned as `StatisticsTrendPayload { timezone, daily[], weekly[], monthly[], total_sessions, total_rest_secs }`.

**Coverage gaps in current aggregation:**

- No hour-of-day distribution.
- No day-of-week distribution (we have it implicitly, but we don't *expose* it as such).
- No streak (consecutive days with ≥N rests).
- No "completion rate" (no concept of "expected vs taken" because we never persist expectation).
- No "session length distribution".

---

## 3. Frontend rendering

`src/pages/main/StatisticsPage.svelte`:

- **Hero card** with eyebrow ("STATISTICS") + title + Export-Backup + Refresh buttons.
- **Metric grid** of 3 cards: `total_sessions`, `total_minutes`, `timezone`.
- **Chart card** with range toggle (Daily / Weekly / Monthly) — ECharts canvas:
  - X-axis: bucket labels.
  - Left Y-axis bar series: `rest_sessions`.
  - Right Y-axis line series: `restMinutes` (smooth).
  - Two-color palette (`--accent`, `--state-active-label`).
- **Export Backup button** opens save dialog → `exportStatistics(path)` → SQLite `VACUUM INTO`.

The chart is generic (count + minutes), no commentary, no annotation, no goal line, no comparison band.

---

## 4. Eyezen-specific signals already available but unrecorded

These are the **highest-leverage** data points already computed but discarded:

| Signal | Where it lives | Currently used for | Could power |
|---|---|---|---|
| `SkipFlags.fullscreen_active` | `timer/state.rs:55` | suppress prompt | "you skipped 4 rests today due to fullscreen apps" |
| `SkipFlags.schedule_inactive` | `timer/state.rs:55` | suppress prompt | weekday vs weekend split |
| `SkipFlags.afk_active` | `timer/state.rs:55` | suppress prompt | distinguish "user paused themselves" from "we paused for them" |
| `SkipFlags.process_whitelisted` | `timer/state.rs:55` | suppress prompt | "you spend most blocked time in {process}" |
| `UserEvent::Skip` from Alerting | `timer/state.rs:39` | direct state transition | adherence rate (taken / (taken + skipped)) |
| `UserEvent::Pause` / `Resume` | `timer/state.rs:40-41` | state transition | manual pause duration |
| `is_long_break` | `timer/state.rs:94` | Pomodoro break duration | long-break adherence vs short-break |
| `cycle_index` | `timer/state.rs:88` | Pomodoro counter | "you most often abandon Pomodoro at cycle X" |
| Foreground process name | `platform/` (already implemented) | whitelist match only | optional "context-of-rest" insight |
| `DetectorCapabilities` per OS | `models/types.rs:37` | UI capability gating | could explain why insights vary by OS |

---

## 5. Backend services in the loop

For implementation context (subset relevant to analytics):

- `TimerService` — owns `Inner` state, emits state transitions, calls effect executor.
- `StatService` — only `record_rest_session()` and `statistics_trends()` (+ export).
- `DetectorService` — produces SkipFlags every tick; the result is currently consumed by TimerService only, not by StatService.

Wiring a "skip events / work_segments" recorder would require a new write path:
DetectorService outputs → bus → StatService. Or simpler: `timer/effect_executor.rs` (which already handles `RestSessionDraft`) gets new effect variants like `Effect::PersistSkipEvent` and `Effect::PersistWorkSegment`.

---

## 6. What this means for proposals

**Free (no schema change)** insights using current `activity_segments` table:

- Hour-of-day histogram of rest sessions
- Day-of-week histogram
- Streak (consecutive days with ≥N sessions; user-set N)
- 7-day vs 30-day rolling averages
- Day-of-week × hour-of-day heatmap
- Median session length, p95 session length

**Schema v2 unlocks** (one migration, additive only):

- Adherence rate (need to persist Skip events)
- Suppression breakdown (need to persist SkipFlags at each suppress event)
- Work segment length distribution (need to persist Working state durations)
- Mode/cycle reporting (need to persist mode and pomodoro cycle context)

**New instrumentation needed** (privacy decision required):

- Per-rest foreground process / window class (sampling at rest-start or work-segment-start; not continuous)
- Subjective check-in ("how do your eyes feel?" 1–5 scale) — purely user-driven event

All of the above stays local-first. None needs cloud.
