# 类型安全

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

---

## 跨边界类型来源：`$lib/bindings/`

Rust 后端通过 ts-rs 把 IPC DTO 导出为 TypeScript 类型，文件落在 `src/lib/bindings/`：

```
src/lib/bindings/
├── Config.ts
├── TimerConfig.ts
├── BehaviorConfig.ts
├── DisplayConfig.ts
└── StatePayload.ts
```

- 这些文件 MUST 由 `cargo test` 触发 ts-rs 重新生成；MUST NOT 手动编辑。
- 这些文件已加入 `.prettierignore`，MUST NOT 被 `npm run format` 改写（ts-rs 与 prettier 的输出风格不一致，反复对冲会脏 diff）。
- ts-rs 版本升级（如 10.1）可能引入分隔符/换行差异，升级 MUST 在同一 PR 内重新生成并提交。
- 前端 MUST 从 `$lib/bindings/<TypeName>` 导入类型，路径别名 `$lib` 由 `vite.config.ts` 中的 `resolve.alias` 提供。

```ts
import type { Config } from '$lib/bindings/Config';
import type { StatePayload } from '$lib/bindings/StatePayload';
```

参考 `src/lib/commands.ts` 与 `src/lib/stores/config.svelte.ts` 的导入模式。

## 禁止手写跨边界类型

- MUST NOT 在前端手写与后端重复的 DTO 接口/类型（如 `interface TimerConfig { ... }`）。
- 后端类型变更 MUST 走 `#[derive(TS)]` + `cargo test` 流程；前端 import 自动同步。
- 新增 command 的请求/响应类型 MUST 在 backend 的 `models/` 下定义并加 `#[derive(TS)]`、附 ts-rs 导出测试（参考 backend spec 的"新增 Command"清单）。

## 跨边界数值类型

- 跨 Rust/TS 边界的 DTO 字段 SHOULD 使用 `u32` 替代 `u64`，避免 ts-rs 生成 `bigint`（TS `bigint` 不能与普通 `number` 直接运算，组件层将不得不到处转换）。
- 纯后端字段（时间戳、数据库主键、平台 API 句柄）MAY 使用 `u64`/`i64`，但 MUST NOT 暴露到前端 DTO。
- 现存示例：`StatePayload.remaining_secs`、`TimerConfig.work_minutes` 等均为 `u32` 导出。

## 回调 Prop 与函数签名

- 回调 prop 的类型 MUST 精确：参数类型 + 返回类型完整声明。
- MUST NOT 使用 `any`，包括 `(v: any) => void`。
- MUST NOT 使用 `unknown` 当作万能逃逸，除非真的不可知（多数情况下 ts-rs 类型已经够用）。

```ts
// 正确（Stepper.svelte）
onchange: (v: number) => void;

// 正确（Toggle.svelte）
onchange: (v: boolean) => void;

// 错误
onchange: (v: any) => void;
onchange: Function;
```

## 类型导入与 `import type`

- 类型导入 MUST 使用 `import type { ... }` 语法（保证 erased import，对 tree-shaking 友好）。
- 同时导入运行时值和类型时 SHOULD 分两个 import 语句，避免 `import { type X, fn }` 混用导致工具链处理不一致。

```ts
import type { TimerConfig } from '$lib/bindings/TimerConfig';
import { updateTimerConfig } from '$lib/commands';
```

## 类型断言

- MUST 优先用类型守卫或 `satisfies` 替代 `as`。
- `as` 仅允许用于：DOM 事件目标缩窄（如 `e.currentTarget as HTMLInputElement`，当 Svelte 类型推断不足时）、跨边界 union 收窄到具体分支（且 MUST 同时加 runtime 检查）。
- MUST NOT 使用 `as any` / `as unknown as T` 双断言。

## DOM 事件

- Svelte 5 模板里的事件回调参数有内置类型推断；MUST 优先依赖推断，少写显式注解。参考 `Select.svelte`：

```svelte
<select onchange={(e) => onchange(e.currentTarget.value)}>
```

`e.currentTarget` 在 `<select onchange>` 上推断为 `HTMLSelectElement`，`.value` 是 `string`，无需断言。

## Event Payload 类型

- `events.ts` 中每个 `onXxx` 封装 MUST 给 `listen<T>` 显式提供 Payload 类型（来自 bindings）：

```ts
export function onStateChanged(callback: (payload: StatePayload) => void): Promise<UnlistenFn> {
  return listen<StatePayload>('state_changed', (e) => callback(e.payload));
}
```

- 自定义内部前端事件（如 `navigate_tab` 的 `string`）SHOULD 也显式标注泛型。

## i18n 翻译 Key 的类型

- `i18nStore.t(key)` 的 `key` 参数 MUST 是 `TranslationKey` 联合类型（来自 `src/lib/i18n/zh-CN.ts` 导出的 `keyof TranslationDict`），编译期就能拒绝拼写错误。
- 新增翻译 key MUST 同时更新 `zh-CN.ts` 与 `en.ts`，缺一不可（参考 `i18n.test.ts` 的对齐测试）。

## Lint 与 svelte-check

- `npx svelte-check --tsconfig ./tsconfig.json` MUST 零错误零警告才能合入（详见 `quality-guidelines.md`）。
- 任何 `@ts-ignore` / `@ts-expect-error` MUST 附加一行注释说明原因，并尽快替换为类型守卫。

## 常见错误

| 错误 | 修复 |
|------|------|
| 手写 `interface Config { ... }` 与后端重复 | 删除，改用 `import type { Config } from '$lib/bindings/Config'` |
| `value: any` / `onchange: Function` | 用具体标量类型 + 精确函数签名 |
| `payload as StatePayload` 双断言 | 改用 `listen<StatePayload>(...)` |
| 编辑 `src/lib/bindings/*.ts` 加字段 | 回到 Rust 端 `#[derive(TS)]` 结构体，重跑 `cargo test` |
| 前端用 `bigint` 处理时间字段 | 后端把对应字段改为 `u32`，重新生成 binding |
