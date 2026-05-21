# Tauri Global Shortcut Plugin Research

## Sources

- Tauri v2 Global Shortcut plugin docs: https://v2.tauri.app/plugin/global-shortcut/
- docs.rs `tauri-plugin-global-shortcut` 2.3.1 API: https://docs.rs/tauri-plugin-global-shortcut/latest/tauri_plugin_global_shortcut/

## Findings

- The official Tauri global-shortcut plugin supports Windows, Linux, and macOS desktop targets.
- Tauri docs show Rust-side setup through `tauri_plugin_global_shortcut::Builder::new().build()` and event handling with `ShortcutState::Pressed`.
- The Rust API exposes runtime `register`, `on_shortcut`, `on_shortcuts`, `unregister`, `unregister_multiple`, `unregister_all`, and `is_registered`.
- `is_registered` only reports shortcuts registered by this application; it does not prove another app owns the same shortcut.
- Registration methods return `Result`, so app startup and settings updates must surface registration errors instead of silently ignoring them.
- JavaScript plugin commands are blocked by default unless capabilities grant plugin permissions. For this task, backend-owned registration is preferred to avoid exposing generic global-shortcut registration to the webview.

## Repo Mapping

- Existing timer actions already exist as backend commands and service events: `start_rest`, `skip_rest`, `pause_timer`, `resume_timer`.
- Existing config service already supports TOML load/save, defaults, and watch-based hot updates.
- This task should add a backend hotkey service that subscribes to config changes, manages shortcut bindings, and dispatches timer `UserEvent`s.
