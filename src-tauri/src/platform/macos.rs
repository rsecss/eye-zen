#![allow(clippy::module_name_repetitions)]

use std::sync::atomic::{AtomicBool, Ordering};

use tracing::warn;

use super::PlatformApi;

pub(crate) struct MacosPlatform {
    warned: AtomicBool,
    degraded_warned: AtomicBool,
}

impl MacosPlatform {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            warned: AtomicBool::new(false),
            degraded_warned: AtomicBool::new(false),
        }
    }
}

impl PlatformApi for MacosPlatform {
    fn is_fullscreen_app_active(&self) -> bool {
        match detect_fullscreen_macos() {
            Ok(FullscreenStatus::Detected(result)) => result,
            Ok(FullscreenStatus::DegradedFalse) => {
                if !self.degraded_warned.swap(true, Ordering::Relaxed) {
                    warn!(
                        "macOS fullscreen detection is using a conservative fallback and will return false"
                    );
                }
                false
            }
            Err(error) => {
                if !self.warned.swap(true, Ordering::Relaxed) {
                    warn!("macOS fullscreen detection failed: {error}");
                }
                false
            }
        }
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
