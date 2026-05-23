pub(crate) mod config;
pub(crate) mod context;
pub(crate) mod detector;
pub(crate) mod hotkeys;
pub(crate) mod i18n;
pub(crate) mod schedule;
pub(crate) mod sound;
pub(crate) mod stat;
pub(crate) mod timer;
#[cfg(not(test))]
pub(crate) mod tray;
pub(crate) mod tray_tooltip;
#[cfg(not(test))]
pub(crate) mod window;
pub(crate) mod window_layout;

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
    pub(crate) stat: stat::StatService,
    pub(crate) tray: tray::TrayService,
    pub(crate) i18n: Arc<i18n::I18nService>,
    pub(crate) hotkeys: hotkeys::HotkeyService,
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
