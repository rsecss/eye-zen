#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

/// Platform abstraction for OS-specific capability detection.
pub trait PlatformApi: Send + Sync {
    fn is_fullscreen_app_active(&self) -> bool;
}

/// Create the platform-specific implementation for the current target.
#[must_use]
pub fn create_platform() -> Box<dyn PlatformApi> {
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
