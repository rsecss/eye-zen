# 质量门禁

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

---

## 强制门禁（合入前必过）

合入前 MUST 在本地与 CI 同时通过下列命令；任一报错都禁止合入。

```bash
npx svelte-check --tsconfig ./tsconfig.json   # TS + Svelte 类型零错误零警告
npm run format:check                          # Prettier 格式
npm test -- --run                             # Vitest（jsdom + svelteTesting）
npm run build                                 # Vite 多入口构建
```

- `svelte-check` MUST 零错误，warning 数 MUST NOT 退步。
- `format:check` 失败时 MUST 跑 `npm run format` 修复；MUST NOT 改 prettier 配置去迁就个别文件。
- `src/lib/bindings/` 已在 `.prettierignore` 中，不参与格式化。

## 测试要求

测试栈：Vitest 3.x + jsdom + `@testing-library/svelte`（配置见 `vite.config.ts` 的 `test` 段、`src/test-setup.ts`）。

### 组件测试维度

可复用组件（`pages/<window>/components/`）MUST 写组件测试，覆盖至少三类断言：

| 维度 | 要点 | 现有示例 |
|------|------|---------|
| 渲染 | 关键 DOM/ARIA 属性是否正确生成 | `Stepper.test.ts`、`Toggle.test.ts` |
| 交互 | 点击/键盘事件触发 callback、传出值与边界 clamp | `Stepper.test.ts` 的 ± 边界 |
| 边界值 | `value === min`/`max` 时按钮 disabled、空 options 等 | `Stepper.test.ts`、`Select.test.ts` |

### Page 组件

- Page 组件 SHOULD 写关键交互路径测试（如"改 stepper → 调用 `updateTimerConfig`"），mock `commands.ts` 即可。
- 复杂 Page（>200 行）SHOULD 拆分后再补测试，减少 mock 表面积。

### Store 测试

- Stores MUST 覆盖 init/destroy、event-before-snapshot、snapshot-before-event 三种路径。参考 `src/lib/stores/__tests__/timer.test.ts`。
- 测试 MUST mock `commands.ts` / `events.ts` 而非 `@tauri-apps/api`，保持封装层职责。

### i18n 测试

- 新增翻译 key MUST 同时更新 `zh-CN.ts` 与 `en.ts`；`src/lib/i18n/__tests__/i18n.test.ts` 已校验 key 对齐，CI 会拦截缺漏。

## 必用 / 禁用模式

### MUST

- CSS 颜色 / 圆角 / 阴影 / 过渡 MUST 使用 `src/app.css` 中的 CSS 变量（完整变量清单见 `component-guidelines.md`）。
- 前端类型 MUST 从 `$lib/bindings/` 导入；MUST NOT 手写跨边界类型。
- IPC 调用 MUST 经 `commands.ts` / `events.ts` 封装；MUST NOT 在业务组件里直接 `invoke()` / `listen()`。
- Stores MUST 在 `onMount` 内 `init`，在 cleanup 中 `destroy`。
- 控件回调 MUST 用 `on` 前缀 callback prop；MUST NOT `bind:` 双向绑定。

### MUST NOT

- MUST NOT 硬编码色值（`#22c55e`、`rgba(0,0,0,0.5)` 等）；用 `--accent`、`--shadow-card` 等。
- MUST NOT 在前端缓存后端状态副本并自行修改。
- MUST NOT 做乐观更新；以后端事件回推为准。
- MUST NOT 使用 `bind:` 父子双向绑定、`createEventDispatcher()`、`$$props` / `$$restProps`。
- MUST NOT 使用 `dangerouslySetInnerHTML` 或动态注入未转义 HTML。
- MUST NOT 编辑 `src/lib/bindings/` 下的任何文件。

## 安全基线

### CSP 与远程内容

- `src-tauri/tauri.conf.json` 的 CSP MUST 禁止加载远程脚本和样式。
- 前端 MUST NOT 从网络加载任何可执行内容（JS、WASM）。
- 字体等静态资源 MUST 走本地（参考 `src/app.css` 中 `Plus Jakarta Sans` 的 `url('./assets/fonts/...')` 引用）。

### Capability 与 Permission 最小化

- 每个窗口的 capability JSON MUST 只引用所需的最小 permission 集合（参考 `src-tauri/capabilities/main-window.json` / `tray-panel.json` / `tip-window.json`）。
- `src-tauri/capabilities/default.json` 的 `permissions` MUST 保持空数组（共享窗口 baseline，不暴露任何 IPC）。
- MUST NOT 在 default 中加 `core:default` 之外的通配权限。
- 每个自定义 command MUST 有对应的 permission 定义（由后端在 `src-tauri/permissions/` 或 ts-rs 自动生成下落地）；前端窗口 capability 才能引用 `allow-xxx`。
- `core:event:allow-listen` MUST 用对象形式列出具体 event 白名单（见 `main-window.json` 中的 `config_changed` / `state_changed` / `navigate_tab`）；MUST NOT 用空 allow 数组放行所有事件。
- `shell:default` 的覆盖范围是 `http(s)://`、`tel:`、`mailto:`；MUST NOT 启用 `shell:allow-execute`。
- 引入新 Tauri plugin（如 `tauri-plugin-autostart`）MUST 评估其权限范围，只引用所需 `<plugin>:allow-<action>`，并在 PR 描述中说明。

### 敏感数据

- 前端 MUST NOT 持有 API key、token、密码等敏感数据。
- 日志（`console.error` 等）MUST NOT 输出完整文件路径中的用户名。

## 新增前端页面/组件清单

参照 [`.trellis/spec/architecture/change-management.md`](../architecture/change-management.md) 的"新增前端页面/组件"小节，新增 MUST 同时完成：

- [ ] 组件放在对应 `pages/<window>/components/` 或 `lib/components/` 目录
- [ ] 类型从 `$lib/bindings/` 导入
- [ ] Props 使用 `$props()` + callback 模式，MUST NOT 使用 `bind:`
- [ ] 超过 200 行时按职责评估是否需要拆分
- [ ] 可复用组件 MUST 写组件测试（渲染 + 交互 + 边界值）
- [ ] 页面组件 SHOULD 写关键交互路径测试
- [ ] CSS 颜色 MUST 使用 `app.css` 中的 CSS 变量，MUST NOT 硬编码色值
- [ ] 确认 `npm run build` + `svelte-check` + `npm test` 全部通过

## 新增窗口清单

新增窗口在前端侧 MUST 完成（backend 侧另见 backend spec）：

- [ ] 根目录新增 `<name>.html`
- [ ] `src/entries/<name>.ts` 仅 mount 根组件
- [ ] `src/pages/<name>/<Name>App.svelte` 根组件
- [ ] `vite.config.ts` 的 `build.rollupOptions.input` 注册新条目
- [ ] `src-tauri/capabilities/<name>.json` 配最小 permission 集
- [ ] `src-tauri/tauri.conf.json` 中窗口 label MUST 与 capability `windows` 数组一致
- [ ] 如需共享 store，确认 store init/destroy 在新窗口根组件接入

## 新增 Command / Event 清单（前端侧）

完整跨端清单见 [`.trellis/spec/architecture/change-management.md`](../architecture/change-management.md)。前端侧 MUST：

- [ ] 在 `src/lib/commands.ts` 添加封装（`invokeWithTimeout` 包裹）
- [ ] 在 `src/lib/events.ts` 添加 `onXxx(cb)` 封装（`listen<T>` 显式泛型）
- [ ] 在涉及到的窗口 capability JSON 中加 `allow-<command>` / `core:event:allow-listen` 白名单
- [ ] 跑 `cargo test` 让 ts-rs 重新生成 `$lib/bindings/`，并确认 `npm run build` 通过

## CI 关联

CI（`.github/workflows/ci.yml`，三平台矩阵）已配置上述命令；本地预提交（husky pre-push）会跑完整 7 步检查。MUST NOT 用 `--no-verify` 绕过 hook，除非用户明确要求。
