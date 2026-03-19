pub mod config;
pub mod context;
pub mod detector;
pub mod sound;
pub mod timer;
#[cfg(not(test))]
pub mod tray;
#[cfg(not(test))]
pub mod window;

#[cfg(not(test))]
use std::sync::Arc;

use crate::error::Result;

#[cfg(not(test))]
pub struct AppServices {
    pub config: config::ConfigService,
    pub timer: timer::TimerService,
    pub detector: detector::DetectorService,
    pub window: window::WindowService,
    pub sound: sound::SoundService,
    pub tray: tray::TrayService,
}

#[cfg(not(test))]
pub type SharedAppServices = Arc<AppServices>;
pub use context::ServiceContext;

/// Lifecycle contract shared by application services.
pub trait Service: Send + Sync {
    fn init(&self, app: &ServiceContext) -> impl std::future::Future<Output = Result<()>> + Send;
    fn start(&self, app: &ServiceContext) -> impl std::future::Future<Output = Result<()>> + Send;
    fn shutdown(&self) -> impl std::future::Future<Output = Result<()>> + Send;
}
