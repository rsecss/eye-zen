# UX Decisions for Process Whitelist

Aggregated, codebase-grounded decision record. Replaces the external competitor research that was skipped (the original `whitelist-ux-patterns.md` subagent completed without producing output; the projecteye/blinkeye notes referenced in memory don't exist in this repo).

Source of truth: existing `src/lib/i18n/zh-CN.ts` + `src/lib/i18n/en.ts` for copy style, existing Schedule Card in `src/pages/main/SettingsPage.svelte:495-528` for UI pattern.

---

## List size cap: **32**

- Typical user whitelists 5–10 apps (IDE / browser / chat / meeting tool)
- 32 leaves headroom without becoming a power-user feature
- Lowercase string comparison of 32 entries per timer tick (1 Hz) is <5 µs — negligible
- 32 chips wrap to 3–4 rows in current Settings layout, still browsable
- Stored in TOML as `Vec<String>`; rejection happens at write boundary (Settings UI add validation + `update_behavior_config` command), not at read time

## Matching strategy: **exact match after `trim().to_lowercase()`**

- Compare stored entry and platform-reported name as lowercase, trimmed
- **Rejected**: substring match — too easy to false-match (`"code"` would match `"code helper"`, `"vscode"`, `"barcode.exe"`)
- **Rejected**: regex / glob — over-scoped for MVP
- macOS `kCGWindowOwnerName` returns names like `"Google Chrome"`, `"Code"`, `"Slack"` (CFBundleExecutable-equivalent, may contain spaces). UI placeholder + desc MUST tell users to match the Activity Monitor / Task Manager name exactly (case-insensitive only)

## Self-skip protection

- Add-time validation: reject if candidate (after trim+lowercase) matches `["eyezen", "eyezen.exe"]`
- Show i18n error toast (key `settings.whitelist.error.self`)
- TOML-edited config that contains `"eyezen"` is NOT blocked at load time (worst case: Settings window becomes a whitelist target, which is annoying but recoverable). UI prevention is the practical safeguard.

## Wayland / unsupported degradation

- `get_foreground_process_name()` returns `None` on Wayland (no portable foreground API)
- `DetectorCapabilities` adds `foreground_process_detection_supported: bool`
- Settings card shows i18n-tagged disabled banner (mirrors the existing AFK pattern: `settings.behavior.afkUnsupported`)
- Toggle + add-input disabled when capability is false

## Duplicate handling

- After `trim().to_lowercase()`, if the candidate already exists in list, reject add (i18n key `settings.whitelist.error.duplicate`)
- Stored canonical form is lowercase (we store lowercase to make matching cheaper at runtime AND to prevent visually-confusing dupes like `"Code"` + `"code"`)
- Display in UI as-is (already lowercased), no case preservation

## Empty input

- Trim whitespace; reject empty string with i18n error `settings.whitelist.error.empty`

## i18n keys (zh-CN + en)

| key | zh-CN | en |
|-----|-------|-----|
| `settings.whitelist.title` | 进程白名单 | Process Whitelist |
| `settings.whitelist.enabled` | 启用白名单 | Enable whitelist |
| `settings.whitelist.enabled.desc` | 前台为列表中的进程时跳过提醒 | Skip reminders when a listed process is in front |
| `settings.whitelist.list` | 进程列表 | Processes |
| `settings.whitelist.list.desc` | 任务管理器 / Activity Monitor 显示的名称，大小写不敏感 | Names as shown in Task Manager / Activity Monitor (case-insensitive) |
| `settings.whitelist.add.placeholder` | 例如 code.exe / Google Chrome | e.g. code.exe / Google Chrome |
| `settings.whitelist.add` | 添加 | Add |
| `settings.whitelist.remove` | 移除 | Remove |
| `settings.whitelist.empty` | 暂无白名单进程 | No whitelisted processes |
| `settings.whitelist.error.duplicate` | 已存在 | Already in list |
| `settings.whitelist.error.self` | 不能将 Eyezen 自身加入白名单 | Cannot whitelist Eyezen itself |
| `settings.whitelist.error.empty` | 名称不能为空 | Name cannot be empty |
| `settings.whitelist.error.limit` | 最多 {max} 项 | Up to {max} entries |
| `settings.whitelist.unsupported` | 当前会话不支持前台进程检测，已禁用 | Foreground process detection unavailable in this session |

Style matches existing `settings.schedule.*` and `settings.behavior.afk*` keys: short labels, one-sentence desc, fixed Latin words for UI affordance.

## Edge cases worth testing

- Add `"code.exe"` on Windows → matches Windows `code.exe`; does NOT match macOS `"Code"` (acceptable, cross-platform same TOML is not a goal)
- Add `"Google Chrome"` (with space) → stored as `"google chrome"`, matches macOS `kCGWindowOwnerName == "Google Chrome"` after normalize
- TOML manually edited to contain mixed-case entries → load normalizes on first config_changed emit (or re-normalize on read) — implementation decision deferred to PR
- List length exceeds 32 due to TOML edit → load silently truncates to 32 with a `tracing::warn!`
- Schema migration: old `config.toml` lacks `process_whitelist` + `process_whitelist_enabled` → `#[serde(default)]` fills with empty vec + false (no functional change)
