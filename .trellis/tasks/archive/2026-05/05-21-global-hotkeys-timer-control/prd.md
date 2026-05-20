# Global Hotkeys Timer Control

## Goal

Add global keyboard shortcuts so Eyezen can control timer behavior without focusing the app: manually trigger a rest, skip the current alert/rest flow, and pause or resume the timer. Shortcuts must be configurable in Settings, apply immediately, degrade safely on unsupported or permission-limited systems, and be covered by CI-relevant tests.

## Requirements

- Provide default global shortcuts that work after first launch with no Settings changes.
- Map shortcut actions to existing timer behavior:
  - `StartRest`: trigger manual rest via the existing `start_rest` timer event.
  - `SkipRest`: skip the current alert/rest via the existing `skip_rest` timer event.
  - `TogglePause`: pause when running, resume when paused, using existing timer state.
- Persist shortcut configuration in TOML under a new config section with backwards-compatible defaults for old config files.
- Update Settings with editable shortcut fields and a clear status message for each shortcut.
- Apply Settings shortcut changes immediately after config save.
- On shortcut update, unregister the old binding before registering the new one; if registration fails, roll back to the previous binding and surface the failure to Settings.
- If a shortcut conflicts with another app or cannot be registered, show a clear UI warning and keep the app running.
- On macOS, missing Accessibility permission must not crash the app. Settings must guide the user to grant permission.
- Avoid exposing generic global-shortcut registration to the frontend; backend owns registration and action dispatch.

## Acceptance Criteria

- [x] Default shortcuts are present in `Config::default()` and trigger the corresponding timer behavior.
- [x] Editing shortcuts in Settings takes effect immediately without app restart.
- [x] Failed registration rolls back to the previous working binding.
- [x] Conflict/registration failure appears clearly in Settings and is not only logged.
- [x] macOS Accessibility permission missing is represented as degraded status with Settings guidance, not a panic.
- [x] Old TOML files without shortcut fields load successfully using defaults, covering the existing `partial_toml_uses_defaults` pattern.
- [x] `npm run ci` passes locally.
- [x] Rust tests cover default config, TOML defaulting, shortcut conflict/rollback logic, and shortcut action enum serialization.

## Definition of Done

- Rust formatting, frontend formatting/check/build/test, and Rust tests pass through `npm run ci`.
- Generated TypeScript bindings are updated if Rust models crossing IPC change.
- User-facing Settings copy is clear for normal, conflict, and macOS permission-degraded states.
- Implementation keeps timer control behavior in existing timer service APIs rather than duplicating timer state transitions.

## Technical Approach

Use the official Tauri v2 global-shortcut plugin on the backend. Add a `HotkeyService` that registers current config shortcuts at startup, subscribes to config changes, updates bindings transactionally, and dispatches timer events on `ShortcutState::Pressed`.

Expose status to Settings through a narrow backend command/event pair, for example `get_hotkey_status` and `hotkey-status-changed`, rather than exposing plugin registration APIs. Settings continues to save through config commands; the backend service handles binding updates after config changes.

Default shortcuts, unless contradicted by repo constraints during implementation:

- Manual rest: `CommandOrControl+Alt+B`
- Skip current alert/rest: `CommandOrControl+Alt+S`
- Pause/resume timer: `CommandOrControl+Alt+P`

## Decision (ADR-lite)

Context: Global shortcuts can be registered from frontend JavaScript or Rust backend. This app already centralizes timer behavior in Rust services and keeps frontend IPC narrow.

Decision: Use backend-owned shortcut registration and dispatch. Frontend Settings edits config only and reads backend status.

Consequences: The frontend cannot accidentally register arbitrary global shortcuts; rollback and conflict handling stay testable in Rust. The backend must expose a small status model and event for Settings.

## Out of Scope

- Per-window local keyboard shortcuts.
- Chord recording UI that captures arbitrary keydown sequences automatically.
- User-defined actions beyond manual rest, skip rest, and pause/resume.
- Mobile platform support.

## Technical Notes

- Research: `research/global-shortcut-plugin.md`.
- Relevant existing modules discovered:
  - `src-tauri/src/services/timer/` for timer state and user events.
  - `src-tauri/src/services/config.rs` for TOML load/save and watch updates.
  - `src-tauri/src/models/config.rs` for config defaults and tests.
  - `src/pages/settings/` and `src/lib/stores/config.svelte.ts` for Settings data flow.
  - `src/lib/commands.ts` and `src/lib/events.ts` for frontend IPC wrappers.
- The Tauri docs state plugin JavaScript permissions are blocked by default; this task avoids adding broad frontend registration permission.
