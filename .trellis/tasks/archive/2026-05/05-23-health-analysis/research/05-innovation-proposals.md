# Innovation Proposals — Eyezen Health Analysis

- **Date**: 2026-05-23
- **Number of proposals**: 5 (per task quality bar "3–5 high-quality")
- **Ordering**: by recommended priority. Proposal 1 is the headline; 2 and 5 are low-cost complements; 3 is the privacy-bounded power feature; 4 is the science-backed differentiator.
- **Each proposal includes**: Name + claim · Differentiation evidence · Why it matters · Technical fit · MVP scope · Risks / open questions.

---

## Proposal 1 — Eye-Strain Risk Index (ESRI)

### Claim

A single 0–100 daily score that summarises eye-care adherence by composing
**(a) rest count vs expected**, **(b) longest unbroken work segment**, and
**(c) skip rate**, with weighted contribution and AOA-aligned thresholds. The
score is the **headline** on the Statistics page, replacing the current "raw
counter" hero.

### Differentiation evidence

| Competitor | Has a composite score? |
|---|---|
| Blink Eye | no |
| Stretchly | no |
| Workrave | no |
| Time Out / EyeLeo | no |
| ProjectEye | has a *level label* (weekly), not a daily score; rule set is Chinese-only and product is archived |
| RescueTime / Apple Screen Time | yes — but for productivity / general screen time, **not** eye-care |

→ **No active eye-care tool offers a daily eye-care score.** This is genuinely
novel positioning for a v0.6 / v0.7 release.

### Why it matters

- The user pain reported in `docs/.local/research/projecteye-research.md` §4.4
  is "the user opens the stats page and just sees numbers". A score *frames*
  the numbers and creates a single thing to improve.
- Aligns with AOA "follow 20-20-20" framing (`04-medical-guidelines.md` §1).
  The score is **adherence** to a published rule, not a fabricated medical
  claim — defensible.
- Replaces "more is better" cognitive load with one number + tooltip
  explaining the components.

### Technical fit with Tauri + SQLite stack

**Data already available** (per `03-eyezen-current-state.md` §6 "Free"):
- `activity_segments` gives us rest count and timestamps directly.
- "Expected rests" derives from `timer.work_minutes` + active hours today.
- "Longest unbroken work segment" derives from gaps between resting segments
  (or rest_segment.ended_at to next rest_segment.started_at) bounded by AFK.

**Pure compute, no new schema needed for v1.** All math runs in
`StatService::statistics_trends()` extension; can ship behind a feature toggle.

**Where the formula lives**: pure functions in `src-tauri/src/services/stat.rs`
alongside `aggregate_sessions()`. They take `&[StoredRestSession]` + timezone
+ config snapshot and return a new field on `StatisticsTrendPayload`. ts-rs
auto-exports.

### MVP scope (smallest shippable)

1. New struct `EyeStrainRiskIndex { score: u8, components: [Component; 3], generated_at }` returned alongside trends.
2. Frontend: replace one hero card with the score (large number + tooltip).
3. Settings: nothing user-facing; score is on by default.
4. Score formula v1 (illustrative, NOT yet final — user must approve):
   ```
   adherence_score   = clamp((taken / expected) * 100, 0, 100)
   longest_session_p = clamp(100 - max(0, (longest_secs - 3600) / 60), 0, 100)
   skip_rate_p       = clamp(100 - skip_rate * 100, 0, 100)
   ESRI              = round(0.5 * adherence + 0.3 * longest_session_p + 0.2 * skip_rate_p)
   ```
   (Note: `skip_rate` part needs Proposal 2 to be real — otherwise we treat
   `Skip` count as 0 and the term collapses to 100 until v2.)
5. Copy MUST say "follow 20-20-20" not "reduce eye strain".

### Risks / open questions

- **Q1**: Normative vs descriptive framing? (open question to user). If
  normative, where is the "good" threshold? Provisional: 80 = good, 60 =
  okay, <60 = needs attention. Source: judgment, not RCT.
- **Q2**: Should ESRI degrade on weekend / non-work-hours? Probably yes — the
  weekday-schedule config (`ScheduleConfig`) already tells us when the user
  expects to work. We should NOT punish a Saturday user for not taking
  breaks during inactive days.
- **R1**: Score volatility on partial days (early morning). Mitigation: show
  the score only after the user has had ≥1 expected rest cycle today;
  otherwise show "warming up".
- **R2**: Misinterpretation risk. Mitigation: tooltip always shows the
  formula; an "Explain" link opens an in-app explainer (no marketing claim).

---

## Proposal 2 — Adherence Timeline & Suppression Breakdown

### Claim

Persist **all** rest-cycle outcomes (not just completed rests): **taken**,
**skipped (user)**, **suppressed (fullscreen / schedule / AFK / process
whitelist)**, plus the **why**. Render a timeline ribbon and a
suppression-cause pie / stacked bar.

### Differentiation evidence

| Competitor | Persists suppression cause? |
|---|---|
| Blink Eye | no — doesn't even have suppression |
| Stretchly | persists postpone/skip but not *cause* |
| Workrave | persists taken/skipped/natural — most advanced — but no cause taxonomy |
| ProjectEye | dispatches suppression but does not store/report it |

→ **Eyezen's `SkipFlags { fullscreen, schedule, afk, process_whitelisted }`
is the richest suppression taxonomy in any open-source tool.** It already
exists in memory; we just throw it away. **Persisting it is a unique
unlock no competitor can ship next quarter.**

### Why it matters

- Answers the user's most natural question: *"why didn't I get more rests
  today?"* — "you were in fullscreen for 2h", "VS Code was whitelisted for
  35 minutes", "AFK from 14:00–14:45".
- Validates Eyezen's own DND policies: if a user sees 80% of skips are from
  process whitelist, they can audit the list.
- Provides a real input for ESRI's `skip_rate` component (Proposal 1).

### Technical fit

**Needs schema v2** (`03-eyezen-current-state.md` §6 "Schema v2 unlocks"):

```sql
-- New table; activity_segments stays for backward compatibility / export.
CREATE TABLE rest_cycle_events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    occurred_at TEXT NOT NULL,        -- RFC3339 UTC
    outcome     TEXT NOT NULL,        -- 'taken' | 'skipped' | 'suppressed'
    reason      TEXT,                 -- NULL | 'fullscreen' | 'schedule' | 'afk' | 'process_whitelisted'
    process_hint TEXT,                -- optional, only when reason='process_whitelisted'
    duration_secs INTEGER,            -- non-null when outcome='taken'
    mode        TEXT NOT NULL,        -- 'twenty_twenty_twenty' | 'pomodoro'
    is_long_break INTEGER NOT NULL    -- 0/1; only meaningful in pomodoro
);
CREATE INDEX idx_rest_cycle_events_occurred_at ON rest_cycle_events(occurred_at);
PRAGMA user_version = 2;
```

**Write path**: extend `Effect` enum (`src-tauri/src/services/timer/effect.rs`) with `PersistSkipEvent` and `PersistSuppressedEvent` variants emitted from `machine.rs` at the precise transitions. `effect_executor.rs` calls a new `StatService::record_cycle_event()`.

**Read path**: new `StatService::cycle_outcomes(range)` → returns Vec of
event rows + roll-up. Frontend renders ECharts pie.

### MVP scope

1. Schema v2 migration in `stat.rs::migrate()`.
2. Three new effect variants + recorder.
3. One new ts-rs binding: `CycleOutcomesPayload`.
4. Settings page: one new toggle "Record adherence detail" (default on).
5. Statistics page: new "Adherence" sub-tab next to Trends. Pie of
   suppression reasons; ribbon of last 24h outcomes.
6. Backfill: existing `activity_segments` rows become "taken" events in the
   new view via a `UNION ALL` query — no data migration needed.

### Risks / open questions

- **R1**: schema migration risk. Mitigation: schema_version PRAGMA already
  in place; v1→v2 is purely additive.
- **R2**: Persisting `process_hint` is sensitive. Mitigation: only record
  when the process is *already in the whitelist* (= user explicitly opted
  in); never log other foreground processes here.
- **Q3**: Should we retroactively populate `mode` on existing rows by
  reading current config? No — leave NULL for legacy and mark them
  "unknown" in the UI.

---

## Proposal 3 — Rest-Context Heatmap (Opt-in, Privacy-Bounded)

### Claim

A 7-day × 24-hour heatmap showing **when the user most often skips or gets
suppressed**, plus an optional **work-context label** sampled at rest-prompt
time (e.g., "you most often skip rest while in `chrome.exe`"). The label
sample is **opt-in** and only captures the foreground basename, never the
window title.

### Differentiation evidence

- **No eye-care competitor produces a heatmap of any kind.**
  ProjectEye has weekly comparison cards but not a hour-of-day grid.
- **ActivityWatch produces timelines** but with full window-title capture
  by default; Eyezen's privacy-bounded version (basename only, sampled at
  rest-prompt rather than continuously) is a milder, opt-in variant — a
  meaningful product positioning difference.

### Why it matters

- The most useful health insight from `02-existing-products-general.md`
  Viva Insights / RescueTime category is **temporal patterns** ("you crunch
  at 3pm; that's where you skip"). Eyezen has the timestamps to build this
  without any new API.
- The work-context label upgrade adds **why** to the **when**, which is the
  conversation users actually want.

### Technical fit

**Heatmap (no extra data needed)**: pure derivation from `rest_cycle_events`
(Proposal 2) or, in a degraded mode, from `activity_segments` (only taken
rests; less useful but still real).

**Context label**:
- Reuse existing `platform/foreground-process` query (already implemented
  for whitelist).
- New sampling moment: at transitions `Working → PreAlert` and `Alerting →
  {Skip|StartRest}` only. Sampling frequency = user's rest cadence, so for
  20-minute users this is 3 samples/hour. **This is *not* a continuous
  watcher**; it's an event-anchored sample.
- Storage: extend `rest_cycle_events.process_hint` already proposed in P2 to
  fire for *all* outcomes when the user opted in.

### MVP scope

1. **Phase A (no opt-in)**: Heatmap rendering only over Proposal 2 data —
   ship together with Proposal 2.
2. **Phase B (opt-in)**: Settings toggle "Include app context in
   statistics" (default OFF). When ON, sample foreground process name at
   transition points; store in `process_hint` for *all* outcomes (not just
   suppression). Frontend shows a "by app" panel.
3. Reuse existing process-detection capability gating
   (`DetectorCapabilities.foreground_process_detection_supported`); silently
   degrade on platforms where this isn't available.

### Risks / open questions

- **Q4**: Should we hash process names locally to reduce sensitivity of
  the `.db` export? Probably no — users export their own DB to themselves;
  raw names are clearer.
- **R3**: Sampling at transition only is NOT representative of *what app
  the user is in for the whole work segment*. We must label the panel
  honestly: "the apps where you were most often prompted to rest", not
  "the apps where you worked the most".
- **R4**: macOS/Linux process detection has gaps (Wayland in particular);
  the panel must visually show "partial data" when capability is degraded.

---

## Proposal 4 — Subjective Symptom Check-in (Weekly DEQ-5-lite)

### Claim

Once a week, on a user-chosen weekday, prompt a **3-question subjective
self-check** ("how often did your eyes feel dry / tired / blurry this week?",
1–5 Likert). Persist locally and chart the symptom trendline against the
adherence score (Proposal 1). When trend diverges, surface
**non-prescriptive** advice.

### Differentiation evidence

- **No** open-source eye-care tool has a subjective symptom self-check.
- DEQ-5 (Dry Eye Questionnaire, 5-item) and CVS-Q (Computer Vision Syndrome
  Questionnaire) are clinically validated; using a *lite* 3-question variant
  in a wellness app is legitimate as long as we never present results as
  diagnosis. `[guideline-level]` (see `04-medical-guidelines.md` §3).

### Why it matters

- This is the **only proposal that connects adherence to outcome**, even
  weakly. Without it, ESRI is purely behavioral.
- A line that shows "adherence trending up, symptoms trending down" is the
  most motivating possible UX for a health tool. The reverse is a useful
  signal to recommend the user see an optometrist.
- It's also Eyezen's **most defensible "we care about health, not just
  productivity"** moat against general productivity-timer competitors.

### Technical fit

**New tiny table**:

```sql
CREATE TABLE symptom_check_ins (
    occurred_at TEXT PRIMARY KEY,         -- weekly cadence; one row per check-in
    dry         INTEGER NOT NULL,         -- 1..5
    tired       INTEGER NOT NULL,
    blurry      INTEGER NOT NULL,
    notes       TEXT                      -- optional, user-typed
);
```

**Prompt mechanism**: reuse existing `TipWindow` infrastructure? **No** — the
TipWindow is a *rest* prompt; mixing UX is bad. Instead: in-app modal in the
Statistics page when the user opens it on a check-in day; or a discreet
banner in `TrayApp` ("It's been a week — quick eye check?").

**Storage**: standalone table, never exported by default in the VACUUM-INTO
backup unless user toggles "include symptoms in export" (separate sensitivity).

### MVP scope

1. New schema_v3 with `symptom_check_ins` table.
2. Settings: "Weekly eye check-in" toggle + day-of-week picker (default OFF
   — opt-in for sensitivity reasons).
3. New page tab in StatisticsPage: "Symptoms" — line chart of dry/tired/
   blurry over time + overlay of weekly ESRI average.
4. In-app modal triggered only when user is *already* in the Settings/Stats
   window on a check-in day. No tray nag for v1.
5. Copy MUST link to AOA "see an optometrist if symptoms persist >2 weeks"
   when any score ≥4 sustained for 2 check-ins (recommendation only; no
   diagnosis).

### Risks / open questions

- **R5**: Sample frequency (1/week) gives 4 datapoints/month — chart looks
  thin. Mitigation: that's fine — show a rolling 8-week trendline.
- **R6**: User skips check-ins; missing data biases the trend. Mitigation:
  no inference about missing weeks; just gaps in the chart.
- **Q5**: Should we use validated DEQ-5 questions verbatim? They have
  copyright considerations on the questionnaire wording. Need legal
  check. Alternative: write our own 3 questions inspired by DEQ-5
  themes (dryness, irritation, blur). Probably safer.

---

## Proposal 5 — Streak & Personal-Best Rhythm Cards (Lightweight Motivation)

### Claim

Two small cards under the hero: **"Current streak: 7 days with ≥X rests"**
and **"Personal best: longest streak of {date range}"**. Streak threshold
auto-set from user's median day. No gamification overlay; purely a "your
rhythm" framing.

### Differentiation evidence

- **No competitor exposes streaks for eye-care.** Apple/Google fitness
  rings popularized streaks but for steps, not breaks.
- Apple has publicly noted streak-pressure as a wellness anti-pattern;
  our version is intentionally **muted**: small, calculated against the
  user's own median (not an external goal), no notifications, no streak-loss
  alarm.

### Why it matters

- Cheapest possible UX upgrade — pure SQL.
- Closes a gap users coming from fitness/productivity apps expect.
- Tests whether motivational mechanics matter for Eyezen before we invest
  in bigger ones.

### Technical fit

**Pure SQL** over existing `activity_segments`:

```sql
-- Days with ≥N rest sessions in the user's local timezone
WITH daily AS (
  SELECT date(started_at, ?tz_offset) AS d, COUNT(*) AS c
  FROM activity_segments WHERE state='resting' GROUP BY 1
)
SELECT * FROM daily WHERE c >= ?N ORDER BY d DESC;
```

**Threshold N**: median of last 30 daily counts; computed in Rust, not SQL.

### MVP scope

1. Two new derived fields on `StatisticsTrendPayload`:
   `current_streak_days`, `best_streak_days`, `streak_threshold`.
2. Two new cards on `StatisticsPage`.
3. No schema change.

### Risks / open questions

- **R7**: First-week users have no median → either no streak shown, or
  threshold defaults to `floor(expected_rests_per_day * 0.6)`.
- **R8**: Streak counts can become a perverse incentive ("I held a 30-day
  streak by skipping work"). Mitigation: copy frames it as "days following
  your rest rhythm", not "days worked".

---

## Cross-proposal dependency matrix

| Proposal | Depends on | Unlocks | Schema change |
|---|---|---|---|
| 1 — ESRI | — (full quality needs 2) | a single headline number | none for v1 |
| 2 — Adherence/Suppression | — | upgrades ESRI, enables 3a | **schema v2** (additive) |
| 3 — Heatmap & Context | 2 | "when & where" insight | additive on 2 |
| 4 — Symptom check-in | — | adherence ↔ outcome story | **schema v3** (additive) |
| 5 — Streak cards | — | motivation/rhythm UX | none |

**Recommended phasing**:

- v0.6.0: P1 + P5 (no schema change, low risk, big UX delta)
- v0.7.0: P2 (schema v2) + upgrade P1 formula
- v0.8.0: P3 phase A (heatmap) + opt-in P3 phase B (context)
- v0.9.0: P4 (schema v3) — most sensitive, save for after the others stabilise

This phasing keeps each release independently shippable, reversible (purely
additive schemas), and demoable.
