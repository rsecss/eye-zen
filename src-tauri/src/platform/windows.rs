#![allow(clippy::module_name_repetitions)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tracing::warn;

use super::{normalize_process_name, PlatformApi};

#[allow(clippy::struct_field_names)]
pub(crate) struct WindowsPlatform {
    fullscreen_warned: AtomicBool,
    idle_warned: AtomicBool,
    foreground_warned: AtomicBool,
}

impl WindowsPlatform {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            fullscreen_warned: AtomicBool::new(false),
            idle_warned: AtomicBool::new(false),
            foreground_warned: AtomicBool::new(false),
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
                if !self.fullscreen_warned.swap(true, Ordering::Relaxed) {
                    warn!("windows fullscreen detection failed: {error}");
                }
                false
            }
        }
    }

    fn idle_duration(&self) -> Option<Duration> {
        match detect_idle_duration() {
            Ok(duration) => Some(duration),
            Err(error) => {
                if !self.idle_warned.swap(true, Ordering::Relaxed) {
                    warn!("windows idle detection failed: {error}");
                }
                None
            }
        }
    }

    fn supports_idle_detection(&self) -> bool {
        true
    }

    fn supports_fullscreen_detection(&self) -> bool {
        true
    }

    fn get_foreground_process_name(&self) -> Option<String> {
        match detect_foreground_process_name() {
            Ok(name) => name.as_deref().and_then(normalize_process_name),
            Err(error) => {
                if !self.foreground_warned.swap(true, Ordering::Relaxed) {
                    warn!("windows foreground process detection failed: {error}");
                }
                None
            }
        }
    }

    fn supports_foreground_process_detection(&self) -> bool {
        true
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

fn detect_idle_duration() -> Result<Duration, String> {
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    let mut input_info = LASTINPUTINFO {
        cbSize: u32::try_from(std::mem::size_of::<LASTINPUTINFO>())
            .map_err(|error| format!("LASTINPUTINFO size conversion: {error}"))?,
        dwTime: 0,
    };

    // SAFETY: LASTINPUTINFO is initialized with the documented cbSize, and the
    // Win32 calls do not retain the pointer after returning.
    unsafe {
        if !GetLastInputInfo(&raw mut input_info).as_bool() {
            return Err("GetLastInputInfo returned false".to_string());
        }

        let now = GetTickCount();
        Ok(Duration::from_millis(u64::from(
            now.wrapping_sub(input_info.dwTime),
        )))
    }
}

fn detect_foreground_process_name() -> Result<Option<String>, String> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, MAX_PATH};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    // SAFETY: Win32 APIs are invoked with validated handles, buffers sized to
    // MAX_PATH, and explicit CloseHandle on the process handle path.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return Ok(None);
        }

        let mut pid: u32 = 0;
        let tid = GetWindowThreadProcessId(hwnd, Some(&raw mut pid));
        if tid == 0 || pid == 0 {
            return Ok(None);
        }

        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return Ok(None);
        };

        let mut buf = vec![0u16; MAX_PATH as usize];
        let mut len: u32 = MAX_PATH;
        let pwstr = PWSTR::from_raw(buf.as_mut_ptr());

        let result = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, pwstr, &raw mut len);
        // CloseHandle releases the kernel reference returned by OpenProcess.
        // Logging a failure is the only sensible action; we cannot retry an
        // opaque handle, and the upstream query result is what callers care
        // about. Suppressing the must-use warning here is intentional.
        if let Err(err) = CloseHandle(handle) {
            warn!("CloseHandle failed: {err}");
        }

        if let Err(error) = result {
            return Err(format!("QueryFullProcessImageNameW: {error}"));
        }

        let len_usize = usize::try_from(len)
            .map_err(|error| format!("QueryFullProcessImageNameW length: {error}"))?;
        let path = String::from_utf16_lossy(&buf[..len_usize]);
        let basename = std::path::Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .map(str::to_owned);
        Ok(basename)
    }
}
