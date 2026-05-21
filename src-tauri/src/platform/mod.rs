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
