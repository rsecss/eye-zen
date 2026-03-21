# Eyezen Update Docs Workflow

Automatically check code changes since last tag and synchronize all documentation.

## Usage

```bash
/update-docs
```

## Context

- Analyze code changes since last Git tag or last documentation update
- Ensure documentation matches actual implementation
- Maintain consistency across CLAUDE.md, README.md, README.zh-CN.md, CHANGELOG.md, and memory files
- Check that module index, tech stack, and feature lists are accurate

## Documentation Files to Sync

| File | Scope | Key Sections |
|------|-------|-------------|
| `CLAUDE.md` | Project context for AI | Status, tech stack, architecture, module index, changelog |
| `README.md` | Public-facing (English) | Features, download, tech stack, roadmap |
| `README.zh-CN.md` | Public-facing (Chinese) | Must stay consistent with README.md |
| `CHANGELOG.md` | Release history | Version entries with categorized changes |
| `docs/plans/README.md` | Plan tracker | Plan status table (Pending/Implemented) |
| `.claude/index.json` | Machine-readable project index | Status, tech stack, files, modules, gaps, next steps |

## Execution Flow

### 1. Get Changes Since Last Tag

```bash
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")

if [ -z "$LAST_TAG" ]; then
  CHANGED_FILES=$(git diff --name-only HEAD~20..HEAD)
else
  CHANGED_FILES=$(git diff --name-only $LAST_TAG..HEAD)
fi
```

### 2. Categorize Impact Areas

Analyze changed files to determine documentation impact:

| File Pattern | Documentation Impact |
|-------------|---------------------|
| `src-tauri/src/services/*.rs` | CLAUDE.md module index, README features |
| `src-tauri/src/commands/*.rs` | CLAUDE.md module index (command count) |
| `src-tauri/Cargo.toml` | CLAUDE.md tech stack table |
| `package.json` | CLAUDE.md tech stack table |
| `src/pages/**/*.svelte` | CLAUDE.md module index (frontend pages) |
| `src/lib/stores/*.ts` | CLAUDE.md module index |
| `.github/workflows/*.yml` | README CI badges, rules/04 CI section |
| `docs/plans/*.md` | docs/plans/README.md status table |
| `src-tauri/capabilities/*.json` | Permissions documentation |
| `.github/release.yml` | Release notes template consistency |

### 3. Check CLAUDE.md Consistency

Verify each section:

```
- [ ] Project status reflects current phase
- [ ] Tech stack versions match Cargo.toml + package.json
- [ ] Module index matches actual files in src-tauri/src/ and src/
- [ ] Service count is accurate
- [ ] Command count is accurate
- [ ] Frontend page list is complete
- [ ] Phase delivery status is current
- [ ] Changelog has entry for recent work
```

### 4. Check README.md Consistency

```
- [ ] Feature list matches implemented capabilities
- [ ] Platform caveats are accurate (fullscreen detection limits)
- [ ] Tech stack table matches actual versions
- [ ] Build instructions work
- [ ] Roadmap checkboxes reflect completed milestones
- [ ] Version badge matches package.json
```

### 5. Check Plan Status

```bash
# Compare plan files with README index
for plan in docs/plans/[0-9]*.md; do
  PLAN_NAME=$(basename "$plan")
  # Check if status in README matches actual implementation
done
```

### 6. Check .claude/index.json

```
- [ ] project.status reflects current phase
- [ ] techStack entries match Cargo.toml + package.json (installed flags accurate)
- [ ] files section lists all source files (services, pages, components, lib, tests)
- [ ] modules section has correct service/command counts and test counts
- [ ] gaps reflect only remaining unimplemented items
- [ ] nextSteps are current
```

### 7. Check Memory Files

```
- [ ] MEMORY.md project state is current
- [ ] Plan completion status is accurate
- [ ] Technical decisions reflect recent changes
- [ ] Bug fix experience captures new findings
```

### 8. Generate Update Report

```markdown
## Documentation Update Report

### Changes Since $LAST_TAG
- N commits, M files changed

### Documentation Files Requiring Updates

#### CLAUDE.md
- [ ] Item 1 — current value vs expected value
- [ ] Item 2 — ...

#### README.md
- [ ] Item 1 — ...

#### Other
- [ ] Item 1 — ...

### Specific Inconsistencies Found
[Detailed list with file:line references]
```

### 9. Apply Updates

For each identified inconsistency:
1. Read the current documentation file
2. Verify the correct value from source code
3. Apply the targeted update
4. Do NOT rewrite entire sections — only fix the specific inconsistency

### 10. Validation

```
- [ ] All updated files have valid Markdown
- [ ] No broken internal links
- [ ] Version numbers consistent across all files
- [ ] Feature descriptions match code capabilities
- [ ] No placeholder text remains (e.g., "TODO", "user/repo")
```

### 11. Commit Updates

```bash
git add CLAUDE.md README.md CHANGELOG.md docs/plans/README.md .claude/index.json
git commit -m "docs: synchronize documentation with codebase

- [List specific updates made]"
```

## Important Notes

- **NEVER** remove existing content without verifying it's actually outdated
- **NEVER** break markdown formatting or internal links
- **ALWAYS** verify against source code, not assumptions
- **ALWAYS** update memory files when project state changes
- Keep CLAUDE.md under control — it's loaded into every AI session
- README.md is public-facing — keep it concise and accurate
- When in doubt about a feature's status, check the source code first
