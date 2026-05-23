# Existing Products — General Productivity / Wellness Analytics

- **Date**: 2026-05-23
- **Purpose**: identify replicable mechanics from mainstream productivity tools that could meaningfully translate to *eye-care* analytics in a local-first Tauri app.
- All claims `[prior-knowledge-unverified]` unless otherwise tagged.

## Why this is in scope

Eye-care products consistently under-deliver on analytics (see `01-…`). But the **mechanics** for turning event streams into insight have been standardized for years in time-tracking / digital-wellbeing tools. We can borrow them.

The filter we apply: a mechanic is "Eyezen-replicable" only if it works with **local data, no account, no cloud sync, no calendar permission**, and fits Tauri + SQLite + a single Rust process.

---

## 1. RescueTime

**Core mechanic**: automatic categorization + daily **Productivity Pulse** (0–100 score).

| Mechanic | Replicable in Eyezen? | Why |
|---|---|---|
| Per-minute foreground app capture | **partial** — Eyezen has a process-detection capability (used for whitelist). Capturing a *per-minute* foreground process trace is a new instrumentation. Major privacy decision. | platform APIs already wired |
| Productivity score 0–100 | **yes** with our own formula | needs ≥2-3 weighted inputs |
| Weekly digest emails | **no** — no mailer; out of scope | requires account/SMTP |
| Goals ("focus 4h on coding") | **yes** in local form | reuse SQLite |
| Alerts ("you spent 2h on social") | **out of scope** for eye-care | not aligned with mission |

**Key idea worth stealing**: *one prominent number* (Productivity Pulse) framed
as the "headline" of the dashboard. Today Eyezen's hero card shows raw counters
(total sessions, total minutes); switching to a **headline score** would be a
straight UX upgrade.

---

## 2. ActivityWatch (open-source, local-first, MIT)

**Core mechanic**: many independent "watchers" feed a local store; a query UI
slices the timeline.

| Mechanic | Replicable in Eyezen? | Why |
|---|---|---|
| Local-first event store | **yes** — already have SQLite | architectural fit |
| Window-title watcher (every Xs) | **possible** — would need new instrumentation + opt-in | aligns with privacy boundary user might accept |
| AFK watcher | **already done** (DetectorService AFK) | reuse |
| Timeline visualisation (Gantt style) | **yes** — ECharts can render bands | one new chart |
| Categorisation rules (user-editable regex → category) | **yes** | adds a rules table |

**Key idea worth stealing**: Eyezen *already* has the boundary primitives
(AFK, foreground process). ActivityWatch's pattern of "many watchers → one
store → query view" maps 1:1 onto what we have, except we currently throw
away most signals (only `state='resting'` rows persist). A schema_v2 that
keeps **work_segments** alongside **rest_segments** unlocks ActivityWatch-style
timeline views with no platform-API change.

---

## 3. Toggl Track

- **Mechanic**: manual time entry + **idle-detection prompt** ("Were you away from 14:32 to 14:51?")
- **Replicable**: the **post-hoc clean-up prompt** is interesting — when Eyezen detects an AFK boundary today, we silently skip the rest prompt; we could optionally **ask the user "was that intentional?"** at AFK end and record a `voluntary_break` event. This gives us "rest sessions you didn't get from us" as a positive signal in adherence math.

---

## 4. Apple Screen Time / Android Digital Wellbeing

- **Mechanic**: OS-level capture, weekly digest, per-app usage time, "pickups" / "first-pickup time" / "longest session".
- **Replicable subset for Eyezen**:
  - "longest unbroken work session today" — we already have all the data (work_segment durations are computable from state transitions).
  - "first focus session of the day" — useful for circadian context.
  - "average session length over 7 days" — trivial roll-up.

**Crucially out of scope**: cross-app usage time (we don't track non-Eyezen
apps), pickups (mobile concept).

---

## 5. Microsoft Viva Insights / Outlook Focus Time

- **Mechanic**: calendar-aware "focus time block" auto-scheduling + weekly digest of "interruptions per hour".
- **Replicable**: not directly — would need calendar permission. **But** the concept of an **"interruptions per focus hour"** rate maps cleanly onto our state machine: count `Alerting → Skip` events per `Working` hour. Skip-rate is a leading indicator of disengagement with the tool.

---

## Mechanics that are NOT replicable / not aligned

| Mechanic | Why we should NOT do it |
|---|---|
| Email weekly digest | no mailer; cloud-y; out of mission |
| Social comparison ("you ranked X% vs peers") | requires telemetry, kills privacy story |
| Coach / chatbot UI | scope creep, LLM dependency |
| Achievements / badges | gamification can backfire for *health* products (cf. Apple's research on streak-stress in fitness rings) — risky for eye-care |

## Cross-tool synthesis

| Mechanic | Source | Eyezen replicable | New data needed |
|---|---|---|---|
| Headline composite score | RescueTime | ✓ | none — derived from existing |
| Per-segment work timeline | ActivityWatch | ✓ | persist work_segments (schema v2) |
| Post-AFK "was that voluntary?" prompt | Toggl | ✓ | one new event type |
| "Longest unbroken session" today | Apple Screen Time | ✓ | none |
| Skip-rate per focus hour | Viva | ✓ | persist Skip events |
| Rule-based commentary | ProjectEye | ✓ | rules engine in Rust |
| Categorised foreground time | ActivityWatch / RescueTime | **only if user opts in** | foreground-process watcher (privacy decision) |

The cluster of items that **need only schema_v2** (persist work segments + skip events + voluntary breaks) and **no new platform API** is the highest-leverage zone. See `05-innovation-proposals.md` Proposal 1, 2, 5.
