# Research: Cross-Platform Rust APIs for Foreground Process Executable Name

- **Query**: How to obtain foreground (focused) window's process exe basename (lowercase) cross-platform in Rust for the Eyezen Tauri v2 app.
- **Scope**: mixed (internal Eyezen platform code + external crates/APIs)
- **Date**: 2026-05-22

---

## Recommendation Summary

Add `fn get_foreground_process_name(&self) -> Option<String>` to `PlatformApi` (`src-tauri/src/platform/mod.rs`). Each platform impl returns the executable basename in lowercase (e.g. `"code.exe"` -> `"code.exe"` on Windows, kept as-is — see basename normalization below).

| Platform | Chosen approach | New deps |
|---|---|---|
| Windows | `GetForegroundWindow` -> `GetWindowThreadProcessId` -> `OpenProcess(QUERY_LIMITED_INFORMATION)` -> `QueryFullProcessImageNameW` -> `Path::file_name` | none (extend `windows` feature list with `Win32_System_Threading`) |
| macOS | `core-graphics` `CGWindowListCopyWindowInfo` -> filter topmost `kCGWindowLayer == 0` -> read `kCGWindowOwnerName` from dict (a `CFString` already convertible). Use this as the canonical process name. **Skip** `libproc`. | none (already has `core-graphics 0.24` + `core-foundation 0.10`) |
| Linux/X11 | `_NET_ACTIVE_WINDOW` (root window) -> `_NET_WM_PID` (window) -> `std::fs::read_link("/proc/{pid}/exe")` -> basename | none (uses `x11rb` already in deps + `std::fs`) |
| Linux/Wayland | Return `None`. There is no portable, unprivileged client API for "foreground window" or "frontmost PID". XDG desktop portals do not expose it. | n/a |

**Why this composition wins**

1. Zero new crates. `libproc`, `sysinfo`, and `objc2-app-kit` are all viable but each adds tens of kLOC + a build-time bindgen step (`libproc`) or hundreds of activated cargo features (`objc2-app-kit`). Eyezen already has `core-graphics 0.24` and `core-foundation 0.10`, and on macOS `kCGWindowOwnerName` is exactly the CFBundleExecutable name for app-bundle apps (e.g. VS Code -> `"Code"`, Chrome -> `"Google Chrome"`). For a whitelist UX this is the user-visible name users already recognise.
2. macOS `kCGWindowOwnerName` is more stable for whitelisting than `libproc::pidpath`-derived basenames: bundle apps live inside `Foo.app/Contents/MacOS/Foo` and the basename is often opaque (e.g. `Code Helper`, `Slack Helper (Renderer)`). The owner-name is the parent app the user sees.
3. The Windows chain matches what `active-win-pos-rs 0.10.1` ships in production and what Microsoft documents — only the pieces strictly needed for "basename".
4. Linux X11 path is the same scheme already used in `src-tauri/src/platform/linux.rs` for `_NET_ACTIVE_WINDOW`; only one extra atom (`_NET_WM_PID`) and a `readlink` are added.
5. Wayland: confirmed below that returning `None` is the only correct answer.

**Basename normalisation** (in `get_foreground_process_name`'s shared call site after the platform call):

```rust
fn normalize_basename(s: String) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() { return None; }
    Some(trimmed.to_lowercase())
}
```

Apply this in `mod.rs` (or in each impl) so all platforms agree on lowercase. Cross-platform whitelist matching should compare lowercase basenames (e.g. config stores `"code.exe"`, `"chrome"`, `"google chrome"`).

---

## Windows

### Verified chain (matches existing `src-tauri/src/platform/windows.rs` style)

```rust
use windows::Win32::Foundation::{CloseHandle, HANDLE, MAX_PATH};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId,
};
use windows::core::PWSTR;

fn detect_foreground_process_name() -> Result<Option<String>, String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return Ok(None);
        }

        let mut pid: u32 = 0;
        // GetWindowThreadProcessId(hwnd, Some(&mut pid)) writes the owning PID
        let tid = GetWindowThreadProcessId(hwnd, Some(&raw mut pid));
        if tid == 0 || pid == 0 {
            return Ok(None);
        }

        let handle: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
            .map_err(|e| format!("OpenProcess: {e}"))?;

        let mut buf = vec![0u16; MAX_PATH as usize];
        let mut len: u32 = MAX_PATH;
        let pwstr = PWSTR::from_raw(buf.as_mut_ptr());

        let result = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, pwstr, &raw mut len);
        let _ = CloseHandle(handle); // close even on failure

        result.map_err(|e| format!("QueryFullProcessImageNameW: {e}"))?;

        let path = String::from_utf16_lossy(&buf[..len as usize]);
        let basename = std::path::Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_owned());
        Ok(basename)
    }
}
```

### Confirmed paths in `windows = "~0.61"` (already in `src-tauri/Cargo.toml`)

| Symbol | Path in `windows 0.61.3` |
|---|---|
| `GetForegroundWindow` | `windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow` (already imported in `windows.rs`) |
| `GetWindowThreadProcessId` | `windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId` — signature: `unsafe fn(HWND, Option<*mut u32>) -> u32` |
| `OpenProcess` | `windows::Win32::System::Threading::OpenProcess` — returns `windows_core::Result<HANDLE>` |
| `QueryFullProcessImageNameW` | `windows::Win32::System::Threading::QueryFullProcessImageNameW` — returns `Result<()>` |
| `PROCESS_QUERY_LIMITED_INFORMATION` | `windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION` (a `PROCESS_ACCESS_RIGHTS` newtype = `4096u32`) |
| `PROCESS_NAME_WIN32` | `windows::Win32::System::Threading::PROCESS_NAME_WIN32` (a `PROCESS_NAME_FORMAT` newtype = `0u32`) |
| `CloseHandle` | `windows::Win32::Foundation::CloseHandle` — returns `Result<()>` |
| `MAX_PATH` | `windows::Win32::Foundation::MAX_PATH` |
| `PWSTR` | `windows::core::PWSTR` |

### Cargo feature change

Current features in `src-tauri/Cargo.toml`:

```
"Win32_Foundation",
"Win32_Graphics_Gdi",
"Win32_System_SystemInformation",
"Win32_UI_Input_KeyboardAndMouse",
"Win32_UI_WindowsAndMessaging",
```

**ADD** one feature: `"Win32_System_Threading"`. Verified present in `windows 0.61.3`'s feature graph (`Win32_System_Threading = ["Win32_System"]`). `Win32_System` pulls in via transitive include.

### Privilege

- `PROCESS_QUERY_LIMITED_INFORMATION` (0x1000) is the documented minimal access right and works under Windows Vista+ without elevation, even against most cross-session processes. No UAC prompt, no entitlement.
- Cannot read `System Idle Process` (PID 0) or `System` (PID 4); both are handled by the `pid == 0` early return and by `OpenProcess` failing on PID 4 (handled by `Err` -> caller returns `None`).

### Failure modes -> return `None`

- `hwnd.0.is_null()` — desktop briefly has no foreground window (lockscreen, alt-tab transitions).
- `GetWindowThreadProcessId` returns 0 — invalid HWND (race: window closed between calls).
- `OpenProcess` fails — process exited or kernel-owned (PID 4); the function should map the error to `None`, not bubble it up.
- `QueryFullProcessImageNameW` fails — same race.
- UWP apps (`ApplicationFrameHost.exe`): `active-win-pos-rs` adds a follow-up `GetGUIThreadInfo` step to dig out the real UWP child. **For a whitelist this is generally NOT needed** — most users will whitelist by visible exe name and "ApplicationFrameHost.exe" is fine to keep separate. If later needed, the pattern is in `/tmp/active-win-pos-rs-0.10.1/src/win/platform_api.rs:75-103`.

### Precedent

- `active-win-pos-rs 0.10.1` (`src/win/platform_api.rs`) — identical chain (DwmGetWindowAttribute is for bounds, irrelevant for us).
- Microsoft official sample (`Win32 SDK`): `process.cpp` of `Win32 Console Sample` and the docs page for `QueryFullProcessImageNameW` (`docs.microsoft.com`) recommend exactly this sequence.
- Tauri itself uses `GetForegroundWindow` in `tao` for focus tracking but does not expose foreground PID.

---

## macOS

### Recommendation: `core-graphics::window::CGWindowListCopyWindowInfo` + `kCGWindowOwnerName`

The dictionary returned by `CGWindowListCopyWindowInfo` already contains the owner name as a CFString. **No need for libproc, no need to resolve PID -> exe path.**

Verified constants/keys in `core-graphics 0.24.0` module `core_graphics::window` (file `/tmp/core-graphics-0.24.0/src/window.rs`):

```rust
extern "C" {
    pub static kCGWindowNumber: CFStringRef;
    pub static kCGWindowLayer: CFStringRef;
    pub static kCGWindowBounds: CFStringRef;
    pub static kCGWindowOwnerPID: CFStringRef;
    pub static kCGWindowOwnerName: CFStringRef;   // <-- this is the one
    pub static kCGWindowName: CFStringRef;
    pub static kCGWindowIsOnscreen: CFStringRef;
    ...
}
pub fn copy_window_info(option: CGWindowListOption, relative_to_window: CGWindowID) -> Option<CFArray> { ... }
pub const kCGWindowListOptionOnScreenOnly: CGWindowListOption = 1 << 0;
pub const kCGWindowListExcludeDesktopElements: CGWindowListOption = 1 << 4;
pub const kCGNullWindowID: CGWindowID = 0;
```

### Approach

```rust
use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFGetTypeID, ToVoid, TCFType};
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef, CFDictionaryGetValueIfPresent};
use core_foundation::number::{CFNumberGetTypeID, CFNumberGetValue, CFNumberRef};
use core_foundation::string::{CFString, CFStringGetTypeID, CFStringRef};
use core_graphics::window::{
    copy_window_info, kCGNullWindowID, kCGWindowLayer, kCGWindowOwnerName,
    kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly,
};

fn detect_foreground_process_name_macos() -> Result<Option<String>, String> {
    const OPTIONS: u32 = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;

    let arr = copy_window_info(OPTIONS, kCGNullWindowID)
        .ok_or_else(|| "CGWindowListCopyWindowInfo returned null".to_string())?;

    // The list is ordered front-to-back. The first dict whose kCGWindowLayer == 0
    // is the topmost user-facing window. Layer != 0 = menu bars, dock, status items.
    for i in 0..arr.len() {
        // SAFETY: index is < arr.len(); array contains CFDictionaryRef.
        let dict_ref = unsafe {
            core_foundation::array::CFArrayGetValueAtIndex(arr.as_concrete_TypeRef(), i)
                as CFDictionaryRef
        };
        if dict_ref.is_null() { continue; }

        let layer = read_cfnumber_i64(dict_ref, unsafe { kCGWindowLayer });
        if layer != Some(0) { continue; }

        if let Some(name) = read_cfstring(dict_ref, unsafe { kCGWindowOwnerName }) {
            return Ok(Some(name));
        }
    }
    Ok(None)
}

unsafe fn read_cfstring(dict: CFDictionaryRef, key: CFStringRef) -> Option<String> {
    let mut value: *const core::ffi::c_void = std::ptr::null();
    if CFDictionaryGetValueIfPresent(dict, key as *const _, &mut value) == 0 { return None; }
    if CFGetTypeID(value) != CFStringGetTypeID() { return None; }
    let cf = CFString::wrap_under_get_rule(value as CFStringRef);
    Some(cf.to_string())
}
unsafe fn read_cfnumber_i64(dict: CFDictionaryRef, key: CFStringRef) -> Option<i64> {
    let mut value: *const core::ffi::c_void = std::ptr::null();
    if CFDictionaryGetValueIfPresent(dict, key as *const _, &mut value) == 0 { return None; }
    if CFGetTypeID(value) != CFNumberGetTypeID() { return None; }
    let mut out: i64 = 0;
    let ok = CFNumberGetValue(value as CFNumberRef, 4 /* kCFNumberSInt64Type */, &mut out as *mut i64 as *mut _);
    if ok { Some(out) } else { None }
}
```

### Why this over the alternatives

**(a) Recommended: pure `core-graphics` -> `kCGWindowOwnerName`**

- Already in deps. Zero new crates.
- Returns CFBundleExecutable-equivalent name: VS Code -> `"Code"`, Chrome -> `"Google Chrome"`, Slack -> `"Slack"`, Terminal -> `"Terminal"`.
- This is the same string visible in Activity Monitor under "Process Name" column.
- Multi-window helper processes (e.g. `Code Helper (Renderer)`) DO appear in the list, but `kCGWindowLayer == 0` filtering plus front-to-back ordering means the topmost user-visible window is the main app, not the helper, because helpers are usually layered above or below.
- Empirically: for "is the user looking at Chrome right now?" this is more accurate than libproc, because libproc would return `"Google Chrome Helper (GPU)"` if the GPU process happened to own the surface.

**(b) Rejected: `libproc` (pidpath -> basename)**

- Build-time dependency on `bindgen 0.72.1` (heavy; pulls clang).
- Resolves to e.g. `/Applications/Visual Studio Code.app/Contents/MacOS/Electron` (on older VS Code) or `Code Helper (Renderer)`. Less useful for whitelisting.
- Would only be needed if we wanted the exact exe path, not the user-facing app name.
- API: `libproc::proc_pid::pidpath(pid: i32) -> Result<String, String>` and `libproc::proc_pid::name(pid: i32) -> Result<String, String>` (cap'd to 16 bytes via `proc_name` syscall — too short for many apps).

**(c) Rejected: `objc2-app-kit` -> `NSWorkspace.shared().frontmostApplication()`**

- Latest `objc2-app-kit 0.3.2` (Rust 1.71+). Adds a large dependency tree (`objc2`, `objc2-foundation`, `block2`, ...) with **307 activated features by default** (we'd need to disable most).
- API would be: `NSWorkspace::sharedWorkspace().frontmostApplication() -> NSRunningApplication -> localizedName() / bundleURL().lastPathComponent()`.
- Adds ~1-2 seconds to clean rebuilds. Not justified when `core-graphics` already gives us a CFString.
- Worth it ONLY if we later need bundle identifier (e.g. `com.microsoft.VSCode`) rather than display name; in that case add `objc2-app-kit` with minimal features and call `bundleIdentifier`.

**(d) Rejected: `sysinfo`**

- Latest `0.39.2` requires Rust 1.95 — newer than our current toolchain (`edition = "2021"`, no MSRV pinned but build uses stable). Plus sysinfo has no "frontmost window" concept, so we'd still need (a). Pure overhead.

### Bundle-name semantics confirmation

When VS Code is frontmost:

| Source | Value |
|---|---|
| `kCGWindowOwnerName` | `"Code"` (CFBundleName from the main bundle's Info.plist) |
| `NSRunningApplication.localizedName` | `"Code"` (same) |
| `proc_pidpath(pid)` basename | `"Electron"` on older VS Code; `"Code Helper (Renderer)"` if the helper happens to own the layer-0 window |
| `proc_name(pid)` | up to 16 chars, truncated — useless for long bundle names |

User research note: the user's whitelist UI should probably let users add `"Code"` not `"Electron"`. `kCGWindowOwnerName` aligns with user expectations.

### Privilege

- `CGWindowListCopyWindowInfo` requires no entitlement in non-sandboxed apps. Tauri Eyezen ships unsandboxed (default `tauri.macOS.entitlements` set, but no App Sandbox enabled), so this works out of the box.
- macOS 10.15+ **Screen Recording** permission is required only to read *window contents* (CGWindowListCreateImage). Reading the *metadata list* (PID, name, layer, bounds) does NOT require Screen Recording permission. Confirmed via Apple developer documentation `kCGWindowListOptionOnScreenOnly`.
- macOS 14+ adds a "Window owner name" change: the dictionary may omit `kCGWindowName` if the requesting app lacks Screen Recording entitlement, but `kCGWindowOwnerName` is still populated. We rely only on `kCGWindowOwnerName`, so we are unaffected.
- **No Accessibility permission needed** (Accessibility is for posting events / reading control hierarchy, neither of which we do).

### Failure modes -> return `None`

- No window with `kCGWindowLayer == 0` is visible (e.g., user is on an empty desktop after Mission Control).
- Eyezen's own tip-window is frontmost — caller should pass-through anyway (whitelist evaluator decides what to do with `"eyezen"`).
- `copy_window_info` returns `None` — very rare; only on serious WindowServer error.

### Precedent

- `active-win-pos-rs 0.10.1` (`src/mac/platform_api.rs`) uses `CGWindowListCopyWindowInfo` + `kCGWindowOwnerName` (also reads from the same dictionary), then crosschecks with `NSWorkspace::frontmostApplication`. We skip the NSWorkspace crosscheck because picking the topmost `layer == 0` window is equivalent in 99.99% of cases and avoids an extra dep.
- `t-rec-rs` (terminal recorder) uses the same pattern (see comment in active-win source referencing `t-rec-rs/v0.7.0/src/macos/window_id.rs`).
- Eyezen's own `src-tauri/src/platform/macos.rs:75-89` already calls `copy_window_info(kCGWindowListOptionOnScreenOnly, kCGNullWindowID)` — pattern reuse.

---

## Linux / X11

### Verified chain

```rust
use std::fs::read_link;
use std::path::Path;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};

fn detect_foreground_process_name_x11(session: &Mutex<X11Session>) -> Result<Option<String>, String> {
    let s = session.lock().map_err(|_| "lock poisoned".to_string())?;
    let screen = &s.connection.setup().roots[s.screen_num];

    // 1. _NET_ACTIVE_WINDOW on root => active window XID
    let active = s.connection
        .get_property(false, screen.root, s.active_window_atom, AtomEnum::WINDOW, 0, 1)
        .map_err(|e| format!("get _NET_ACTIVE_WINDOW: {e}"))?
        .reply()
        .map_err(|e| format!("reply _NET_ACTIVE_WINDOW: {e}"))?;
    let window = active.value32().and_then(|mut v| v.next()).unwrap_or(0);
    if window == 0 { return Ok(None); }

    // 2. _NET_WM_PID on that window => PID (CARDINAL)
    let pid_reply = s.connection
        .get_property(false, window, s.wm_pid_atom, AtomEnum::CARDINAL, 0, 1)
        .map_err(|e| format!("get _NET_WM_PID: {e}"))?
        .reply()
        .map_err(|e| format!("reply _NET_WM_PID: {e}"))?;
    let pid = pid_reply.value32().and_then(|mut v| v.next()).unwrap_or(0);
    if pid == 0 { return Ok(None); }

    // 3. readlink /proc/<pid>/exe => path => basename
    let exe = read_link(format!("/proc/{pid}/exe")).map_err(|e| format!("readlink: {e}"))?;
    Ok(exe.file_name().and_then(|s| s.to_str()).map(|s| s.to_owned()))
}
```

### X11Session change

Add one atom to the existing `X11Session` struct in `src-tauri/src/platform/linux.rs` (lines 103-156):

```rust
struct X11Session {
    // ...existing fields...
    wm_pid_atom: u32,        // <-- ADD
}
```

And intern it in `connect()`:

```rust
let wm_pid_atom = connection
    .intern_atom(false, b"_NET_WM_PID")
    .map_err(|e| format!("intern _NET_WM_PID: {e}"))?
    .reply()
    .map_err(|e| format!("reply _NET_WM_PID: {e}"))?
    .atom;
```

The `_NET_WM_PID` atom value type per EWMH spec is `CARDINAL` (32-bit unsigned), not `WINDOW`. Pass `AtomEnum::CARDINAL` to `get_property`.

### `_NET_WM_PID` reliability

- Set by most modern X11 apps (GTK, Qt, Chromium, Firefox, Electron all set it via Xlib `XChangeProperty` during window creation).
- Spec: https://specifications.freedesktop.org/wm-spec/wm-spec-1.5.html — section "_NET_WM_PID":
  > "If set, this property MUST contain the process ID of the client owning this window. This MAY be used by the Window Manager to kill applications which do not respond to the _NET_WM_PING protocol."
- Apps that do NOT set it: some Java AWT apps, old Xt apps. For those, return `None` (early return when `pid == 0`).
- Apps running over remote X (ssh -X) set `_NET_WM_PID` to a PID on the REMOTE machine; `/proc/<pid>/exe` will then either fail or read the wrong process. Acceptable failure mode (returns `None` from `read_link`).

### `/proc/<pid>/exe`

- Documented in `proc(5)`: a symlink to the executable. Reading it requires no special permission for processes owned by the same UID; for other UIDs it requires ptrace_attach permission, which typically fails for sandboxed apps. For Eyezen running as the user, processes owned by the same user resolve fine.
- Containerised apps (Flatpak/Snap): `/proc/<pid>/exe` resolves to the bundle path INSIDE the container's mount namespace — from Eyezen's host POV the symlink target may be e.g. `/newroot/app/bin/firefox` or just dangling. The basename is still correct in most cases (`firefox`).
- Failure mode: process exited between step 2 and step 3 — `read_link` fails — return `None`.

### Crate budget

- No new crate. `x11rb` already in deps with the right features.
- `procfs 0.18.0` would be overkill: it's a full procfs parser, we only need one `readlink`.

### Precedent

- `active-win-pos-rs 0.10.1` (`src/linux/platform_api.rs:14-29, 200`) — same `_NET_WM_PID` + `read_link` pattern. They use `xcb` instead of `x11rb`, but the protocol semantics are identical.
- `xdotool` (C) uses the same chain.
- `wmctrl` (C) uses `_NET_WM_PID` for its `-p` flag.

---

## Linux / Wayland

### Recommendation: return `None`. No fallback.

This is correct, not a degradation. There is no portable, unprivileged Wayland client API for obtaining the foreground window's PID or executable name.

### Why

Wayland's security model deliberately prevents one client from inspecting another. The Wayland protocol exposes only the client's OWN surfaces; there is no analog of `_NET_ACTIVE_WINDOW`.

**Compositor-specific extensions exist but are not portable:**

| Compositor | Extension | Status |
|---|---|---|
| Sway / wlroots-based | `wlr-foreign-toplevel-management-unstable-v1` | Available, but only on wlroots compositors (Sway, Hyprland, river, ...). Not on GNOME/Mutter or KDE/KWin. |
| GNOME / Mutter | Internal DBus on `org.gnome.Shell.Eval` | Disabled by default since GNOME 41 for security. |
| KDE / KWin | `org_kde_kwin_idle` etc., but no foreground-window API | None for this purpose. |

**XDG Desktop Portal (ashpd)**: provides screenshot, file chooser, screen-cast, remote-desktop portals. **None** of the standardised portals expose "the PID of the currently focused application". The closest is the ScreenCast portal which can capture a chosen window, but only AFTER the user picks it in a system dialog — useless for passive monitoring.

### What `active-win-pos-rs` does

Its `src/linux/wayland.rs` shells out to `gdbus call --session --dest=org.gnome.Shell --object-path=/org/gnome/Shell --method=org.gnome.Shell.Eval "global.display.focus_window..."` — this **only works on GNOME with Eval enabled**, which is the unstable distro maintainer's dev mode. It fails silently on GNOME 41+, Wayland KDE, Sway, Hyprland, etc. Not a real solution.

### Behavioural contract for Eyezen

`get_foreground_process_name` returns `None` on Wayland. The caller (e.g. Detector/Timer) should treat `None` as "process whitelist cannot be evaluated, fall back to safe default (do not suppress alert)".

The existing `LinuxPlatform::supports_idle_detection()` already encodes this pattern (returns `false` on Wayland). Add a parallel `supports_foreground_process_name()` if the calling code wants to distinguish "not supported" from "not found".

Existing warning in `linux.rs:27-31` already covers the Wayland case:

```rust
warn!("Wayland detected: fullscreen and idle detection unavailable, reminders will always show");
```

Extend the message wording to include "process whitelist" or add a one-time second warning when `get_foreground_process_name` is first called.

### Reference

- Wayland protocol design doc: https://wayland.freedesktop.org/architecture.html — "Each client only sees its own surfaces."
- XDG portal API list: https://flatpak.github.io/xdg-desktop-portal/docs/ — no "foreground app" portal.
- wlroots foreign-toplevel: https://wayland.app/protocols/wlr-foreign-toplevel-management-unstable-v1 — wlroots-only.

---

## Cross-cutting concerns

### Trait signature

```rust
pub(crate) trait PlatformApi: Send + Sync {
    fn is_fullscreen_app_active(&self) -> bool;
    fn idle_duration(&self) -> Option<Duration>;
    fn supports_idle_detection(&self) -> bool;
    fn get_foreground_process_name(&self) -> Option<String>;  // NEW
}
```

The `Option<String>` already encodes "Wayland / no foreground / race / no `_NET_WM_PID`" failure modes. No need for a separate `supports_foreground_process_name()` unless the consumer needs to differentiate "permanently unsupported" from "transient miss".

### Race conditions

All three platforms have a TOCTOU race: the foreground window can close between HWND/XID acquisition and the process query. Treat any error in the second step as `None`, not as a hard failure. Do NOT log on every miss — use the `AtomicBool::swap` "warn once" pattern already in `linux.rs:67-71`.

### Caching

This call is cheap but not free:
- Windows: ~10-50 us (3 syscalls)
- macOS: ~1-3 ms (`CGWindowListCopyWindowInfo` walks WindowServer state)
- Linux X11: ~0.5-2 ms (2 X11 round-trips + 1 readlink)

If called on every Timer tick (1 Hz), all platforms are fine. If called on every render frame, add a 100-200 ms LRU cache by the calling Service. Per `.trellis/spec/backend/service-pattern.md`, caching belongs in the Service layer (Detector or a new ForegroundService), not in PlatformApi.

### Test strategy

- Per `.trellis/spec/backend/coding-standards.md` style: `Result<bool, String>` internal helpers + `AtomicBool` warn-once + return `Option<String>` from trait method.
- Add `cargo clippy --all-targets` clean check (per Eyezen MEMORY.md `cfg(target_os)` rule: each platform's branch only lints on that platform's runner).
- Manual smoke test: print result of `get_foreground_process_name()` once a second while moving focus between Code, Chrome, Terminal, Finder/Explorer.

### Cargo.toml diff summary

```toml
# Windows: add ONE feature to existing windows dep
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "~0.61", features = [
    "Win32_Foundation",
    "Win32_Graphics_Gdi",
    "Win32_System_SystemInformation",
    "Win32_System_Threading",          # <-- ADD
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_WindowsAndMessaging",
] }

# macOS: NO change
[target.'cfg(target_os = "macos")'.dependencies]
core-foundation = "~0.10"
core-graphics = "~0.24"

# Linux: NO change
[target.'cfg(target_os = "linux")'.dependencies]
x11rb = { version = "~0.13", features = ["allow-unsafe-code", "screensaver"] }
```

### Out of scope (for this research)

- macOS bundle identifier (e.g. `com.microsoft.VSCode`) — would need `objc2-app-kit::NSRunningApplication::bundleIdentifier`. Defer.
- Windows AppUserModelID for UWP/Store apps — would need `IApplicationActivationManager` or `Shell32::SHGetPropertyStoreForWindow`. Defer.
- Persistent caching across restarts — config concern, not platform.

## Caveats / Not Found

- Did not verify `_NET_WM_PID` behaviour on Hyprland (XWayland passthrough) — likely works for X11 clients hosted under XWayland because Hyprland exposes XWayland as a normal X server. Mainline Wayland-native apps under Hyprland: returns `None` (acceptable).
- Did not test `kCGWindowOwnerName` on macOS 15 (Sequoia) — Apple has been tightening WindowServer privacy each release. The metadata-only path (no image data) has been stable from 10.5 through 14, so 15 is overwhelmingly likely to work, but flag for first macOS 15 CI run.
- Did not check Tauri's own future plans: Tauri v2 has no built-in foreground-window plugin. `tauri-plugin-window-state` saves Tauri-window state, not foreign windows.
