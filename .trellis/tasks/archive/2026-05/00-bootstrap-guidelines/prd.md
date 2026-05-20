# Bootstrap Task：填充项目开发规范并迁移历史工作流

**本任务由 AI 主导执行，开发者不读此文件。**

## 背景

项目状态（2026-05-19，dev 分支）：
- v0.1.0 已发布；技术栈 Tauri v2 + Svelte 5 (Runes) + Vite 6 + TailwindCSS v4
- 已有高质量规则源：`rules/01-07-*.md` 共 7 篇 + `rules/README.md`
- `CLAUDE.md` 含完整项目总览与对 `rules/` 的多处引用
- `docs/workflows/` 含 PR / release / dev / update-docs / release-naming 流程文档
- **"Superpower 工作流" 实际残留极少**：仅 `.gitignore:29,33` 两条目录忽略条目（`docs/superpowers/` 与 `.superpowers/`，实际目录不存在），全库 grep 无其他命中

## 范围（用户已确认）

1. **重塑 spec 布局**：默认模板只有 `frontend/`，但项目是 Tauri Rust 后端 + Svelte 前端双层架构。改为：
   - `.trellis/spec/backend/` —— Rust 服务/IPC/编码/平台/错误日志
   - `.trellis/spec/frontend/` —— Svelte 5 Runes/多窗口/store/IPC/视觉
   - `.trellis/spec/architecture/` —— 分层 DAG / 状态机 / 变更管理 / 测试质量
   - `.trellis/spec/guides/` —— 保留原有思维指南（已填充）
2. **迁移源**：以 `rules/` 7 篇为主，采样真实源码（`src-tauri/src/services/`、`src/lib/stores/`、`src/lib/bindings/` 等）补充示例与路径锚点。规则中文风格与 MUST/SHOULD/MAY 关键词约定保留。
3. **删除 rules/**：迁移完成后整体删除 `rules/` 目录，`CLAUDE.md` 中所有 `rules/XX-*.md` 引用改指 `.trellis/spec/{backend,frontend,architecture}/`。保留 `docs/workflows/`（与 trellis workflow 互补，记录发版 9 步流程 + CI 踩坑）。
4. **清理 superpower 残留**：删除 `.gitignore` 第 29 行（`docs/superpowers/`）和第 33 行（`.superpowers/`）。全库 grep 复检。
5. **Codex MCP review**：spec 写完且本地自检通过后，提交 `mcp__codex__codex (sandbox: read-only)` 做一次综合 review；据反馈迭代。

## Status（动态更新）

- [x] 创建 `.trellis/spec/backend/` 和 `.trellis/spec/architecture/` 目录
- [x] backend/ spec（5 文件）：index, service-pattern, coding-standards, platform-storage, error-and-logging
- [x] architecture/ spec（5 文件）：index, layering, ipc-and-state, change-management, testing-quality
- [x] frontend/ spec（重写 7 文件）：index, directory-structure, component-guidelines, store-and-ipc-patterns（替代旧 hook-guidelines）, state-management, type-safety, quality-guidelines
- [x] 删除 `rules/` 目录
- [x] 清理 `.gitignore` 第 29、33 行 superpower 条目
- [x] 更新 `CLAUDE.md` 中对 `rules/` 的全部 active 引用 → `.trellis/spec/`
- [x] 更新 `CONTRIBUTING.md` / `docs/workflows/{dev,release,pr,update-docs}.md` / `.claude/index.json` 中 rules/ 引用
- [x] CLAUDE.md 顶部 changelog 新增 2026-05-19 迁移条目
- [x] 全库 grep 验证（仅 CLAUDE.md changelog 历史条目 + 本 PRD 内保留 rules/ 字样，均为预期）
- [x] Codex MCP 只读 review（codex 因本地 `.codex/config.toml:32` FeatureToml 解析错误失败，已 fallback 至 general-purpose subagent 完成等价 only-read review）
- [x] 根据 review 反馈迭代：修复 `backend/platform-storage.md:98` 残缺 markdown 链接、补全 `task.json` `relatedFiles`

## 完成判据

- `.trellis/spec/` 描述当前实际代码状态（非理想态、非未来设计）
- 每个 spec 文件至少包含 2-3 个真实文件路径示例（来自 `src-tauri/src/` 或 `src/`）
- 没有模板占位符（`(To be filled by the team)`）/ 空 heading / 复制残留
- `index.md` 与最终 spec 文件集合一致
- 全库无 `rules/` 引用、无 `superpower(s)?` 痕迹
- Codex review 报告附在任务 research/ 下，反馈已消化

## 完成后

```bash
python ./.trellis/scripts/task.py finish
python ./.trellis/scripts/task.py archive 00-bootstrap-guidelines
```
