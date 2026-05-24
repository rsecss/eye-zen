# Update Docs Workflow

## Rules

- Prefer deletion and links over duplicated explanation.
- Keep root docs short; put durable rules in `.trellis/spec/`.
- Keep public docs accurate for users; keep agent docs accurate for agents.
- Never rewrite generated or managed blocks by hand.
- Verify against source, not memory.

## Scope

| File | Role |
|------|------|
| `README.md`, `.github/README.zh-CN.md` | Public user-facing docs. |
| `CHANGELOG.md` | Release history. |
| `CLAUDE.md` | Claude-facing project index. |
| `AGENTS.md` | Trellis-managed agent entrypoint. |
| `docs/workflows/*.md` | Human workflow docs. |
| `AGENTS.md` | Trellis-managed agent entrypoint and platform file map. |
| `.trellis/spec/` | Canonical engineering rules. |
| `.claude/index.json` | Machine-readable project index. |

## When To Update

| Change | Update |
|--------|--------|
| User-visible behavior | README, CHANGELOG, relevant workflow/spec doc. |
| Architecture or API contract | `.trellis/spec/`, CLAUDE.md index if needed. |
| CI/release behavior | `docs/workflows/`, `.trellis/spec/architecture/testing-quality.md`. |
| Agent config | `AGENTS.md` (platform file map) and actual platform files. |
| Plan status | `docs/plans/README.md`. |

## Style Constraints

- One fact, one home.
- No long templates unless they are copied into tools directly.
- No historical essays in active workflow docs.
- No relative time words such as "recently" or "today".
- Keep commands executable and current.

## Validation

```bash
git diff --check
npx prettier --check README.md .github/README.zh-CN.md CLAUDE.md docs/**/*.md
```
