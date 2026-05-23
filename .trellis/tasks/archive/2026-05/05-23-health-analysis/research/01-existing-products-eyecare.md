# Existing Products — Eye-Care / Micro-Break Tools

- **Date**: 2026-05-23
- **Confidence legend**:
  - `[verified-in-repo]` — sourced from `docs/.local/research/projecteye-research.md` or `blinkeye-research.md` (graded high-confidence in those reports, dated 2026-03-18)
  - `[prior-knowledge]` — author's prior knowledge of public product README / website. Should be re-verified before commitment.
  - `[inferred]` — logical conclusion from feature absence in available docs.

## Focus

We are interested in **analytics / stats / reports / insights**, not the rest-prompting feature itself. The question for each product is:

> "If a user opens this app's Stats page today, what story does it tell them, and what action can they take?"

---

## 1. ProjectEye (C#/WPF, Windows-only, MIT, stopped 2022) `[verified-in-repo]`

Source: `docs/.local/research/projecteye-research.md` §4.4

- **Storage**: SQLite via EF6 (`data.db`), table includes rest events and Pomodoro records.
- **Page**: `StatisticWindow` with **3 layers**: weekly / monthly / Pomodoro analysis.
- **Charts**: animated comparison cards + bar charts; not generic — has **hard-coded rule-based commentary**.
- **Output**: can export to **xlsx**, can generate a **monthly report image** (PNG-style).
- **"Score / level" mechanic**: present — there is an **"等级标签 (level label)"** system tied to weekly analysis. Exact thresholds not in research note.
- **Actionable insight**: explanatory copy generated from hard-coded rules (e.g., interpreting the weekly cards).
- **Coverage gap**: no continuous time-of-day distribution, no anomaly detection, no goal tracking.

**Why it matters for us**: ProjectEye is the only competitor with **rule-based commentary + level system**, and it's already MIT-archived — we can study the *rule design* (even without running the binary) but the project itself is dead and Windows-only. The level label idea is the prior art most aligned with Proposal 1.

---

## 2. Blink Eye (Tauri 2 + React, cross-platform, GPL+paid) `[verified-in-repo]`

Source: `docs/.local/research/blinkeye-research.md` §3.3, §4.3

- **Storage**: `UserScreenTime.db` (SQLite from Rust) + `appconfig.db` + several others.
- **Table**: `time_data(id, date, first_timestamp, second_timestamp)` — every 60s the `second_timestamp` is bumped. This effectively measures **app-driven screen-on duration**, NOT *user-active* time. Idle filter is **not implemented** (idle crate is commented out).
- **Stats page**: time-range tabs (Day / Week / Month / Year / All) with **Recharts** bar/line charts of "screen time". Front-end runs `SELECT` + JS aggregation directly.
- **Insight layer**: **none** detected. Pure descriptive charts.
- **Score / streaks / anomaly**: none.
- **Coverage gap**: doesn't even know the user is at the desk; "screen time" is wall-clock since app start. This is the **weakest analytics in the competitor set despite the most active development**.

**Why it matters for us**: Blink Eye is the closest tech-stack peer (same Tauri-Rust-SQLite-charts pattern) and the gap is huge — they have charts but no story. Eyezen already does idle filtering (AFK detection); we can leapfrog with minimal architectural change.

---

## 3. Stretchly (Electron, MIT/BSD-2, very active) `[prior-knowledge]`

- **Storage**: configurable, stores break history locally.
- **Stats page**: known to have a **"break statistics" section** showing breaks taken vs scheduled and a **"schedule preview"** (timeline of upcoming breaks). Date-range filter exists.
- **Insight layer**: very thin — counts only. No score, no anomaly, no commentary. `[inferred]` from product README and screenshots.
- **Unique mechanic**: has a **"Postpone vs Skip vs Take"** distinction that gets recorded — Eyezen currently records "Skip" only via `SkipFlags`, and *doesn't persist skip events* (only completed rests go into `activity_segments`).

**Why it matters for us**: Stretchly's *event vocabulary* (postpone / skip / taken) is richer than Eyezen's `state='resting'` row. If we want adherence analytics ("you skipped 4 of 12 breaks today"), we need to extend our event schema.

---

## 4. EyeLeo (Windows native freeware, abandoned) `[prior-knowledge]`

- **Stats page**: very minimal or none. Product was always focused on *exercises* (animated eye drills), not on instrumentation.
- **Insight layer**: none.

**Why it matters for us**: Confirms that the historical eye-care category was built on "prompt people, don't measure them". The whole *category* has under-invested in analytics. Big opportunity.

---

## 5. Time Out (macOS, free) `[prior-knowledge]`

- **Insight layer**: none reported. Counts/history exposed in app preferences as a list; no charting.
- Unique: supports **dual schedules** (short + long breaks) — Eyezen now has this via Pomodoro mode (focus + short + long).

---

## 6. Workrave (Linux/Windows, open-source, RSI focus) `[prior-knowledge]`

- **Storage**: rolls daily/weekly/monthly **stats files** locally.
- **Stats page**: a real one — exposes:
  - daily totals of mouse clicks / keystrokes / mouse distance
  - "total active time"
  - **per-break-type taken/skipped/natural counts**
- **Insight layer**: again descriptive only; no score, but the **dimensionality is the richest of any open-source eye-care tool**. They count *typed characters* and *mouse-meters* as activity intensity proxies.

**Why it matters for us**: Workrave proves that **activity intensity** (not just time) is a legitimate health signal users want to see, and it has worked in production for ~20 years. Eyezen does *not* currently measure intensity, but we could derive a proxy (work session length distribution + AFK boundaries).

---

## 7. Iris (commercial Cross-platform) `[prior-knowledge]`

- **Marketed claim**: includes "blue-light analytics" and break analytics in Pro tier.
- **Detail**: paywalled, cannot verify chart set.
- `[inferred]` from marketing pages: still per-day counters, not behavioral insight.

---

## 8. f.lux `[prior-knowledge]`

- Out of scope-ish. f.lux is about color temperature, not break tracking. **No break analytics**, no insights page.

---

## Cross-product summary

| Product | Counter | Charts | Score | Adherence (skip vs taken) | Time-of-day | Commentary | Streak | Export |
|---|---|---|---|---|---|---|---|---|
| ProjectEye | ✓ | ✓ | level | partial | partial | **rule-based** | ? | xlsx + image |
| Blink Eye | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Stretchly | ✓ | partial | ✗ | **✓ (postpone)** | ✗ | ✗ | ✗ | ✗ |
| Workrave | ✓ | partial | ✗ | **✓ (taken/skipped/natural)** | ✗ | ✗ | ✗ | partial |
| EyeLeo | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Time Out | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Iris | ?paid | ?paid | ?paid | ? | ? | ? | ? | ? |
| **Eyezen today** | ✓ | **✓ (3-range)** | ✗ | ✗ | ✗ | ✗ | ✗ | **✓ (.db backup)** |

## Key takeaways

1. **No competitor offers a composite score.** ProjectEye's "level" is closest.
2. **Adherence event vocabulary** is richer in Stretchly/Workrave than in Eyezen.
3. **Time-of-day distribution** (e.g., "you skip more breaks in afternoon") is in NO competitor.
4. **Rule-based commentary** is only in ProjectEye and only in Chinese, only on Windows, and dead.
5. **Eyezen's `.db` VACUUM-INTO export is unique** among open-source peers — Stretchly/Workrave don't have a clean "give me my raw database" button.
