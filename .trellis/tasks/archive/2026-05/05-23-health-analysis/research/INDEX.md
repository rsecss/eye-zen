# Health Analysis Research — Index

- **Task**: `.trellis/tasks/05-23-health-analysis`
- **Date**: 2026-05-23
- **Researcher**: Claude (Opus 4.7 1M) — Research Agent

## Files

| # | File | Purpose |
|---|------|---------|
| 1 | [`01-existing-products-eyecare.md`](./01-existing-products-eyecare.md) | Eye-care / micro-break tools and what their analytics actually do |
| 2 | [`02-existing-products-general.md`](./02-existing-products-general.md) | RescueTime / ActivityWatch / Screen Time as inspiration |
| 3 | [`03-eyezen-current-state.md`](./03-eyezen-current-state.md) | What Eyezen records, aggregates, and renders today (codebase-grounded) |
| 4 | [`04-medical-guidelines.md`](./04-medical-guidelines.md) | AOA / WHO / NIOSH on digital eye strain & 20-20-20 evidence |
| 5 | [`05-innovation-proposals.md`](./05-innovation-proposals.md) | 5 differentiated proposals with claim / evidence / feasibility / MVP |

## Important methodology caveat

**Web search MCP tools (`mcp__grok-search`, `mcp__exa`) listed in the task brief
were NOT exposed in this environment.** External product-feature claims in
files 01, 02, 04 are therefore based on:

1. Existing research already in the repo (`docs/.local/research/projecteye-research.md`, `blinkeye-research.md`) — these are dated 2026-03-18 and graded "high confidence".
2. Author's prior knowledge of public product pages / GitHub READMEs (current to training data).

Every product claim is tagged either `[verified-in-repo]` (from existing research files) or `[prior-knowledge-unverified]`. The user/main agent should re-verify any `[prior-knowledge-unverified]` claim against the live product page before committing to a roadmap.

## Headline finding

The dominant pattern across eye-care tools is **"counter dashboards"** — they
show *how many* rest sessions / *how many* minutes. Almost none expose
**rhythm**, **adherence pattern**, **risk** or **behavior insight**. The big
productivity tools (RescueTime, ActivityWatch) show how this is done, but no
one has translated that vocabulary back to eye-care. **This is the gap.**

See `05-innovation-proposals.md` for 5 concrete proposals; the strongest is
**Proposal 1 — Eye-Strain Risk Index** (0-100 composite daily score).

## Top 3 open questions for the user

1. **Privacy ceiling**: are we willing to passively record per-minute foreground process / window-title hashes locally to enable "context-of-strain" insights, or must we stick to rest-session events only? This decides whether Proposals 3 and 4 are feasible.
2. **Data instrumentation budget**: are we OK adding 2-3 new SQLite tables (work_sessions, schedule_events, possibly subjective_check_in) in a schema_v2 migration, or must we stay backward-compatible on the single `activity_segments` table?
3. **Score philosophy**: do we want a *normative* score ("you got 78/100, target is 90+") or *descriptive* analytics ("you took 12 breaks today, your 7-day median is 9")? The former needs anchored thresholds — likely from AOA guidance; the latter avoids judgment but is less actionable.

## Could-not-research (honestly flagged)

- Stretchly's exact dashboard chart set (only know it has "schedule preview"); no live fetch.
- EyeLeo statistics page — believed minimal/none; not verified live.
- Iris Pro's commercial analytics — paywalled, no source access.
- Peer-reviewed evidence quantifying *which* break frequency reduces CVS — claims here are at guideline level (AOA / NIH MedlinePlus), not RCT level.
