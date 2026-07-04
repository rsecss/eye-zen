# Eyezen 全维度深度审计计划

**日期**: 2026-07-03  
**基线**: main @ 83940d7 (v0.7.1, 2026-05-25)  
**审计范围**: 9 维度 × 独立 agent  
**先前基线**: docs/.local/v1.0.0-audit-report.md (F01-F29, 2026-05-23 @ v0.6.0)

---

## 执行策略

采用 **Agent-team 并行** 方式,每个维度启动一个独立 agent:

1. 每个 agent 输出到独立文件 `.trellis/workspace/Maple/audit-results/<dimension>.md`
2. 所有 agent 完成后,主会话汇总并生成最终报告
3. 每个 agent 严格只读审计,禁止修改代码/文档/git

---

## 9 个审计维度

### D1: 需求痛点与产品体验 (req)
- **核心问题**: 20-20-20 核心闭环痛点覆盖、首次运行体验、统计 actionable 程度、与竞品对比、打扰谱系、可访问性
- **材料**: README、rebuild design spec、竞品调研、i18n 文案、页面代码
- **输出**: 缺失功能/体验增强建议,按「用户价值 × 成本」分级

### D2: 架构约束与理由链 (arch)
- **定义**: 架构 = 被持续坚持的约束 × 能够被追溯的理由链
- **核心问题**: spec 约束的实际遵守情况、理由链完整性、ServiceContext 耦合、EffectSink 模式一致性、启动顺序保证、前端架构同构性
- **材料**: .trellis/spec/architecture/* + backend/service-pattern + lib.rs + services/context.rs + commands/
- **输出**: 活约束 vs 腐化约束 vs 教条约束清单

### D3: 后端 Rust 质量 (rust)
- **核心问题**: panic 面、异步卫生、阻塞混入 async、unsafe soundness、错误处理、大文件拆分、依赖、数值时间
- **材料**: src-tauri/src/ 全部,重点 services/ 大文件、platform/ FFI、error.rs
- **输出**: P0-P3 findings,每条必须有 file:line 证据

### D4: 前端 Svelte 5 质量 (fe)
- **核心问题**: Runes 卫生、单一数据源、IPC 边界、i18n 对齐、主题三层、组件质量、错误呈现、构建
- **材料**: src/lib/* + src/pages/* + vite.config.ts
- **输出**: P0-P3 findings

### D5: 测试质量与验收 (test)
- **核心问题**: 93% 是否覆盖率剧场?断言质量、行为 vs 实现、关键路径盲区、平台测试策略、mock 契约、验收体系、flaky
- **材料**: vite.config coverage 配置、__tests__/、#[cfg(test)]、CI、testing-quality.md、release.md
- **输出**: 覆盖率排除清单审计 + 测试盲区

### D6: 文档一致性与漂移 (docs)
- **核心问题**: 文档声明 vs 仓库事实逐处核对
- **材料**: CLAUDE.md、README、CHANGELOG、AGENTS、docs/workflows/、docs/plans/、.trellis/spec/ 抽查、.github/
- **输出**: 漂移清单 + 缺失文档(安装/FAQ/卸载)

### D7: 可维护性与熵代谢 (ent)
- **理念**: 代谢率 > 腐朽率
- **核心问题**: 腐朽存量(500 行红线违例、TODO/FIXME、allow/ignore、重复、dead code)、依赖代谢、git 卫生、代谢机制评估、单人维护风险
- **材料**: 全仓、npm outdated、cargo update --dry-run、git log、package.json/Cargo.toml
- **输出**: 腐朽率 vs 代谢率判断 + 机制补强建议

### D8: 安全与边界约束 (sec)
- **核心问题**: capability 最小权限、CSP、command 输入验证(导出路径白名单绕过?)、SQL 注入、unsafe FFI、供应链、敏感信息、分发信任
- **材料**: capabilities/、tauri.conf.json、commands/、stat/export.rs、platform/ unsafe、deny.toml、workflows/、logging.rs
- **输出**: P0-P3 findings,按 pre-1.0 开源标准定级

### D9: CI/CD 与工程流程 (cicd)
- **核心问题**: 三平台矩阵覆盖、缓存、release 流程、本地/云端 parity、hooks、流程缺口、bump-version 质量、v1.0.0 准备
- **材料**: .github/workflows/*、scripts/*、.husky/*、commitlint.config.js、docs/workflows/*
- **输出**: 流程债务 + v1.0.0 缺口

---

## 每个 agent 的输出格式

```markdown
# [维度标题] 审计结果

**审计员**: [dimension-key]  
**审计时间**: 2026-07-03  
**基线**: main @ 83940d7  

## 总体评估 (summary)

[3-6 句话:健康度、最大风险、趋势]

## 优势 (strengths)

- [具体实践 1,文件/机制级]
- [具体实践 2]
- [具体实践 3+]

## Findings

### [id] [title] — [severity]

**Category**: [bug/design/architecture/test/docs/maintainability/ux/security/process]  
**Effort**: [S/M/L]

**Evidence**:  
[file:line 级证据 + 为什么这是问题]

**Recommendation**:  
[具体可执行修复/优化建议]

---

[重复 findings 结构,最多 15 条,P3 不超过 5 条]
```

---

## 审计纪律(所有 agent 必须遵守)

1. **只读**: 严禁修改/创建/删除仓库文件;严禁 git add/commit;Bash 只用于只读命令
2. **亲自取证**: 每条 finding 必须读到代码/文档原文并给出真实 file:line
3. **基线意识**: 熟读 docs/.local/v1.0.0-audit-report.md F01-F29,不要重报已如实修复的项;抽查"已关闭"项的修复质量
4. **宁缺毋滥**: 最多 15 条,优先影响大的,不用琐碎凑数
5. **简体中文**: 全部输出简体中文;代码标识符/路径/命令保持原文

---

## 先前审计 F01-F29 状态速查

v0.7.0 宣称关闭全部 P0/P1 (14 项):
- F01 导出路径白名单 ✅
- F02 migration 事务化 ✅
- F04 IPC 覆盖 ✅
- F05/F13/F14 npm 漏洞 ✅
- F08 有界 channel ✅
- F09-F11 兜底代码 ✅
- F12 覆盖率门禁 ✅
- F21/F23/F26-F27 ✅

v0.7.1 宣称关闭 P2/P3 (10 项):
- F17 stat.rs 拆分 ✅
- F18 大页面拆分 ✅
- F19 locale en canonical ✅
- F20 IPC event 常量化 ✅
- F16 IPC timeout 三档 ✅
- F22 capability emit 收紧 ✅
- F25 docs 漂移 ✅
- F29 平台路径已知限制 ✅
- F03+F28 macOS fullscreen 真实现 ✅

未修/已挪到 v1.0.0/未排期:
- F15 stat fetch 全表扫描
- F06/F07 API 重命名 + Beta 标记移除
- 覆盖率推 95%
- tip-window mini/角落通知模式

**agent 职责**: 抽查"已关闭"项质量(修了但没修好 = 新 finding);重点找该报告没发现的新问题;F15/F06/F07 属已知未修,除非影响比记录更严重,否则不重报。
