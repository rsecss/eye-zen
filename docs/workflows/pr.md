# PR Workflow

## Rules

- PRs target `main`.
- Release and hotfix changes also go through PR.
- PR descriptions must be in English.
- Wait for `dev` CI before opening release PRs.
- Squash merge is preferred.

## Pre-flight

```bash
git status --short --branch
npm run ci
git push origin dev
gh pr create --base main --head dev
```

## Description

Keep it short:

```markdown
## Summary

- ...

## Test Plan

- [x] `npm run ci`
- [x] GitHub Actions: Windows / macOS / Linux / Security Audit

## Risk

- ...
```

## Required Checks

- Local `npm run ci`
- GitHub Actions on PR
- No unrelated changes
- Docs updated when workflow, public behavior, architecture, or release process changes
