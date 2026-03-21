#![allow(clippy::module_name_repetitions)]

use std::sync::atomic::{AtomicBool, Ordering};

use tracing::warn;

use super::PlatformApi;

pub(crate) struct WindowsPlatform {
    warned: AtomicBool,
}

impl WindowsPlatform {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            warned: AtomicBool::new(false),
        }
    }
}

impl Default for WindowsPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformApi for WindowsPlatform {
    fn is_fullscreen_app_active(&self) -> bool {
        match detect_fullscreen() {
            Ok(result) => result,
            Err(error) => {
                if !self.warned.swap(true, Ordering::Relaxed) {
                    warn!("windows fullscreen detection failed: {error}");
                }
                false
            }
        }
    }
}

fn detect_fullscreen() -> Result<bool, String> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect};

    // SAFETY: Win32 APIs are called with initialized buffers and handles returned
    // by the system. Null/invalid handles are handled defensively.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return Ok(false);
        }

        let mut window_rect = RECT::default();
        GetWindowRect(hwnd, &raw mut window_rect)
            .map_err(|error| format!("GetWindowRect: {error}"))?;

        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if monitor.0.is_null() {
            return Ok(false);
        }

        let mut monitor_info = MONITORINFO {
            cbSize: u32::try_from(std::mem::size_of::<MONITORINFO>())
                .map_err(|error| format!("MONITORINFO size conversion: {error}"))?,
            ..Default::default()
        };

        if !GetMonitorInfoW(monitor, &raw mut monitor_info).as_bool() {
            return Err("GetMonitorInfoW returned false".to_string());
        }

        let monitor_rect = monitor_info.rcMonitor;
        let is_fullscreen = window_rect.left <= monitor_rect.left
            && window_rect.top <= monitor_rect.top
            && window_rect.right >= monitor_rect.right
            && window_rect.bottom >= monitor_rect.bottom;

        Ok(is_fullscreen)
    }
}
