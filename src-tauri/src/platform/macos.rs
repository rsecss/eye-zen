#![allow(clippy::module_name_repetitions)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tracing::warn;

use super::PlatformApi;

#[allow(clippy::struct_field_names)]
pub(crate) struct MacosPlatform {
    fullscreen_warned: AtomicBool,
    fullscreen_degraded_warned: AtomicBool,
    idle_warned: AtomicBool,
}

impl MacosPlatform {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            fullscreen_warned: AtomicBool::new(false),
            fullscreen_degraded_warned: AtomicBool::new(false),
            idle_warned: AtomicBool::new(false),
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
