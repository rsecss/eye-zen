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

/// Display-coordinate rectangle used to compare a foreground window's bounds
/// against each active display when detecting fullscreen apps. Kept
/// platform-agnostic so the comparison logic can be unit-tested on any host
/// (production callers only exist on macOS).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DisplayRect {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) w: f64,
    pub(crate) h: f64,
}

/// Returns `true` when the window rect matches any display rect within
/// `tolerance_px` on every dimension. macOS reports both rects in the same
/// global display coordinate space, so a fullscreen window's bounds equal
/// the display's bounds (modulo sub-pixel scaling on Retina/notched panels,
/// hence the tolerance).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[must_use]
pub(crate) fn covers_any_display(
    window: DisplayRect,
    displays: &[DisplayRect],
    tolerance_px: f64,
) -> bool {
    displays.iter().any(|d| {
        (window.x - d.x).abs() <= tolerance_px
            && (window.y - d.y).abs() <= tolerance_px
            && (window.w - d.w).abs() <= tolerance_px
            && (window.h - d.h).abs() <= tolerance_px
    })
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

#[cfg(test)]
mod tests {
    use super::{covers_any_display, normalize_process_name, DisplayRect};

    #[test]
    fn trims_and_lowercases() {
        assert_eq!(
            normalize_process_name("  Chrome.exe  "),
            Some("chrome.exe".into())
        );
    }

    #[test]
    fn returns_none_for_empty_after_trim() {
        assert_eq!(normalize_process_name(""), None);
        assert_eq!(normalize_process_name("   "), None);
    }

    #[test]
    fn preserves_inner_characters() {
        assert_eq!(
            normalize_process_name("MyApp-Beta.exe"),
            Some("myapp-beta.exe".into())
        );
    }

    fn rect(x: f64, y: f64, w: f64, h: f64) -> DisplayRect {
        DisplayRect { x, y, w, h }
    }

    #[test]
    fn covers_any_display_exact_match() {
        let main = rect(0.0, 0.0, 2560.0, 1440.0);
        assert!(covers_any_display(main, &[main], 1.0));
    }

    #[test]
    fn covers_any_display_within_tolerance() {
        // 27" Retina at 2x can report sub-pixel rects after scaling; the
        // 1px tolerance covers the rounding without admitting near-fullscreen
        // windowed apps (4px gap would not match).
        let display = rect(0.0, 0.0, 2560.0, 1440.0);
        let window = rect(0.5, -0.4, 2559.8, 1440.3);
        assert!(covers_any_display(window, &[display], 1.0));
    }

    #[test]
    fn covers_any_display_off_by_more_than_tolerance() {
        // Maximized-with-titlebar (Mission Control + Stage Manager edge): the
        // window is 40px shorter than the display, must not count as fullscreen.
        let display = rect(0.0, 0.0, 2560.0, 1440.0);
        let titled = rect(0.0, 24.0, 2560.0, 1416.0);
        assert!(!covers_any_display(titled, &[display], 1.0));
    }

    #[test]
    fn covers_any_display_multi_monitor_match_secondary() {
        // Two displays side-by-side; fullscreen game on the secondary.
        let primary = rect(0.0, 0.0, 2560.0, 1440.0);
        let secondary = rect(2560.0, 0.0, 1920.0, 1080.0);
        let game = rect(2560.0, 0.0, 1920.0, 1080.0);
        assert!(covers_any_display(game, &[primary, secondary], 1.0));
    }

    #[test]
    fn covers_any_display_zero_displays_never_matches() {
        // Defensive: if CGGetActiveDisplayList returned zero, the caller
        // should map to DegradedFalse upstream — but the helper itself
        // must not optimistically return true.
        let any = rect(0.0, 0.0, 100.0, 100.0);
        assert!(!covers_any_display(any, &[], 1.0));
    }

    #[test]
    fn covers_any_display_window_off_bounds() {
        // Mission Control overlay sometimes reports negative-origin rects
        // larger than any single display; must not match.
        let display = rect(0.0, 0.0, 2560.0, 1440.0);
        let overlay = rect(-500.0, -500.0, 3500.0, 2500.0);
        assert!(!covers_any_display(overlay, &[display], 1.0));
    }
}
