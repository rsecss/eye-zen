# Research: macOS Fullscreen Detection APIs (Tauri v2 / Rust)

- **Query**: Pick a macOS API to replace `detect_fullscreen_macos()` stub in `src-tauri/src/platform/macos.rs` (currently returns `DegradedFalse`).
- **Scope**: external (API survey) + internal (existing platform abstraction)
- **Date**: 2026-05-24

## Current State

`src-tauri/src/platform/macos.rs:102-116` already calls `CGWindowListCopyWindowInfo` successfully but bails out with `DegradedFalse` instead of comparing window bounds. `supports_fullscreen_detection()` returns `false` at line 74 so the Settings toggle is gated off. Dependencies present: `core-foundation ~0.10`, `core-graphics ~0.24`. `objc2` + `objc2-app-kit` are already pulled in transitively via Tauri.

## Option Comparison

### 1. CGWindowListCopyWindowInfo + bounds comparison (PUBLIC, RECOMMENDED PRIMARY)

- **Stability**: stable since 10.5; same API on Mojave→Sequoia.
- **Permissions**: none — does **not** require Screen Recording permission for the *list* (only `kCGWindowName` triggers TCC since 10.15). We only need `kCGWindowBounds` + `kCGWindowLayer` + `kCGWindowOwnerName`, all permission-free.
- **Reliability**: detects classic native fullscreen *and* "borderless windowed" games (covers Metal/OpenGL fullscreen exclusive too). For each on-screen window where `kCGWindowLayer == 0`, read `kCGWindowBounds` (CGRect dict) and compare against `CGDisplayBounds` for every active display (`CGGetActiveDisplayList`). Match = window covers entire display rect → fullscreen on that monitor.
- **Rust bindings**: already in `core-graphics 0.24` (`copy_window_info`, `kCGWindowBounds`, `CGDisplay`). Mirrors the existing `detect_foreground_process_name_macos()` pattern at `macos.rs:143-180` — same CF dictionary parsing scaffold.
- **CI friction**: zero. No entitlement, no `MACOSX_DEPLOYMENT_TARGET` bump, no Info.plist key, no codesigning impact. Works on macOS-latest GitHub runner.
- **Failure modes**: (a) Native fullscreen apps move to a *separate Space*; window only appears in the list when that Space is active — fine for "is fullscreen active on the currently-viewed monitor". (b) Mission Control overlay briefly inflates window counts — mitigated by layer==0 filter. (c) Notch on M-series MacBooks: `CGDisplayBounds` includes the notch area, but `kCGWindowBounds` of a fullscreen app on a notched display also includes it, so equality still holds.

### 2. NSWindow.styleMask & .fullScreen (PUBLIC, only sees OUR app)

- **Stability**: stable since 10.7.
- **Critical limitation**: `NSApp.windows` only enumerates windows owned by the current process. To check **other apps' windows** you need `NSRunningApplication.runningApplicationsWithBundleIdentifier:` followed by per-app AX queries — at which point you're back to option 3.
- **Verdict**: unsuitable for the use case. Skip.

### 3. Accessibility API `kAXFullScreenAttribute` (PUBLIC, needs user grant)

- **Stability**: stable since 10.10; `kAXFullScreenAttribute` is documented public.
- **Permissions**: requires the user to grant Accessibility in System Settings → Privacy & Security → Accessibility. First call shows nothing — you must prompt with `AXIsProcessTrustedWithOptions({kAXTrustedCheckOptionPrompt: true})`. This is a heavyweight UX cost for a *break reminder*.
- **Reliability**: most accurate (asks AppKit directly per app, so Spaces don't matter), but the permission requirement is a deal-breaker for a "quietly skip break during fullscreen" feature — users won't grant Accessibility for that.
- **Rust bindings**: no first-class crate; raw FFI via `objc2` or `accessibility-sys` (low-quality crate). Significant code volume.
- **Verdict**: skip unless option 1 proves unreliable in practice.

### 4. CGSGetWorkspace / CGSGetWindowProperty (PRIVATE)

- **Stability**: undocumented `_CGSPrivate.h`; broken/renamed across major versions (notable churn 10.14→11, 12→13).
- **Permissions**: none.
- **Reliability**: used by Dock + tools like Mission Control hackers; can directly detect "in fullscreen Space".
- **Verdict**: rejected. Private APIs violate the project's "let it crash, no hidden fallbacks" stance and the App Store/notarisation surface, even though Eyezen isn't App-Store-bound. Maintenance debt is high.

### 5. `active-win-pos-rs` / community crates

- `active-win-pos-rs` (mac branch) uses **option 1** under the hood — confirms this is the community-accepted approach.
- Stretchly (Electron) uses Electron's `screen` API which itself wraps `CGWindowListCopyWindowInfo` + bounds compare. Same approach.
- No mainstream Rust crate uses CGS private APIs; `swift-rs`-based solutions exist (e.g., for AX) but add Swift toolchain + Xcode dependency to CI — heavier than needed.

## Recommendation

**Primary**: extend the existing `detect_fullscreen_macos()` (option 1). Algorithm:

1. `CGGetActiveDisplayList` → vec of `CGDirectDisplayID` + their `CGDisplayBounds` rects (note: in "global display coordinate space", same coord system as `kCGWindowBounds`).
2. `CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements, kCGNullWindowID)`.
3. For each dict with `kCGWindowLayer == 0`: parse `kCGWindowBounds` CFDictionary into `(x, y, w, h)`. If `(x, y, w, h)` equals any display rect (with ≤1px tolerance for non-integer scales) → return `true`.
4. Flip `supports_fullscreen_detection()` to `true`.

Code structure mirrors `detect_foreground_process_name_macos()` (same file). Add a `read_cfdict` helper for the bounds sub-dictionary. ~80 LoC.

**Fallback**: keep the `FullscreenStatus::DegradedFalse` arm and the capability flag mechanism — if `CGGetActiveDisplayList` errors or returns zero displays, log + return `DegradedFalse` (capability stays effectively-true at runtime via the existing `is_fullscreen_app_active()` plumbing, which already maps `DegradedFalse → false`). No need for option 3 as second tier.

## Test Strategy (no Mac available locally)

- Unit tests: extract `compare_window_bounds_to_displays(window_rect: CGRect, displays: &[CGRect]) -> bool` as a pure helper, test with hand-crafted rects covering: exact match, sub-pixel offset, multi-monitor, off-by-display-bounds. CI runs these on macos-latest *and* linux/windows (pure helper has no cfg gate).
- Integration: macos-latest CI runner does not actually run a fullscreen window during tests — accept that `is_fullscreen_app_active()` will return `false` in CI. The contract test should be: capability reports `true` AND the function returns `Ok(_)` without panic.

## Caveats

- macOS-latest GH runner cannot exercise "real fullscreen present"; the integration test only validates the API binds + returns cleanly.
- Notched display CGRect quirks were stable through Sequoia (15.x) at cutoff, but worth a comment in code.
- If a user runs in Sidecar / external display only, `CGGetActiveDisplayList` returns the external — algorithm still works.

## External References

Not fetched live this session (no web tool active); pointers for verification before implementation:

- Apple Developer: `CGWindowListCopyWindowInfo` reference + `kCGWindow*` keys (CoreGraphics framework).
- Apple Developer: `CGDisplayBounds`, `CGGetActiveDisplayList`.
- crate docs.rs: `core-graphics` 0.24 — `window` and `display` modules.
- GitHub `dimusic/active-win-pos-rs` — reference impl of window enumeration.
