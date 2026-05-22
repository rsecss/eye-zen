#![allow(clippy::module_name_repetitions)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tracing::warn;

use super::{normalize_process_name, PlatformApi};

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

fn detect_fullscreen_macos() -> Result<FullscreenStatus, String> {
    use core_graphics::window::{
        copy_window_info, kCGNullWindowID, kCGWindowListOptionOnScreenOnly,
    };

    let windows = copy_window_info(kCGWindowListOptionOnScreenOnly, kCGNullWindowID)
        .ok_or_else(|| "CGWindowListCopyWindowInfo returned null".to_string())?;

    let _window_count = windows.len();

    // CoreGraphics access is verified above. Detailed bounds comparison needs
    // real macOS hardware to validate across Spaces/displays, so MVP degrades
    // conservatively here.
    Ok(FullscreenStatus::DegradedFalse)
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
/// Caller must ensure `dict` is a valid non-null CFDictionaryRef and `key` is
/// a valid static CFStringRef from CoreGraphics. The value is borrowed via
/// `wrap_under_get_rule`, which retains it for the lifetime of the returned String.
unsafe fn read_cfstring(
    dict: core_foundation::dictionary::CFDictionaryRef,
    key: core_foundation::string::CFStringRef,
) -> Option<String> {
    use core_foundation::base::{CFGetTypeID, TCFType};
    use core_foundation::dictionary::CFDictionaryGetValueIfPresent;
    use core_foundation::string::{CFString, CFStringGetTypeID, CFStringRef};

    let mut value: *const core::ffi::c_void = std::ptr::null();
    if CFDictionaryGetValueIfPresent(dict, key.cast::<core::ffi::c_void>(), &mut value) == 0 {
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
    if CFDictionaryGetValueIfPresent(dict, key.cast::<core::ffi::c_void>(), &mut value) == 0 {
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
