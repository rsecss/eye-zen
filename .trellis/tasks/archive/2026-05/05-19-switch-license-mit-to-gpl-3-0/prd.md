# Switch License: MIT → GPL-3.0-or-later

## Goal

将 Eyezen 项目的开源许可证从 MIT 切换到 GPL-3.0-or-later，以防止商业实体闭源 fork / 重打包分发 Eyezen 的修改版二进制。AGPL-3.0 已评估并排除（桌面 app 不触发其网络条款，徒增企业用户避雷反应）。

## Requirements

- R1: `LICENSE` 替换为 GPL-3.0 官方 verbatim 文本（GNU 源），不附加自定义版权头（GPL 推荐 LICENSE 文件保持原文）
- R2: 所有 license 引用统一为 SPDX 标识符 `GPL-3.0-or-later`
- R3: README badge 更新（中英两版）+ License section 文本更新
- R4: 补全缺失字段：`src-tauri/Cargo.toml` 加 `license = "GPL-3.0-or-later"`、`package.json` 加 `"license": "GPL-3.0-or-later"`
- R5: `src-tauri/tauri.conf.json:44` 改为 `"license": "GPL-3.0-or-later"`
- R6: `CONTRIBUTING.md:314` 一句话改为 inbound=outbound 表述：
  > 提交贡献即表示你同意以 [GPL-3.0-or-later](LICENSE) 许可证发布你的代码（inbound license = outbound license）。
- R7: `CLAUDE.md` 顶部 "开源 (MIT)" 标记改为 "开源 (GPL-3.0-or-later)"
- R8: `docs/devlog.md` 新增 license 切换记录（日期 + 动机 + 排除 AGPL 的理由）
- R9: 不动源文件（不加 SPDX 短头）
- R10: 走 PR 工作流提交（dev → main），不直接 push 到 main

## Acceptance Criteria

- [ ] `LICENSE` 首行匹配 GPL-3.0 标准首行：`                    GNU GENERAL PUBLIC LICENSE`
- [ ] `LICENSE` 文件 SHA-256 与 GNU 官方一致（或字符级 diff 仅在空白处）
- [ ] `grep -ri "MIT License" -- ':!**/node_modules' ':!**/target' ':!**/.git'` 无残留（test fixtures 除外）
- [ ] `grep -ri "license-MIT" .` 无残留
- [ ] `grep -ri "GPL-3.0-or-later" --include='*.json' --include='*.toml' --include='*.md'` 至少命中 8 处
- [ ] 本地预跑全部通过：`cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` / `cargo test` / `npm run format:check` / `npx svelte-check` / `npm test` / `npm run build`
- [ ] commit 信息：`chore(license): switch from MIT to GPL-3.0-or-later`
- [ ] PR 描述清晰说明动机 + 不可逆性 + 受影响位置 + 已排除 AGPL 的理由

## Definition of Done

- 所有 SPDX 标识符一致
- CLAUDE.md / devlog.md 记录变更日期与决策依据
- 完整本地验证（fmt + clippy --all-targets + test + svelte-check + build）
- PR 工作流（不直推 main）
- v0.1.0 制品仍 under MIT 这一事实在 PR 描述与 devlog 中注明（避免下游误解）

## Technical Approach

### 文件改动清单（按依赖排序）

1. **`LICENSE`** — 整文件覆写为 GPL-3.0 verbatim 文本（来源：https://www.gnu.org/licenses/gpl-3.0.txt）
2. **`src-tauri/tauri.conf.json:44`** — `"license": "MIT"` → `"license": "GPL-3.0-or-later"`
3. **`src-tauri/Cargo.toml`** — `[package]` 段下追加 `license = "GPL-3.0-or-later"`
4. **`package.json`** — 顶层追加 `"license": "GPL-3.0-or-later"`
5. **`README.md`** — 第 12 行 badge URL 改 `license-GPL--3.0--or--later-blue.svg`；第 159 行 `MIT License` → `GPL-3.0-or-later`
6. **`README.zh-CN.md`** — 同上（第 12 行 + 第 172 行）
7. **`CONTRIBUTING.md:314`** — 改为 inbound=outbound 表述
8. **`CLAUDE.md` 顶部** — `开源 (MIT)` → `开源 (GPL-3.0-or-later)`
9. **`docs/devlog.md`** — 追加 `2026-05-19 — License 切换 MIT → GPL-3.0-or-later` 段落
10. **`CLAUDE.md` 变更记录段** — 追加同日条目

### SPDX 标识符

- 选定：`GPL-3.0-or-later`
- 理由：FSF 推荐，允许未来 GPL-4 自动适用；与 -only 在 GPL-3.0 时代法律效果完全一致

### 源文件级 SPDX 头

- 决定不加
- 理由：LICENSE 文件已具备完整法律效力；Eyezen 体量小、贡献者单一，避免 ~100 个文件的噪声 diff

### Inbound License 条款

- `CONTRIBUTING.md` 改为 inbound=outbound 一句话表述
- 不引入 CLA、不引入 DCO（Signed-off-by）
- 依赖 GitHub ToS §D.6 默认假设 + 显式声明双重保障

## Decision (ADR-lite)

**Context**
v0.1.0 已发布于 MIT。社区驱动定位下，希望防止商业实体闭源 fork / 重打包分发。需要在 copyleft 强度与社区友好度之间取舍。

**Decision**
切换到 **GPL-3.0-or-later**，配套补全 SPDX 字段、加 inbound=outbound 声明、不动源文件头、不引入 CLA/DCO。

**Consequences**
- ✅ 任何分发的衍生版本必须开源，达成"防闭源 fork"目标
- ✅ 个人/公司自用不受影响（合理边界）
- ✅ 唯一贡献者状态下，重新许可零阻力（窗口期不可错过）
- ⚠️ 未来贡献者池可能缩小（部分公司明文禁止 GPL 入员工提交）
- ⚠️ 不可逆：再回宽松许可需所有未来贡献者同意
- ℹ️ v0.1.0 制品仍永久 under MIT（已发布事实不可追溯）
- ℹ️ Tauri/Svelte 上游 (Apache-2.0/MIT) 与下游 GPL-3.0 合法兼容

## Out of Scope

- 不重写 git 历史
- 不撤销或重新打包 v0.1.0（其 MIT 状态永久保留）
- 不联系下游用户/分支
- 不引入 CLA 工具（CLA Assistant 等）
- 不切换到 AGPL-3.0 / GPL-2.0 / LGPL（已决策排除）
- 不在源文件添加 SPDX 短头

## Technical Notes

- LICENSE 文件来源：https://www.gnu.org/licenses/gpl-3.0.txt（GNU 官方）或 SPDX 镜像
- 文件大小预期：~35KB / ~675 行
- SPDX 规范：https://spdx.dev/learn/handling-license-info/
- "or-later" 选择参考：FSF 历史推荐 https://www.gnu.org/licenses/identify-licenses-clearly.html
- 关键引用位置已在 PRD 头部全列；实施时按"文件改动清单"顺序操作
