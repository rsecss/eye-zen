pub(crate) mod config;
pub(crate) mod context;
pub(crate) mod detector;
pub(crate) mod sound;
pub(crate) mod timer;
#[cfg(not(test))]
pub(crate) mod tray;
#[cfg(not(test))]
pub(crate) mod window;

#[cfg(not(test))]
use std::sync::Arc;

use crate::error::Result;

#[cfg(not(test))]
pub(crate) struct AppServices {
    pub(crate) config: config::ConfigService,
    pub(crate) timer: timer::TimerService,
    pub(crate) detector: detector::DetectorService,
    pub(crate) window: window::WindowService,
    pub(crate) sound: sound::SoundService,
    pub(crate) tray: tray::TrayService,
}

#[cfg(not(test))]
pub(crate) type SharedAppServices = Arc<AppServices>;
pub(crate) use context::ServiceContext;

/// Lifecycle contract shared by application services.
pub(crate) trait Service: Send + Sync {
    fn init(&self, app: &ServiceContext) -> impl std::future::Future<Output = Result<()>> + Send;
    fn start(&self, app: &ServiceContext) -> impl std::future::Future<Output = Result<()>> + Send;
    fn shutdown(&self) -> impl std::future::Future<Output = Result<()>> + Send;
}
