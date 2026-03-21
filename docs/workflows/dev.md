# Development Workflow

Daily development cycle from feature idea to merged code.

## Git Hooks (Automatic)

Project uses [husky](https://typicode.github.io/husky/) to run git hooks automatically.

### pre-commit (every `git commit`)

```
git commit → .husky/pre-commit runs → lint-staged (prettier on staged files) → commit succeeds/fails
```

- Runs `npx lint-staged`: auto-formats staged `.ts/.js/.svelte/.html/.css/.json` files with prettier
- Fast (<5s), does NOT block development flow
- Also runs commitlint via `.husky/commit-msg` to enforce Conventional Commits format

### pre-push (every `git push`)

```
git push → .husky/pre-push runs → 7-step validation → push succeeds/fails
```

Steps:
1. `cargo fmt --check` — Rust format
2. `cargo clippy --all-targets -D warnings` — Rust lint (MUST use `--all-targets`)
3. `cargo test` — Rust tests (also regenerates ts-rs bindings)
4. `npx svelte-check` — Frontend type check
5. `npm test -- --run` — Frontend tests
6. `npm run format:check` — Prettier check
7. `npm run build` — Frontend build

- Takes 1-3 minutes, ensures pushed code will pass CI
- Skip with `git push --no-verify` for WIP pushes (MUST NOT skip before release)

### Why two layers

| Hook | When | Scope | Speed | Purpose |
|------|------|-------|-------|---------|
| pre-commit | Every commit | Format staged files only | <5s | Don't break formatting |
| pre-push | Every push | Full compile + test + lint | 1-3min | Don't break CI |

## Development Cycle

```
1. Task Brief         → Define scope, non-goals, acceptance criteria
       ↓
2. Design/Research    → Architecture, IPC interfaces, platform feasibility
       ↓
3. Implementation     → Read existing code → Define interface → Implement → Test
       ↓
4. Local Validation   → pre-push hook catches issues automatically
       ↓
5. Push & CI          → Three-platform CI validates
       ↓
6. Code Review        → Multi-model review for cross-boundary changes
       ↓
7. Commit             → Atomic, Conventional Commits format
```

### Implementation Order (MUST follow)

1. Read existing code, understand patterns
2. List impact scope (per `rules/05-change-management.md` checklist)
3. Define interfaces first, then implement
4. Write implementation code
5. Write corresponding tests
6. Run automated checks (pre-push handles this)
7. Update docs if API changed

### When to Use Multi-model Review

Trigger review when ANY of these apply:
- Cross frontend-backend boundary
- New permissions/plugins
- New async tasks or state machine states
- New persistence/migration
- Changes > 150 lines or > 3 files

| Model | Role | Focus |
|-------|------|-------|
| Claude Code | Primary implementer | Long-chain implementation, iterative development |
| Codex | Code-level reviewer | Blocking issues, missing tests, regression risks |
| Gemini | Requirements reviewer | Missing scenarios, uncovered state transitions |

## Branching

- `dev` — Primary development branch, all work happens here
- `main` — Release branch, only receives merges via PR
- `release/vX.Y.Z` — Short-lived release branches (see release workflow)
- `fix/<name>` — Hotfix branches from main (see release workflow)

## Session Management

Start a new session when ANY of these apply:
- Changes span > 8 files or > 3 modules
- Session covers > 1 major feature
- AI starts repeating or contradicting earlier decisions
- Context window has been compressed

## References

- Detailed 10-stage lifecycle guide: `docs/.local/dev-workflow.md` (local reference)
- Commit conventions: [Conventional Commits](https://www.conventionalcommits.org/)
- Testing requirements: `rules/04-testing-quality.md`
- Change checklists: `rules/05-change-management.md`
- Release workflow: `docs/workflows/release.md`
