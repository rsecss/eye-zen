# Medical / Scientific Guidelines on Digital Eye Strain & Breaks

- **Date**: 2026-05-23
- **Confidence**: this section relies on prior knowledge of public guidance documents from AOA, AAO, NIH MedlinePlus, OSHA, NIOSH, and WHO. Live URL verification was not possible in this environment.
- **Tag**: `[guideline-level]` means an official body publishes the recommendation but a quantitative RCT may or may not exist; `[RCT-level]` is reserved for citations that came from a peer-reviewed trial we have actually seen. (Currently we have none we can cite verbatim — see Caveats §3.)

## 1. The 20-20-20 rule itself

### What the rule actually says

> Every 20 minutes, look at something 20 feet (~6 m) away for at least 20 seconds.

- **Origin**: attributed to Jeffrey Anshel, OD (American optometrist), late 1990s. Not the product of a single RCT; widely endorsed by AOA and AAO. `[guideline-level]`
- **AOA position page**: "Computer vision syndrome" — endorses 20-20-20 plus blink reminders. `[guideline-level]`
- **NIH MedlinePlus** "Computer vision syndrome" entry: endorses 20-20-20. `[guideline-level]`
- **AAO EyeSmart** consumer page: endorses 20-20-20. `[guideline-level]`

### What is actually evidence-based vs convention

| Claim | Evidence level | Note |
|---|---|---|
| Frequent micro-breaks reduce subjective eye-strain symptoms | **multiple small studies / guideline** | the *direction* is settled; the *exact* 20/20/20 numbers are conventional |
| 20 feet specifically is required | **convention** | the underlying mechanism is accommodation relaxation; *any* sufficiently far gaze works (~6m+) |
| 20 seconds specifically is required | **convention** | "long enough to fully relax accommodation" — sometimes recommended as 30s+ in clinical sources |
| Blinking 10–15 times during the break helps | **guideline** | tear-film stability; CVS is partly dry-eye-driven |

### Implication for Eyezen analytics

We can't honestly tell a user "you had X mg less eye strain because you took N breaks". We **can** tell them "you adhered to 20-20-20 N times out of M expected". The score we expose must be an **adherence proxy**, not a clinical outcome.

## 2. Related guidance bodies

### 2.1 NIOSH / OSHA (workplace ergonomics)

- General recommendation: brief breaks every **20–30 minutes** away from VDT; longer "rest pause" every **2 hours**. `[guideline-level]`
- This is the canonical basis for Pomodoro's 25/5/15 pattern *also* matching eye-care guidance, not just productivity. **Eyezen's existing Pomodoro mode is medically reasonable, not just productivity theater.**

### 2.2 WHO

- "Optimal screen time" guidance from WHO is mostly **age-targeted for children** (<1y: zero; 2–4y: <1h). Adult workplace guidance defers to occupational-health bodies. **Not directly useful** for Eyezen's adult-desk-user persona.

### 2.3 AOA Computer Vision Syndrome guidance

Standard mitigations endorsed:

1. 20-20-20 rule.
2. Conscious blinking.
3. Screen 20–28 inches from eyes; top of screen at or slightly below eye level.
4. Reduce glare / increase ambient contrast.
5. Lubricating drops if symptoms persist.
6. Comprehensive eye exam if symptoms persist >2 weeks.

**Eyezen scope**: items 1–2 are directly actionable in-app; items 3–4 are environmental and we cannot measure them; 5–6 are referral content (a one-time tip).

## 3. Measurable behaviors that *could* link to outcomes

For any "score" Eyezen builds to be defensible, its inputs should plausibly correlate with reduced symptoms. Candidates with at least **guideline-level** backing:

| Behavior measurable in Eyezen | Linked to outcome by | Confidence |
|---|---|---|
| Number of rest sessions per work hour | AOA, NIOSH | high |
| Length of longest unbroken work segment | NIOSH "rest pause every 2h" | high |
| Adherence rate (taken / expected) | implicit in any guideline | medium — there is no RCT showing 80% adherence beats 60%, but face-valid |
| Late-evening usage flag | circadian / sleep literature (not strictly eye-strain but workplace wellness) | medium |
| Skip during fullscreen video | none formally; common-sense "you're staring more" | low |

What is **NOT** an evidence-backed health signal but might still be useful UX:

- Streaks (no clinical basis; gamification only)
- "Score went up 5%" comparisons (motivational only)

## 4. Caveats / honest limits

1. **No RCT was sourced live in this session.** All claims here are from generalised familiarity with AOA/AAO/NIH guidance. The repo's roadmap should re-verify against current AOA position statements before marketing language is locked in.
2. **The 20-minute interval has weaker evidence than the rule's popularity suggests.** Several optometry papers note that *any* regular distance-gaze interval helps; the magic of "20" is round-number convention.
3. **Subjective symptom measurement** (e.g., DEQ-5 dry-eye questionnaire, CVS symptom score) is RCT-validated and would let Eyezen offer a **monthly self-assessment** that correlates with literature. Opt-in only. See Proposal 4 in `05-…`.
4. **Anti-marketing rule**: Eyezen MUST NOT claim "reduces eye strain by X%". We can claim "helps you follow the 20-20-20 rule recommended by [AOA]" with citation, and "measures your adherence to that rule" — these are factually defensible.

## 5. Citations to verify before public claims

Items to fact-check against live URLs when web search is available:

- AOA "Computer vision syndrome" — https://www.aoa.org/healthy-eyes/eye-and-vision-conditions/computer-vision-syndrome
- AAO EyeSmart "Computer eye strain" — https://www.aao.org/eye-health/tips-prevention/computer-usage
- NIH MedlinePlus "Computer vision syndrome"
- DEQ-5 questionnaire (Chalmers RL et al., 2010) — for subjective dry-eye self-check
- NIOSH ergonomic recommendations on VDT work

(URL strings above are from prior knowledge; reconfirm before linking from product UI.)
