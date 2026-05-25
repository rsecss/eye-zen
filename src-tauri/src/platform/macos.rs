#![allow(clippy::module_name_repetitions)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tracing::warn;

use super::{covers_any_display, normalize_process_name, DisplayRect, PlatformApi};

#[allow(clippy::struct_field_names)]
pub(crate) struct MacosPlatform {
    fullscreen_warned: AtomicBool,
    fullscreen_degraded_warned: AtomicBool,
    idle_warned: AtomicBool,
    foreground_warned: AtomicBool,
}

impl MacosPlatform {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            fullscreen_warned: AtomicBool::new(false),
            fullscreen_degraded_warned: AtomicBool::new(false),
            idle_warned: AtomicBool::new(false),
            foreground_warned: AtomicBool::new(false),
        }
    }
}

impl PlatformApi for MacosPlatform {
    fn is_fullscreen_app_active(&self) -> bool {
        match detect_fullscreen_macos() {
            Ok(FullscreenStatus::Detected(result)) => result,
            Ok(FullscreenStatus::DegradedFalse) => {
                if !self
                    .fullscreen_degraded_warned
                    .swap(true, Ordering::Relaxed)
                {
                    warn!(
                        "macOS fullscreen detection is using a conservative fallback and will return false"
                    );
                }
                false
            }
            Err(error) => {
                if !self.fullscreen_warned.swap(true, Ordering::Relaxed) {
                    warn!("macOS fullscreen detection failed: {error}");
                }
                false
            }
        }
    }

    fn idle_duration(&self) -> Option<Duration> {
        match detect_idle_duration_macos() {
            Ok(duration) => Some(duration),
            Err(error) => {
                if !self.idle_warned.swap(true, Ordering::Relaxed) {
                    warn!("macOS idle detection failed: {error}");
                }
                None
            }
        }
    }

    fn supports_idle_detection(&self) -> bool {
        true
    }

    // macOS fullscreen detection is implemented via `CGWindowListCopyWindowInfo`
    // + `CGGetActiveDisplayList`; see `detect_fullscreen_macos` below. The
    // detection returns `DegradedFalse` only when display enumeration fails
    // (zero displays / FFI error), so the capability is `true` in the normal
    // case and the Settings "Fullscreen Skip" toggle is available.
    fn supports_fullscreen_detection(&self) -> bool {
        true
    }

    fn get_foreground_process_name(&self) -> Option<String> {
        match detect_foreground_process_name_macos() {
            Ok(Some(name)) => normalize_process_name(&name),
            Ok(None) => None,
            Err(error) => {
                if !self.foreground_warned.swap(true, Ordering::Relaxed) {
                    warn!("macOS foreground process detection failed: {error}");
                }
                None
            }
        }
    }

    fn supports_foreground_process_detection(&self) -> bool {
        true
    }
}

#[allow(dead_code)]
enum FullscreenStatus {
    Detected(bool),
    DegradedFalse,
}

/// Detect whether any on-screen application window covers an entire display.
///
/// Algorithm (per `research/macos-fullscreen-apis.md`, Option 1):
/// 1. Enumerate active displays via `CGGetActiveDisplayList` + `CGDisplayBounds`.
/// 2. Enumerate on-screen, non-desktop-element windows via
///    `CGWindowListCopyWindowInfo`.
/// 3. For each window with `kCGWindowLayer == 0`, parse `kCGWindowBounds`
///    into a `DisplayRect` and check it covers any display rect within 1px.
/// 4. If display enumeration fails or returns zero displays, return
///    `DegradedFalse` so the caller logs once and reports false.
fn detect_fullscreen_macos() -> Result<FullscreenStatus, String> {
    use core_foundation::array::CFArrayGetValueAtIndex;
    use core_foundation::base::TCFType;
    use core_foundation::dictionary::CFDictionaryRef;
    use core_graphics::window::{
        copy_window_info, kCGNullWindowID, kCGWindowBounds, kCGWindowLayer,
        kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly,
    };

    const OPTIONS: u32 = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;

    let displays = active_display_bounds()?;
    if displays.is_empty() {
        return Ok(FullscreenStatus::DegradedFalse);
    }

    let arr = copy_window_info(OPTIONS, kCGNullWindowID)
        .ok_or_else(|| "CGWindowListCopyWindowInfo returned null".to_string())?;

    // SAFETY: The CFArray owns CFDictionary references. Keys are interned
    // static CFStringRefs from CoreGraphics. Values are borrowed read-only.
    unsafe {
        let arr_ref = arr.as_concrete_TypeRef();
        let count = arr.len();
        for i in 0..count {
            let dict_ptr = CFArrayGetValueAtIndex(arr_ref, i) as CFDictionaryRef;
            if dict_ptr.is_null() {
                continue;
            }

            // Only foreground app windows live at layer 0; menu bars, Dock,
            // status-item windows and other system chrome use higher layers.
            if read_cfnumber_i64(dict_ptr, kCGWindowLayer) != Some(0) {
                continue;
            }

            let Some(bounds_dict) = read_cfdict_ref(dict_ptr, kCGWindowBounds) else {
                continue;
            };
            let Some(window_rect) = rect_from_dict(bounds_dict) else {
                continue;
            };

            if covers_any_display(window_rect, &displays, 1.0) {
                return Ok(FullscreenStatus::Detected(true));
            }
        }
    }

    Ok(FullscreenStatus::Detected(false))
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGGetActiveDisplayList(
        max_displays: u32,
        active_displays: *mut u32,
        display_count: *mut u32,
    ) -> i32;
    fn CGDisplayBounds(display: u32) -> CoreGraphicsCGRect;
    fn CGRectMakeWithDictionaryRepresentation(
        dict: core_foundation::dictionary::CFDictionaryRef,
        rect: *mut CoreGraphicsCGRect,
    ) -> u8;
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CoreGraphicsCGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CoreGraphicsCGSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CoreGraphicsCGRect {
    origin: CoreGraphicsCGPoint,
    size: CoreGraphicsCGSize,
}

impl CoreGraphicsCGRect {
    const fn zero() -> Self {
        Self {
            origin: CoreGraphicsCGPoint { x: 0.0, y: 0.0 },
            size: CoreGraphicsCGSize {
                width: 0.0,
                height: 0.0,
            },
        }
    }

    const fn into_display_rect(self) -> DisplayRect {
        DisplayRect {
            x: self.origin.x,
            y: self.origin.y,
            w: self.size.width,
            h: self.size.height,
        }
    }
}

fn active_display_bounds() -> Result<Vec<DisplayRect>, String> {
    // Cap at 16 displays — far above any real-world rig (Mac Pro 6-display max
    // for Apple silicon, Studio Display chains top out around 8).
    const MAX_DISPLAYS: u32 = 16;
    let mut ids = [0u32; MAX_DISPLAYS as usize];
    let mut count: u32 = 0;

    // SAFETY: CGGetActiveDisplayList writes up to MAX_DISPLAYS u32s into `ids`
    // and stores the actual count in `count`. Both pointers refer to stack
    // locals with sufficient capacity. The call is process-global and
    // re-entrant according to Apple docs.
    let err = unsafe { CGGetActiveDisplayList(MAX_DISPLAYS, ids.as_mut_ptr(), &raw mut count) };
    if err != 0 {
        return Err(format!("CGGetActiveDisplayList: CGError {err}"));
    }

    let mut rects = Vec::with_capacity(count as usize);
    for &id in ids.iter().take(count as usize) {
        // SAFETY: id was returned by CGGetActiveDisplayList in the same call;
        // CGDisplayBounds is safe to invoke on any active display ID and
        // returns CGRectZero for an inactive ID rather than crashing.
        let r = unsafe { CGDisplayBounds(id) };
        rects.push(r.into_display_rect());
    }
    Ok(rects)
}

/// Extract a `DisplayRect` from a `kCGWindowBounds` `CFDictionary` using the
/// dedicated Core Graphics helper (handles both integer and float-keyed
/// representations across macOS versions).
fn rect_from_dict(dict: core_foundation::dictionary::CFDictionaryRef) -> Option<DisplayRect> {
    let mut rect = CoreGraphicsCGRect::zero();
    // SAFETY: `dict` is a borrowed CFDictionaryRef from the window-info array.
    // CGRectMakeWithDictionaryRepresentation reads the dict and writes to the
    // stack-local rect; returns 0 (false) on malformed input, 1 (true) on
    // success. We treat any non-zero as success per the C ABI for Boolean.
    let ok = unsafe { CGRectMakeWithDictionaryRepresentation(dict, &raw mut rect) };
    if ok == 0 {
        None
    } else {
        Some(rect.into_display_rect())
    }
}

/// # Safety
/// Caller must ensure `dict` is a valid non-null `CFDictionaryRef` and `key` is
/// a valid static `CFStringRef` from CoreGraphics. The returned `CFDictionaryRef`
/// is borrowed (no retain) and must not outlive `dict`.
unsafe fn read_cfdict_ref(
    dict: core_foundation::dictionary::CFDictionaryRef,
    key: core_foundation::string::CFStringRef,
) -> Option<core_foundation::dictionary::CFDictionaryRef> {
    use core_foundation::base::CFGetTypeID;
    use core_foundation::dictionary::{CFDictionaryGetTypeID, CFDictionaryGetValueIfPresent};

    let mut value: *const core::ffi::c_void = std::ptr::null();
    if CFDictionaryGetValueIfPresent(dict, key.cast::<core::ffi::c_void>(), &raw mut value) == 0 {
        return None;
    }
    if value.is_null() || CFGetTypeID(value) != CFDictionaryGetTypeID() {
        return None;
    }
    Some(value as core_foundation::dictionary::CFDictionaryRef)
}

fn detect_idle_duration_macos() -> Result<Duration, String> {
    const CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE: u32 = 1;
    const CG_ANY_INPUT_EVENT_TYPE: u32 = u32::MAX;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventSourceSecondsSinceLastEventType(state_id: u32, event_type: u32) -> f64;
    }

    // SAFETY: CoreGraphics reads process-global event-source state and does not
    // retain pointers. Constants match the documented HID system / any input API.
    let seconds = unsafe {
        CGEventSourceSecondsSinceLastEventType(
            CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE,
            CG_ANY_INPUT_EVENT_TYPE,
        )
    };

    if !seconds.is_finite() || seconds < 0.0 {
        return Err(format!("invalid idle seconds: {seconds}"));
    }

    Ok(Duration::from_secs_f64(seconds))
}

fn detect_foreground_process_name_macos() -> Result<Option<String>, String> {
    use core_foundation::array::CFArrayGetValueAtIndex;
    use core_foundation::base::TCFType;
    use core_foundation::dictionary::CFDictionaryRef;
    use core_graphics::window::{
        copy_window_info, kCGNullWindowID, kCGWindowLayer, kCGWindowListExcludeDesktopElements,
        kCGWindowListOptionOnScreenOnly, kCGWindowOwnerName,
    };

    const OPTIONS: u32 = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;

    let arr = copy_window_info(OPTIONS, kCGNullWindowID)
        .ok_or_else(|| "CGWindowListCopyWindowInfo returned null".to_string())?;

    // SAFETY: The CFArray owns CFDictionary references. Keys are interned
    // static CFStringRefs from CoreGraphics. Values are borrowed read-only.
    unsafe {
        let arr_ref = arr.as_concrete_TypeRef();
        let count = arr.len();
        for i in 0..count {
            let dict_ptr = CFArrayGetValueAtIndex(arr_ref, i) as CFDictionaryRef;
            if dict_ptr.is_null() {
                continue;
            }

            let layer = read_cfnumber_i64(dict_ptr, kCGWindowLayer);
            if layer != Some(0) {
                continue;
            }

            if let Some(name) = read_cfstring(dict_ptr, kCGWindowOwnerName) {
                return Ok(Some(name));
            }
        }
    }

    Ok(None)
}

/// # Safety
/// Caller must ensure `dict` is a valid non-null `CFDictionaryRef` and `key` is
/// a valid static `CFStringRef` from CoreGraphics. The value is borrowed via
/// `wrap_under_get_rule`, which retains it for the lifetime of the returned String.
unsafe fn read_cfstring(
    dict: core_foundation::dictionary::CFDictionaryRef,
    key: core_foundation::string::CFStringRef,
) -> Option<String> {
    use core_foundation::base::{CFGetTypeID, TCFType};
    use core_foundation::dictionary::CFDictionaryGetValueIfPresent;
    use core_foundation::string::{CFString, CFStringGetTypeID, CFStringRef};

    let mut value: *const core::ffi::c_void = std::ptr::null();
    if CFDictionaryGetValueIfPresent(dict, key.cast::<core::ffi::c_void>(), &raw mut value) == 0 {
        return None;
    }
    if value.is_null() || CFGetTypeID(value) != CFStringGetTypeID() {
        return None;
    }
    let cf = CFString::wrap_under_get_rule(value as CFStringRef);
    Some(cf.to_string())
}

/// # Safety
/// Same contract as `read_cfstring`. The output buffer is written by
/// CoreFoundation according to `kCFNumberSInt64Type`.
unsafe fn read_cfnumber_i64(
    dict: core_foundation::dictionary::CFDictionaryRef,
    key: core_foundation::string::CFStringRef,
) -> Option<i64> {
    use core_foundation::base::CFGetTypeID;
    use core_foundation::dictionary::CFDictionaryGetValueIfPresent;
    use core_foundation::number::{CFNumberGetTypeID, CFNumberGetValue, CFNumberRef};

    const K_CF_NUMBER_S_INT64_TYPE: u32 = 4;

    let mut value: *const core::ffi::c_void = std::ptr::null();
    if CFDictionaryGetValueIfPresent(dict, key.cast::<core::ffi::c_void>(), &raw mut value) == 0 {
        return None;
    }
    if value.is_null() || CFGetTypeID(value) != CFNumberGetTypeID() {
        return None;
    }
    let mut out: i64 = 0;
    let ok = CFNumberGetValue(
        value as CFNumberRef,
        K_CF_NUMBER_S_INT64_TYPE,
        (&raw mut out).cast::<core::ffi::c_void>(),
    );
    if ok {
        Some(out)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_fullscreen_detection_is_true_after_real_impl() {
        // The CGWindowListCopyWindowInfo + CGGetActiveDisplayList real impl
        // landed; capability MUST now report `true` so the Settings toggle
        // becomes available. CI cannot exercise an actual fullscreen window —
        // see is_fullscreen_app_active_runs_without_panic below for the
        // weaker but achievable contract.
        let platform = MacosPlatform::new();
        assert!(platform.supports_fullscreen_detection());
    }

    #[test]
    fn is_fullscreen_app_active_runs_without_panic() {
        // The CI mac runner has no fullscreen window, so the expected result
        // is `false`. The point of the test is that the FFI bindings, display
        // enumeration and bounds-dict parsing all complete without panic —
        // any future regression in those will surface here even though we
        // can't exercise the positive-match path in CI.
        let platform = MacosPlatform::new();
        let _ = platform.is_fullscreen_app_active();
    }
}
