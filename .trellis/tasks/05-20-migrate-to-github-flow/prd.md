# Migrate to GitHub Flow

## Goal

将仓库从「dev + main 双分支 + squash merge + back-merge」工作流迁移到 GitHub Flow（单 main + 短生命 feature 分支 + self-PR + squash merge + tag-based release），消除 dev/main 分支同步开销（如 `dev ahead main N commits` 噪音、release 后 back-merge 步骤等），同时保留所有现有质量门禁（PR CI、squash 历史、分支保护、pre-push hook）。

## Requirements

### 代码 / 配置层

- [ ] `.github/workflows/ci.yml` — `on.push.branches` 从 `[main, dev]` 改为 `[main]`
- [ ] `CONTRIBUTING.md` — "Target dev" 改为 "Target main"，移除 dev checklist
- [ ] `.husky/pre-push` 保持不变（与分支无关）
- [ ] `.husky/pre-commit` 保持不变

### 文档层

- [ ] `docs/workflows/dev.md` — Branches 表删除 dev 行；Flow 步骤更新为基于 main 开 feature 分支；规则首条"Work on dev"改为"Work on short-lived feat/fix/chore branches off main"
- [ ] `docs/workflows/release.md` — Release 起点改为 main；步骤 1 改为"Validate main locally"；删除步骤 8（back-merge）；命令示例中 `git push origin dev` / `git checkout dev` 全部移除；命令示例改为 `git checkout main && git pull && git checkout -b release/vX.Y.Z`
- [ ] `docs/workflows/pr.md` — Pre-flight 移除 `git push origin dev`，PR 基线改为基于 feat/fix branch；Rules 中"Wait for dev CI"删除
- [ ] `docs/workflows/branch-protection.md` — 删除 `## dev` 章节；保留 main 章节
- [ ] `CLAUDE.md` — 分支保护描述中的"main/dev 保护"改为"main 保护"
- [ ] `.claude/index.json` — `branch: "dev"` 改为 `branch: "main"`（如指 current branch；若是默认 branch 也需同步）

### Memory 层

- [ ] `memory/release-v020-experience.md` — 在"Key Lessons" 第 3 条加 deprecation note：「已切换 GitHub Flow，dev/main 双分支模型已弃用」
- [ ] `memory/MEMORY.md` — 删除或更新"squash merge + back-merge: dev 永远 ahead main N commits" 条目，改为"已迁移到 GitHub Flow（2026-05-20）"

### 操作层

- [ ] 在 dev 上完成所有上述文件改动 + commit
- [ ] 开 PR `chore(workflow): migrate to GitHub Flow` dev → main
- [ ] PR CI 全绿后 squash merge（带 `--delete-branch` 删 release/feature 分支不影响 dev 自身）
- [ ] checkout main + pull
- [ ] GitHub Settings 取消 dev 的 branch protection rule（如有）
- [ ] `git push origin --delete dev` 删 remote dev
- [ ] `git branch -D dev` 删本地 dev（依赖 GitHub 14 天 restore 作为底牌，不创建 archive tag）
- [ ] 端到端验证：开一个 trivial PR（例如 docs typo 或 journal）跑通 feat/fix branch → PR → squash merge → branch auto-delete 流程

## Acceptance Criteria

- [ ] `git ls-remote --heads origin dev` 无输出
- [ ] `git branch | grep dev` 无输出（本地已删）
- [ ] `git remote show origin | grep "HEAD branch"` 显示 main
- [ ] GitHub repo Settings：main 是唯一受保护分支
- [ ] 所有 workflow 文档不再出现 `dev` 分支引用（除 historical context 如 devlog/plans）
- [ ] CI 仍然在 push to main + PR to main 上触发，3 平台 + Security Audit 全绿
- [ ] 一个 trivial PR 完整跑通：分支 → push → PR → CI → squash → auto-delete
- [ ] memory/CLAUDE.md/索引文档与现实一致

## Definition of Done

- 所有 Requirements 完成
- 所有 Acceptance Criteria 验证通过
- 一次完整的 PR 流程演练成功
- 不破坏 release.yml（tag 触发不变）
- 不丢失 dev 上已有的 docs sync 内容（CLAUDE.md v0.2.0 状态同步）

## Out of Scope

- 不重写 git 历史（不 rebase main、不删除历史 back-merge commits）
- 不发 v0.2.1 patch release（实际 patch diff 无 runtime 影响，仅 docs sync）
- 不引入 trunk-based / feature flags
- 不动 release.yml 的 tag trigger（已 GitHub Flow 友好）
- 不创建 dev-archive tag（依赖 GitHub 14 天 restore）
- 不批量改 docs/devlog.md / docs/plans/ 等历史文档中的 dev 提及（视为历史快照）

## Decisions (ADR-lite)

### D1 — 一次性 PR 全打包

**Context**: dev 上 24 个 ahead commit 实际 patch diff 只有 2 个文件（CLAUDE.md docs sync + bump-version.mjs squash hash 差异）。

**Decision**: 直接基于 dev 开一个 PR 到 main，标题 `chore(workflow): migrate to GitHub Flow`，包含 (a) docs sync (b) 工作流文档迁移 (c) ci.yml 调整 (d) memory 更新。一次 PR 收尾。

**Consequences**: PR 范围较大但主题单一（"迁移"），review 时直观；省去分多次 PR 的来回。回滚单点。

### D2 — 保留 self-PR 流程

**Context**: 单人项目可以选择直接 push main。

**Decision**: 保留 feat/fix/chore branch → self-PR → PR CI gate → squash merge → auto-delete 流程。

**Consequences**: 保住 (a) PR CI 3 平台 matrix 验证 (b) main 分支保护 (c) squash 历史 (d) PR 作为变更叙事点 (e) 未来引入 contributor 零摩擦切换。代价：每次改动多一步开 PR，但 `gh pr create` 已是命令一行。

### D3 — 直接删 dev，依赖 GitHub restore 作为底牌

**Context**: 是否创建 dev-archive tag 作为本地备份。

**Decision**: remote dev + local dev 都直接删除，不创建 archive tag。

**Consequences**: 清爽。万一需要回溯 dev 上原始 granular commit 历史，14 天内可在 GitHub UI "Restore deleted branch" 恢复。本地 reflog 也可恢复（90 天）。

## Implementation Plan (small commits inside one PR)

PR 标题：`chore(workflow): migrate from dev/main to GitHub Flow`

```
PR breakdown (commits):
  1. chore(ci): drop dev branch from ci.yml push trigger
  2. docs(workflow): rewrite dev.md / pr.md for GitHub Flow
  3. docs(workflow): rewrite release.md (remove back-merge, start from main)
  4. docs(workflow): remove dev section from branch-protection.md
  5. docs: update CONTRIBUTING.md target branch
  6. docs: sync CLAUDE.md and .claude/index.json branch references
  7. (already on dev: 46e31b5 docs: sync project status to v0.2.0 release — squashed in)
```

Post-merge sequence:
  1. checkout main + pull
  2. GitHub Settings → Branches → 删除 dev rule（如有）
  3. `git push origin --delete dev`
  4. `git branch -D dev`
  5. Update memory: MEMORY.md + release-v020-experience.md deprecation note
  6. 开一个 trivial PR（如 docs/journal）验证流程

## Research References

- 2026 best practice：GitHub Flow > simplified dev/main > full GitFlow（for solo/small open-source desktop apps）
- "dev ahead main N commits" 是 squash + back-merge 的预期产物
- Tauri 生态（Tauri / Pake / Yaak）均使用单 main 分支

## Technical Notes

### Files to touch（最终清单）

| 类型 | 文件 | 改动 |
|------|------|------|
| CI | `.github/workflows/ci.yml` | line 5: `branches: [main]` |
| 公开文档 | `CONTRIBUTING.md` | line 9/60: target main; remove dev checklist |
| 工作流 | `docs/workflows/dev.md` | 整体 GitHub Flow 化 |
| 工作流 | `docs/workflows/release.md` | 起点 main + 删 back-merge |
| 工作流 | `docs/workflows/pr.md` | 移除 dev pre-flight |
| 工作流 | `docs/workflows/branch-protection.md` | 删 dev 章节 |
| 索引 | `CLAUDE.md` | 分支保护描述 |
| 索引 | `.claude/index.json` | branch 字段 |
| Memory | `memory/release-v020-experience.md` | deprecation note |
| Memory | `memory/MEMORY.md` | 删除 dev ahead 条目 |

### 不动的文件

- `docs/devlog.md` — 历史记录
- `docs/.local/dev-workflow.md` — 本地参考文档，"dev" 指开发工作流不是分支
- `docs/plans/001-config-service.md` — 历史 plan
- `AGENTS.md` — 无 dev 分支引用
- `.husky/pre-push`, `.husky/pre-commit`, `.husky/commit-msg` — 与分支无关
- `.github/workflows/release.yml` — tag trigger 与 GitHub Flow 兼容
- `src-tauri/*`, `src/*` — 无影响

### Risk & Mitigation

| Risk | Mitigation |
|------|------------|
| 删 dev 后发现遗漏的引用 | 已 grep 全仓库，9 个文件覆盖；不动的 3 个为历史文档 |
| migration PR CI 失败 | 所有改动都是 docs/config 性质，pre-push hook 会跑完整 ci |
| GitHub branch protection 配置失误 | 操作前截图当前 dev rule，操作后立刻验证 main 保护仍生效 |
| 误删 main 分支 | `git push origin --delete dev` 明确指定 dev；命令双重确认 |
