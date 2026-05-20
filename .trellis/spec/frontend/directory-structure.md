# 目录结构

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

---

## Vite 多入口布局

Eyezen 是纯 Svelte 5 + Vite 应用，每个 Tauri 窗口对应一个 HTML 入口。结构如下：

```
(项目根目录)
├── index.html           → main-window
├── tip.html             → tip-window
├── tip-minimal.html     → tip-window-minimal
└── tray.html            → tray-panel

src/
├── entries/             各窗口 TS 入口（仅做 mount）
│   ├── main.ts
│   ├── tip.ts
│   ├── tip-minimal.ts
│   └── tray.ts
├── pages/               各窗口页面组件
│   ├── main/
│   │   ├── MainApp.svelte
│   │   ├── SettingsPage.svelte
│   │   ├── AboutPage.svelte
│   │   └── components/      窗口级可复用组件
│   │       ├── Stepper.svelte
│   │       ├── Toggle.svelte
│   │       ├── Select.svelte
│   │       └── SettingsCard.svelte
│   ├── tip/TipApp.svelte
│   ├── tip-minimal/TipMinimalApp.svelte
│   └── tray/TrayApp.svelte
├── lib/
│   ├── bindings/        ts-rs 生成类型（MUST NOT 手动编辑）
│   ├── components/      跨窗口共享组件（当前为空，按需提升）
│   ├── stores/          Svelte stores（`.svelte.ts` 后缀）
│   ├── i18n/            语言字典 + i18nStore
│   ├── commands.ts      IPC command 封装
│   └── events.ts        IPC event 封装
└── app.css              TailwindCSS v4 入口 + 全局 CSS 变量
```

- HTML 入口 MUST 放在项目根目录，与 `vite.config.ts` 的 `rollupOptions.input` 一一对应。
- `src/lib/bindings/` MUST 由 `cargo test` 触发 ts-rs 生成覆盖，MUST NOT 手动编辑或被 prettier 重排（已加入 `.prettierignore`）。
- TS 入口（`src/entries/*.ts`）MUST 只承担 `mount(App, { target })`，MUST NOT 引入业务逻辑。参考 `src/entries/main.ts`：

```ts
import MainApp from '../pages/main/MainApp.svelte';
import { mount } from 'svelte';

const app = mount(MainApp, { target: document.getElementById('app')! });
export default app;
```

## 窗口与入口的对应关系

| 窗口 label | HTML | TS entry | Page 根组件 | Capability |
|-----------|------|----------|------------|-----------|
| `main-window` | `index.html` | `src/entries/main.ts` | `src/pages/main/MainApp.svelte` | `src-tauri/capabilities/main-window.json` |
| `tip-window-{n}` | `tip.html` | `src/entries/tip.ts` | `src/pages/tip/TipApp.svelte` | `src-tauri/capabilities/tip-window.json` |
| `tip-window-minimal-{n}` | `tip-minimal.html` | `src/entries/tip-minimal.ts` | `src/pages/tip-minimal/TipMinimalApp.svelte` | `src-tauri/capabilities/tip-window.json` |
| `tray-panel` | `tray.html` | `src/entries/tray.ts` | `src/pages/tray/TrayApp.svelte` | `src-tauri/capabilities/tray-panel.json` |

- 动态窗口（tip 系列）MUST 使用 `{n}` 后缀表示显示器编号；创建/销毁由后端 `WindowService` 管理。
- 动态窗口前端 MUST NOT 假设单例；如需窗口实例，使用 `getCurrentWindow()`（参考 `MainApp.svelte` 调用 `setTheme()` 的方式）。

## 新增窗口流程

新增窗口 MUST 同步完成以下五步，缺一不可：

1. 项目根目录添加 `<name>.html`
2. `src/entries/<name>.ts` 仅做 `mount(App, { target })`
3. `src/pages/<name>/` 下放根组件
4. 在 `vite.config.ts` 的 `build.rollupOptions.input` 注册（见现有 4 个条目）
5. 在 `src-tauri/capabilities/` 添加对应 JSON，引用最小化 permission 集合

完整影响清单（包括 backend 侧 `WindowService` 注册、tauri.conf.json 更新）见 `quality-guidelines.md`。

## 组件放置原则（就近优先，按需提升）

| 位置 | 适用场景 | 当前示例 |
|------|---------|---------|
| `src/pages/<window>/components/` | 仅该窗口使用、有独立交互逻辑的组件 | `Stepper.svelte`、`Toggle.svelte`、`Select.svelte`、`SettingsCard.svelte` |
| `src/lib/components/` | 真实跨 ≥2 窗口使用的组件 | 当前为空（按需提升） |

- 组件 MUST 先放在 `pages/<window>/components/`（就近原则、避免过早抽象）。
- 当两个以上窗口需要同一组件时，才提升到 `lib/components/`。
- MUST NOT 预设"可能被复用"而提前提升到 `lib/`（YAGNI）。
- 纯布局排列（如 `setting-row`）MUST NOT 抽组件，用 CSS 类即可（见 `SettingsPage.svelte` 的 `.setting-row` 模式）。

## 命名规范（前端部分）

| 对象 | 规范 | 示例 |
|------|------|------|
| Svelte 组件文件 | `PascalCase.svelte` | `MainApp.svelte`、`SettingsCard.svelte` |
| Svelte 组件 Prop | `camelCase` | `value`、`timerState` |
| Store 文件 | `kebab-case.svelte.ts` | `config.svelte.ts`、`timer.svelte.ts` |
| 其他 TS 文件（`lib/`） | `kebab-case.ts` | `commands.ts`、`events.ts` |
| TS 入口（`entries/`） | `kebab-case.ts` | `tip-minimal.ts` |
| HTML 入口 | `kebab-case.html` | `tip-minimal.html`、`tray.html` |
| CSS 变量 | `--kebab-case` | `--bg-primary`、`--accent-deep` |
