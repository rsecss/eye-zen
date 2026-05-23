#[cfg(target_os = "linux")]
pub(crate) mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(target_os = "windows")]
pub(crate) mod windows;

use std::time::Duration;

/// Platform abstraction for OS-specific capability detection.
pub(crate) trait PlatformApi: Send + Sync {
    fn is_fullscreen_app_active(&self) -> bool;
    fn idle_duration(&self) -> Option<Duration>;
    fn supports_idle_detection(&self) -> bool;

    /// Whether this platform reliably detects fullscreen foreground apps.
    /// macOS currently returns `false` while a real implementation is pending;
    /// Settings disables the "Fullscreen Skip" toggle accordingly.
    /// Windows / Linux X11 return `true`; Linux Wayland returns `false`.
    fn supports_fullscreen_detection(&self) -> bool;

    /// Return the foreground (focused) window's process executable basename,
    /// normalised to lowercase and trimmed. Returns `None` when no foreground
    /// window exists, the platform cannot resolve it (Wayland), or a transient
    /// race occurs (window closed mid-call).
    fn get_foreground_process_name(&self) -> Option<String>;

    /// Whether this platform supports foreground process detection at all.
    /// Wayland returns `false`; Windows / macOS / Linux X11 return `true`.
    fn supports_foreground_process_detection(&self) -> bool;
}

/// Normalise a platform-reported process name for whitelist comparison.
///
/// Trims surrounding whitespace and lowercases the result. Returns `None`
/// when the input is empty after trimming (treated as "no foreground" by
/// the caller).
#[must_use]
pub(crate) fn normalize_process_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_lowercase())
    }
}

/// Create the platform-specific implementation for the current target.
#[must_use]
pub(crate) fn create_platform() -> Box<dyn PlatformApi> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsPlatform::new())
    }

    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacosPlatform::new())
    }

    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxPlatform::new())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        compile_error!("unsupported platform");
    }
}
