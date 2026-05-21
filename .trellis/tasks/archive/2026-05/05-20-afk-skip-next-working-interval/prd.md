# AFK Skip Next Working Interval

## Goal

When the user has no keyboard or mouse input for the configured AFK threshold, Eyezen should reuse the existing `SkipFlags` timer mechanism to skip the next working interval prompt instead of showing the rest alert. This keeps the current timer state machine simple while avoiding reminders after the user has already stepped away.

## What I Already Know

* The existing timer machine uses `SkipFlags` in `Working` timeout handling. When any flag is active, `step_time` returns `Working -> Working`, which triggers `ResetWorkTimer` and avoids `PreAlert`.
* Workday scheduling already uses the same mechanism via `schedule_inactive`.
* Current behavior settings include `sound_enabled`, `fullscreen_skip`, and `auto_start`; new behavior fields must keep old TOML files readable with serde defaults.
* Settings uses immediate-save flow from `configStore.current` to `updateBehaviorConfig`, then waits for `config_changed` before UI updates.
* Linux already detects Wayland for fullscreen and degrades conservatively without panics.

## Assumptions

* AFK means system-level idle time since the most recent keyboard or mouse input, not merely cursor position changes sampled inside the app.
* Default AFK threshold is 5 minutes and AFK skipping is enabled only when the platform reports idle detection support.
* Unsupported platforms or sessions return an unavailable capability and must not skip prompts based on unknown idle state.
* On Linux Wayland, AFK detection is unavailable for this task and the Settings controls are disabled with explanatory copy.

## Requirements

* Add behavior configuration for AFK skipping and threshold minutes, with serde defaults so old TOML files start normally.
* Reuse `SkipFlags` by adding an AFK flag; do not introduce a new `Away` timer state for this task.
* At each Working threshold tick, compute AFK state from current config and platform idle information; if idle duration is at or above threshold, skip the prompt and reset the work timer.
* Log `skip: afk` when AFK skip suppresses the alert.
* If input occurred more recently than the threshold, preserve current behavior and show the normal alert.
* Settings threshold changes must take effect immediately without restarting the app or interrupting the current timer cycle.
* Platform idle detection must degrade safely. On Wayland, the app must not error and the AFK controls must be disabled.
* Frontend types must come from regenerated ts-rs bindings; no hand-written duplicate DTOs.

## Acceptance Criteria

* [x] 5 minutes with no keyboard/mouse input causes the next working alert not to show and logs `skip: afk`.
* [x] Recent keyboard/mouse input below threshold allows the normal alert flow.
* [x] Changing the AFK threshold in Settings affects the next timer check immediately.
* [x] Wayland reports AFK detection unavailable, does not log repeated errors, and Settings controls are disabled.
* [x] Old TOML missing AFK fields loads successfully, covered through the existing `partial_toml_uses_defaults` pattern.
* [x] Rust tests cover threshold boundaries and `SkipFlags` AFK state behavior.
* [x] `npm run ci` passes locally, with CI expected to run the same command on Windows, macOS, and Linux.

## Definition of Done

* Rust unit tests cover config defaults, platform-independent AFK threshold resolution, and timer skip transitions.
* Frontend tests cover Settings AFK control rendering/disabled behavior and update calls where practical.
* ts-rs bindings are regenerated through `cargo test`.
* `npm run ci` completes successfully.
* Trellis check/finish flow is followed before final completion.

## Technical Approach

Use the existing skip pipeline:

* Extend `BehaviorConfig` with AFK fields and defaults.
* Extend `PlatformApi` with an idle detection capability returning `Option<Duration>` or equivalent availability-aware result.
* Extend `DetectorService` with `idle_duration()` / support query wrappers.
* Extend `SkipFlags` with `afk_active`, update `any_active()`, and keep `step_time` pure.
* Compute AFK skip in `current_skip_flags` using the latest config snapshot, so Settings changes are visible on the next tick.
* Surface platform support to Settings through config-derived/platform-derived state with minimal cross-layer API changes.

## Decision (ADR-lite)

**Context:** The state machine spec already defines `SkipFlags` as the suppression mechanism for `Working` timeout prompts. Adding a separate `Away` state would enlarge the behavior surface and conflict with the explicit requirement to reuse `SkipFlags`.

**Decision:** Implement AFK as a new `SkipFlags` condition and keep the timer in `Working` when the user is idle past the threshold.

**Consequences:** The behavior is simple and aligns with workday scheduling. The trade-off is that AFK is evaluated at tick time rather than represented as a long-lived state, so Settings must expose platform support separately if controls need to be disabled.

## Out of Scope

* No new statistics database records for AFK segments.
* No audio, foreground-process, or whitelist based AFK heuristics.
* No Wayland global input workaround or portal integration.
* No new timer state named `Away`.

## Technical Notes

* Current task: `.trellis/tasks/05-20-afk-skip-next-working-interval`.
* Relevant specs: `.trellis/spec/architecture/ipc-and-state.md`, `.trellis/spec/architecture/change-management.md`, `.trellis/spec/architecture/testing-quality.md`, `.trellis/spec/backend/platform-storage.md`, `.trellis/spec/frontend/state-management.md`.
* Key code anchors: `src-tauri/src/services/context.rs`, `src-tauri/src/services/timer/state.rs`, `src-tauri/src/services/timer/machine.rs`, `src-tauri/src/models/config.rs`, `src-tauri/src/platform/*.rs`, `src/pages/settings/SettingsPage.svelte`.
