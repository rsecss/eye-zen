#![allow(clippy::module_name_repetitions)]

use std::time::Duration;

use tracing::info;

use crate::error::Result;
use crate::models::types::DetectorCapabilities;
use crate::platform::PlatformApi;
use crate::services::{Service, ServiceContext};

/// Thin wrapper around `PlatformApi` for timer-facing detection queries.
pub(crate) struct DetectorService {
    platform: Box<dyn PlatformApi>,
}

impl DetectorService {
    #[must_use]
    pub(crate) fn new(platform: Box<dyn PlatformApi>) -> Self {
        Self { platform }
    }

    #[must_use]
    pub(crate) fn is_fullscreen(&self) -> bool {
        self.platform.is_fullscreen_app_active()
    }

    #[must_use]
    pub(crate) fn is_afk_for_threshold(&self, threshold_minutes: u32) -> bool {
        idle_reaches_threshold(self.platform.idle_duration(), threshold_minutes)
    }

    /// Whether the current foreground process matches any entry in `whitelist`.
    /// The whitelist is expected to already be sanitised (lowercase, trimmed).
    /// Returns `false` when no foreground process can be resolved (Wayland,
    /// transient race, no focused window) or when the list is empty.
    #[must_use]
    pub(crate) fn is_foreground_in_whitelist(&self, whitelist: &[String]) -> bool {
        if whitelist.is_empty() {
            return false;
        }
        let Some(name) = self.platform.get_foreground_process_name() else {
            return false;
        };
        whitelist.iter().any(|entry| entry == &name)
    }

    /// Return the whitelist entry the foreground process matched, normalised
    /// to the lowercase basename. Used by the Health Analysis pipeline to
    /// store the `process_hint` alongside a `process_whitelisted` suppress
    /// reason. Returns `None` when the list is empty, no foreground process
    /// is available, or no entry matches.
    #[must_use]
    pub(crate) fn foreground_whitelist_match(&self, whitelist: &[String]) -> Option<String> {
        if whitelist.is_empty() {
            return None;
        }
        let name = self.platform.get_foreground_process_name()?;
        whitelist.iter().find(|entry| *entry == &name).cloned()
    }

    #[must_use]
    pub(crate) fn capabilities(&self) -> DetectorCapabilities {
        DetectorCapabilities {
            afk_detection_supported: self.platform.supports_idle_detection(),
            foreground_process_detection_supported: self
                .platform
                .supports_foreground_process_detection(),
            fullscreen_detection_supported: self.platform.supports_fullscreen_detection(),
        }
    }
}

#[must_use]
pub(crate) fn idle_reaches_threshold(
    idle_duration: Option<Duration>,
    threshold_minutes: u32,
) -> bool {
    let threshold = Duration::from_secs(u64::from(threshold_minutes) * 60);
    idle_duration.is_some_and(|duration| duration >= threshold)
}

impl Service for DetectorService {
    async fn init(&self, _app: &ServiceContext) -> Result<()> {
        info!("detector service initialized in sync pull mode");
        Ok(())
    }

    async fn start(&self, _app: &ServiceContext) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        info!("detector service shutdown");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PlatformApi;

    // Test-only fixture; bools mirror PlatformApi's boolean methods so the
    // four-flag aggregate is intentional and not a state-machine candidate.
    #[allow(clippy::struct_excessive_bools)]
    struct MockPlatform {
        fullscreen: bool,
        idle_duration: Option<Duration>,
        supports_idle_detection: bool,
        foreground_process: Option<String>,
        supports_foreground_process: bool,
        supports_fullscreen: bool,
    }

    impl PlatformApi for MockPlatform {
        fn is_fullscreen_app_active(&self) -> bool {
            self.fullscreen
        }

        fn idle_duration(&self) -> Option<Duration> {
            self.idle_duration
        }

        fn supports_idle_detection(&self) -> bool {
            self.supports_idle_detection
        }

        fn supports_fullscreen_detection(&self) -> bool {
            self.supports_fullscreen
        }

        fn get_foreground_process_name(&self) -> Option<String> {
            self.foreground_process.clone()
        }

        fn supports_foreground_process_detection(&self) -> bool {
            self.supports_foreground_process
        }
    }

    impl MockPlatform {
        const fn new(fullscreen: bool) -> Self {
            Self {
                fullscreen,
                idle_duration: None,
                supports_idle_detection: false,
                foreground_process: None,
                supports_foreground_process: false,
                supports_fullscreen: true,
            }
        }
    }

    #[test]
    fn reports_fullscreen_from_platform() {
        let detector = DetectorService::new(Box::new(MockPlatform::new(true)));
        assert!(detector.is_fullscreen());
    }

    #[test]
    fn reports_not_fullscreen() {
        let detector = DetectorService::new(Box::new(MockPlatform::new(false)));
        assert!(!detector.is_fullscreen());
    }

    #[test]
    fn idle_below_threshold_is_not_afk() {
        assert!(!idle_reaches_threshold(Some(Duration::from_secs(299)), 5));
    }

    #[test]
    fn idle_at_threshold_is_afk() {
        assert!(idle_reaches_threshold(Some(Duration::from_mins(5)), 5));
    }

    #[test]
    fn idle_above_threshold_is_afk() {
        assert!(idle_reaches_threshold(Some(Duration::from_secs(301)), 5));
    }

    #[test]
    fn unavailable_idle_detection_is_not_afk() {
        assert!(!idle_reaches_threshold(None, 5));
    }

    #[test]
    fn reports_afk_from_platform_idle_duration() {
        let detector = DetectorService::new(Box::new(MockPlatform {
            fullscreen: false,
            idle_duration: Some(Duration::from_mins(5)),
            supports_idle_detection: true,
            foreground_process: None,
            supports_foreground_process: false,
            supports_fullscreen: true,
        }));

        assert!(detector.is_afk_for_threshold(5));
    }

    #[test]
    fn reports_detector_capabilities() {
        let detector = DetectorService::new(Box::new(MockPlatform {
            fullscreen: false,
            idle_duration: None,
            supports_idle_detection: true,
            foreground_process: None,
            supports_foreground_process: true,
            supports_fullscreen: true,
        }));

        let caps = detector.capabilities();
        assert!(caps.afk_detection_supported);
        assert!(caps.foreground_process_detection_supported);
        assert!(caps.fullscreen_detection_supported);
    }

    #[test]
    fn capability_reflects_unsupported_fullscreen_detection() {
        // Simulates the macOS path: capability=false even though the platform
        // would still answer `is_fullscreen_app_active()` with `false`.
        // The capability is the source of truth for UI; behaviour stays safe
        // because the stub never reports `true`.
        let detector = DetectorService::new(Box::new(MockPlatform {
            fullscreen: false,
            idle_duration: None,
            supports_idle_detection: true,
            foreground_process: None,
            supports_foreground_process: true,
            supports_fullscreen: false,
        }));

        let caps = detector.capabilities();
        assert!(!caps.fullscreen_detection_supported);
        // Consistency: the stub MUST NOT lie about an active fullscreen app
        // when the capability is gated off.
        assert!(!detector.is_fullscreen());
    }

    #[test]
    fn empty_whitelist_returns_false() {
        let detector = DetectorService::new(Box::new(MockPlatform {
            fullscreen: false,
            idle_duration: None,
            supports_idle_detection: false,
            foreground_process: Some("code.exe".to_string()),
            supports_foreground_process: true,
            supports_fullscreen: true,
        }));

        assert!(!detector.is_foreground_in_whitelist(&[]));
    }

    #[test]
    fn whitelist_match_returns_true() {
        let detector = DetectorService::new(Box::new(MockPlatform {
            fullscreen: false,
            idle_duration: None,
            supports_idle_detection: false,
            foreground_process: Some("code.exe".to_string()),
            supports_foreground_process: true,
            supports_fullscreen: true,
        }));

        let list = vec!["chrome.exe".to_string(), "code.exe".to_string()];
        assert!(detector.is_foreground_in_whitelist(&list));
    }

    #[test]
    fn whitelist_miss_returns_false() {
        let detector = DetectorService::new(Box::new(MockPlatform {
            fullscreen: false,
            idle_duration: None,
            supports_idle_detection: false,
            foreground_process: Some("notepad.exe".to_string()),
            supports_foreground_process: true,
            supports_fullscreen: true,
        }));

        let list = vec!["chrome.exe".to_string(), "code.exe".to_string()];
        assert!(!detector.is_foreground_in_whitelist(&list));
    }

    #[test]
    fn whitelist_returns_false_when_platform_returns_none() {
        let detector = DetectorService::new(Box::new(MockPlatform {
            fullscreen: false,
            idle_duration: None,
            supports_idle_detection: false,
            foreground_process: None,
            supports_foreground_process: false,
            supports_fullscreen: true,
        }));

        let list = vec!["code.exe".to_string()];
        assert!(!detector.is_foreground_in_whitelist(&list));
    }
}
