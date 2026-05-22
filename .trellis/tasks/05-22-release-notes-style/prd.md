# 统一 release note 风格为 chrome-devtools-mcp 形态

## Goal

把 Eyezen 的 CHANGELOG/release note 从「叙述段落 + 长描述」改为 chrome-devtools-mcp 风格：分类小标题（emoji）+ 一行 imperative 描述 + `(#PR) (sha)`。同时固化为可复用模板，覆盖未来所有发版。

## What I already know

- 当前 `CHANGELOG.md` 每个版本 body 是一段长 prose（v0.4.0 是单段 12 行无 bullet 的混合说明，v0.3.0 三个 feature 各一长段）
- `scripts/extract-changelog.mjs` 取 `## [X.Y.Z]` 与下一个 `## [` 之间的内容作为 GitHub release body
- `.github/release.yml` 是 GitHub 自动生成 PR 列表的 label 分类配置，**实际不被使用**（release 走 extract-changelog → release.yml 内的 body）
- 当前 GitHub 4 个 release 都用的旧风格（CHANGELOG 直接渲染）
- 提交→PR 映射可从 git log 拿到：
  - v0.2.0: #1 #2 #3 + #4 release + #5 hotfix
  - v0.3.0: #10 AFK, #11 SQLite stats, #12 hotkeys + #13 release
  - v0.4.0: #14 whitelist + #15 release
  - v0.1.0: 早于 PR-flow，无 PR# 可用
- 参考样式（image）只有 4 类：🎉 Features / 🛠️ Fixes / 📃 Documentation / 🧪 Refactor

## Assumptions (temporary)

- 保留 `## [X.Y.Z] - YYYY-MM-DD` 头部不动（extract-changelog 依赖它）
- 用户希望既改 CHANGELOG 又同步回填已发布 GitHub releases（待 Q1 确认）
- "release" PR（#4 #13 #15）本身不进 changelog 条目，只发布版本号
- 短 SHA 用 7 位（git 默认）

## Open Questions

- ~~Q1（Blocking, Preference）~~ **Answered**: 全部回填 — CHANGELOG.md + `gh release edit` 同步 4 个已发布 release body
- ~~Q2（Preference）~~ **Answered**: 5 类 — 🎉 Features / 🛠️ Fixes / 📃 Documentation / 🧪 Refactor / 🔧 Maintenance
- ~~Q3（Preference）~~ **Answered**: v0.1.0 仅保留 sha（无 PR#，无 prefix 文案）

## Decision (ADR-lite)

**Context**: 当前 CHANGELOG.md 每个版本 body 是长 prose，与 chrome-devtools-mcp 等开源标杆的「分类 emoji + 一行 imperative + (#PR)(sha7)」风格差距较大。GitHub release body 由 extract-changelog.mjs 抽 `## [X.Y.Z]` 段落而来，所以风格集中在 CHANGELOG.md。

**Decision**:
- 5 类分类：🎉 Features / 🛠️ Fixes / 📃 Documentation / 🧪 Refactor / 🔧 Maintenance
- 行格式：`- description (#NN) (sha7)`，imperative 小写开头，无尾句号
- v0.1.0 无 PR# 时省略 `(#NN)`，保留 `(sha7)`
- 回填 4 个已发布 GitHub release body 用 `gh release edit --notes-file`
- 模板固化进 `docs/workflows/release.md`，未来 bump-version 生成的 stub 直接遵循

**Consequences**:
- 历史归档损失：原 CHANGELOG body 里那些长描述（如 macOS APIs 详情、TOML 兼容、零依赖选择等）只移到 docs/devlog.md 或 memory，不再出现在 release note
- 文档一致性提升：今后所有 release body 来源一致（CHANGELOG → extract → release）
- PR #6 的曲折历史（base=dev，已死分支）说明 mergeCommit ≠ main 实际 SHA，未来引用 PR 编号时要核对 `git log` 真实存在

## Requirements (locked)

- 模板规则：分类标题 H3 含 emoji；每条 `- imperative-lowercase (#NN) (sha7)`；行尾不带句号
- CHANGELOG.md v0.1.0–v0.4.0 body 全部转新风格，保留 `## [X.Y.Z] - YYYY-MM-DD` 头部
- `docs/workflows/release.md` 增加「CHANGELOG entry 风格」章节
- 4 个 GitHub release body 通过 `gh release edit <tag> --notes-file <tmp>` 同步
- 不修改 `.github/release.yml`（label-based 自动生成不被 extract-changelog 使用，留作历史/未来手动 release 选项）

## Acceptance Criteria

- [ ] CHANGELOG.md 4 个版本 body 全部为新风格，无遗漏 PR
- [ ] `node scripts/extract-changelog.mjs 0.4.0` 输出非空且首行匹配 `### 🎉` 或 `### 🛠️`
- [ ] `docs/workflows/release.md` 新增章节展示完整模板 + 5 类分类表
- [ ] 4 个 GitHub release body 与 CHANGELOG 中对应段落 byte-equal（extract-changelog 输出对照）
- [ ] PR 标题：`docs(changelog): adopt terse release note style`

## Definition of Done

- 改动文件：CHANGELOG.md、docs/workflows/release.md
- 远端动作：`gh release edit` × 4
- 不需要 npm/cargo test（pure doc）；只需 `npm run format:check` 通过
- 不发版（pure doc PR）

## Out of Scope (explicit)

- 不动 bump-version.mjs（stub 头部已经是 `## [X.Y.Z] - <date>`，足够）
- 不改 `.github/release.yml`（label-based 自动生成不被实际使用）
- 不为旧 release 重发 tag
- 不引入 conventional-changelog / changesets 等工具

## Technical Notes

- chrome-devtools-mcp 截图分类：🎉 Features / 🛠️ Fixes / 📃 Documentation / 🧪 Refactor
- eye-zen 多一类 🔧 Maintenance 收纳 chore/ci/build/refactor（refactor 单独分时，与 chore 区分）
- 短 SHA：git 默认 7 位
- 提交→PR 映射（验证后）：
  - v0.1.0: 无 PR# (pre-PR flow), 取代表性 commit shas
  - v0.2.0: weekday scheduling 与 main window resize 都在 release branch 通过 PR #4 squash 进入（commit 49be514）；GPL switch + Trellis adoption 在 #1 (bc4aa8f)；release pipeline #2 (6e2e3ce)；cargo-deny #3 (99eaa80)；Cargo.lock hotfix #5 (5b60ab8)
  - v0.3.0: AFK #10 (58332ba), stats #11 (5a68535), hotkeys #12 (bc96fb5), GitHub Flow + bump-version Cargo.lock fix #7 (b15a993)
  - v0.4.0: whitelist #14 (6e0abbc)
- v0.1.0 代表 SHAs:
  - 9d431b5 backend MVP services
  - f56cfeb tip window
  - f75fb5e tray panel
  - 5748b27 SettingsPage
  - a8191a2 AboutPage
  - 244a73b i18n
  - 69bd8b4 theme
  - e1d6f32 autostart
  - cd57866 scaffold init
  - 3d96503 ts-rs bindings
  - b10533b CI matrix
- PR #6 (0dec18f) 历史曲折：merged into 已死的 dev 分支，main 不可达；其内容（bump-version Cargo.lock sync）由 PR #7 的 GitHub Flow 迁移带到 main
