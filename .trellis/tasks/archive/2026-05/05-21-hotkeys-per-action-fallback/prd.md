# Hotkeys per-action fallback

## Goal

Fix the "all-or-nothing" failure mode in the global hotkey service so a single shortcut conflict no longer disables the other working shortcuts. Each shortcut should succeed or fail independently, with the previous working binding preserved on per-action failure and a clear per-binding status in Settings.

## What I already know

- Current `apply_config` in `src-tauri/src/services/hotkeys.rs:110-168` is fully transactional: any single `registry.register` failure rolls back the entire update to the previous bindings via `rollback_failed_update`.
- At startup, previous bindings are empty (`ActiveBindings::default()`), so a single conflict (e.g. `Ctrl+Alt+S` taken by QQ/WeChat screenshot) causes all three default hotkeys to remain unregistered. Settings then shows "绑定失败" on every row plus a misleading top banner.
- The original feature PRD (`.trellis/tasks/archive/2026-05/05-21-global-hotkeys-timer-control/prd.md`) says "Failed registration rolls back to the previous **working** binding" — singular, so the intent was per-action, not all-or-nothing.
- 5 existing tests cover the transactional path: `default_config_registers_all_actions`, `update_unregisters_old_bindings_before_registering_new_ones`, `registration_failure_rolls_back_previous_bindings`, `duplicate_shortcuts_are_rejected`, `action_lookup_uses_registered_shortcut_id`.
- macOS Accessibility Missing path is a separate whole-service skip + permission alert and stays unchanged.
- `HotkeyStatus.last_error` is currently set on any apply_config failure and surfaces the Settings top banner ("快捷键未生效").
- Frontend `SettingsPage.svelte:299-311` renders the top banner from `hotkeyLastError` and per-row status from each `HotkeyBindingStatus.state`.

## Requirements

- `apply_config` registers each action independently; one failure must not affect the others.
- On per-action register failure, restore that action's previous shortcut (best-effort) and emit `HotkeyBindingStatus { state: Conflict, message: <registry error> }` for that action only.
- Mixed result (some `Registered`, some `Conflict`) is a valid steady state. `last_error` MUST be `None` in this case so the misleading top banner is suppressed.
- `last_error` is reserved for "global" failures only:
  - macOS Accessibility Missing (whole-service skip), or
  - All desired bindings fail (the per-binding row labels alone would not be enough; user sees a fully degraded service).
- Duplicate shortcuts within the desired set MUST still be rejected before any registry call (`bindings_from_config` invariant). Duplicate errors keep all previous bindings intact (no partial unregister).
- macOS Accessibility Missing path remains a whole-service skip (no per-action attempt).
- No `HotkeyStatus` / `HotkeyBindingStatus` shape change — only state semantics. ts-rs bindings stay frozen.
- No frontend logic change expected. Top banner naturally disappears in mixed-conflict cases via `last_error == None`.

## Acceptance Criteria

- [ ] Starting Eyezen on a system where exactly one default shortcut conflicts results in: the conflicting action shows `Conflict`, the other two show `Registered` and actually trigger their timer actions when pressed.
- [ ] Settings top "快捷键未生效" banner does NOT appear in the partial-conflict case.
- [ ] Settings top banner DOES appear when (a) macOS Accessibility is Missing or (b) all three bindings fail registration.
- [ ] Editing a `Conflict` shortcut in Settings to a free combination updates only that row to `Registered`; the other rows stay `Registered` without re-registration churn.
- [ ] Editing a working shortcut to a duplicate of another action's shortcut still returns the "duplicates another hotkey action" error AND keeps all bindings unchanged.
- [ ] `npm run ci` passes locally.
- [ ] Existing test `registration_failure_rolls_back_previous_bindings` is updated to reflect per-action rollback semantics (failure on action B does not undo action A's successful registration).
- [ ] New test asserts mixed status: 2 Registered + 1 Conflict, `last_error == None`.
- [ ] New test asserts "all fail" sets `last_error` so the global banner still surfaces.
- [ ] New test asserts editing the Conflict action to a valid shortcut transitions only that action to Registered without touching the other actions' active bindings.

## Definition of Done

- Rust fmt + clippy `--all-targets` + cargo test green
- Frontend svelte-check + vitest + prettier + vite build green (all via `npm run ci`)
- Manual E2E re-confirmed on user's Windows machine: 3 hotkeys verified with real keyboard, including at least one default conflict scenario
- ts-rs bindings regenerated only if `HotkeyStatus` shape changed (expected: NO change)
- Settings UI behavior verified for partial-conflict case (no top banner, mixed row colors)

## Technical Approach

Refactor `HotkeyInner::apply_config` from transactional to per-action best-effort. Pseudocode:

```
fn apply_config(config):
    accessibility = current_macos_accessibility_status()
    if accessibility == Missing:
        unregister all active best-effort
        publish_status(permission)
        return Ok

    desired = bindings_from_config(config)?   // duplicates rejected here, no partial state mutation

    let mut next_active = Vec<Binding>;
    let mut next_status = Vec<HotkeyBindingStatus>;
    let mut failed_count = 0;

    for desired_binding in desired:
        let current = active.entry(desired_binding.action);
        if current.shortcut == desired_binding.shortcut:
            next_active.push(current.clone())
            next_status.push(Registered{...})
            continue
        // unregister current (ignore error)
        registry.unregister(current.shortcut).ok()
        // try register desired
        match registry.register(desired_binding.shortcut):
            Ok => {
                next_active.push(desired_binding)
                next_status.push(Registered{...})
            }
            Err(err) => {
                // try restore previous; success/failure both end as Conflict UX
                if current.is_some():
                    if registry.register(current.shortcut).is_ok():
                        next_active.push(current)
                // either way, the desired action's state is Conflict
                next_status.push(Conflict{ message: err })
                failed_count += 1
            }
    set_active(next_active)
    last_error = if failed_count == desired.len() { Some(aggregate) } else { None }
    publish_status(next_status, last_error)
```

Implementation notes:
- `ActiveBindings` already stores `Vec<Binding>`; per-action lookup is `find by action`.
- Replace `rollback_failed_update` with helpers `unregister_one_best_effort` / `restore_previous`.
- `publish_status` keeps emitting one `HotkeyStatus` carrying all three `HotkeyBindingStatus` rows + global `last_error`.
- `set_active` now stores the actually-registered bindings (which may be the restored-previous when desired failed).
- `bindings_from_config` keeps its duplicate-check semantics. The new per-action loop runs only after duplicate check passes.

## Decision (ADR-lite)

**Context**: Original implementation chose transactional rollback for safety. In practice, startup-time previous bindings are empty, so rollback degenerates to "give up everything on the first conflict". On systems where common shortcuts are taken (Ctrl+Alt+S by QQ/WeChat screenshot, Ctrl+Alt+P by Office, etc.), the first-launch UX is "all three shortcuts dead" + a confusing banner naming only one of them. Users cannot tell which combinations actually conflict and lose access to the non-conflicting shortcuts they should have had.

**Decision**: Move `apply_config` to per-action best-effort. Each action's success/failure is independent. Duplicate-within-desired check stays whole-config (semantic protection — two actions can't share a shortcut). macOS Accessibility Missing stays whole-service (no shortcuts can be registered without permission, by OS contract).

**Consequences**:
- Better first-launch UX on systems with common-shortcut conflicts.
- Faster feedback loop in Settings (changing one row doesn't disturb the others).
- `last_error` becomes a meaningful "global failure" signal again instead of firing on every partial failure.
- Slightly more code in `apply_config`, but the new shape is straightforward.
- One existing test (`registration_failure_rolls_back_previous_bindings`) rewritten to assert per-action semantics; two new tests added for mixed and all-fail states.

## Out of Scope

- Surfacing the registry error `message` inline on the failed row (current UI only renders the state label "绑定失败"; the message lives in `HotkeyBindingStatus.message` but is not yet rendered).
- Discovering which OS app holds a conflicting shortcut.
- Auto-retry on conflict resolution (e.g. polling after another app's window closes).
- Recording shortcuts via key capture UI instead of typed strings.
- Changing the duplicate-check semantics or making them per-action.
- Refactoring `ActiveBindings` storage to a `HashMap<HotkeyAction, Binding>` (current `Vec<Binding>` works fine for n=3).

## Technical Notes

- Files affected:
  - `src-tauri/src/services/hotkeys.rs` — refactor `apply_config`, replace `rollback_failed_update`, adjust `publish_status` / `failed_status` semantics
  - Tests in the same file — update existing test, add 3 new tests (mixed, all-fail, edit-conflict-to-free)
  - No frontend file changes expected
- IPC shape unchanged:
  - `HotkeyStatus { bindings, macos_accessibility, last_error }`
  - `HotkeyBindingStatus { action, shortcut, state, message }`
  - `HotkeyBindingState { Registered, Conflict, PermissionMissing }`
- Verification:
  - `npm run ci` (full pipeline)
  - Manual E2E with real `Ctrl+Alt+S` conflict on user's Windows machine
