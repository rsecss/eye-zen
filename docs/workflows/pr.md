# PR Workflow

## Rules

- All PRs target `main`.
- Release and hotfix changes go through PR.
- PR descriptions are in English.
- Squash merge is preferred; the head branch is auto-deleted on merge.

## Pre-flight

```bash
git status --short --branch
npm run ci
git push -u origin <type>/<scope>
gh pr create --base main --head <type>/<scope>
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
