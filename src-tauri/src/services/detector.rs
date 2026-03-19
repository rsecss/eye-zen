#![allow(clippy::module_name_repetitions)]

use tracing::info;

use crate::error::Result;
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

    struct MockPlatform {
        fullscreen: bool,
    }

    impl PlatformApi for MockPlatform {
        fn is_fullscreen_app_active(&self) -> bool {
            self.fullscreen
        }
    }

    #[test]
    fn reports_fullscreen_from_platform() {
        let detector = DetectorService::new(Box::new(MockPlatform { fullscreen: true }));
        assert!(detector.is_fullscreen());
    }

    #[test]
    fn reports_not_fullscreen() {
        let detector = DetectorService::new(Box::new(MockPlatform { fullscreen: false }));
        assert!(!detector.is_fullscreen());
    }
}
