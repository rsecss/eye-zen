#![allow(clippy::module_name_repetitions)]

//! `TrayService` orchestrates the system-tray menu, tooltip and click
//! dispatch.
//!
//! The Tauri runtime APIs (`TrayIconBuilder`, `MenuItem::set_text`,
//! `WebviewWindow::emit`, …) are wrapped behind the [`TrayPort`] trait so the
//! deterministic orchestration — tooltip rendering, locale-driven menu retext,
//! pause↔resume label flip, menu-event dispatch — can be unit-tested without
//! constructing a Tauri `AppHandle`. The production wiring (`TrayIconBuilder`
//! callbacks, near-tray panel positioning) lives in the `#[cfg(not(test))]`
//! `runtime` sub-module.

use std::sync::{Arc, Mutex};

use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::error::Result;
use crate::events;
use crate::models::config::Config;
use crate::services::i18n::I18nService;
use crate::services::timer::{TimerState, TrayTooltip};
use crate::services::{Service, ServiceContext};

use super::tray_tooltip;

/// Identifier passed to the tray menu items so the production adapter can
/// locate the right `MenuItem` to update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrayMenuItemId {
    Pause,
    Settings,
    About,
    Quit,
}

impl TrayMenuItemId {
    #[cfg(test)]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Settings => "settings",
            Self::About => "about",
            Self::Quit => "quit",
        }
    }
}

/// Menu-item click outcome dispatched by `TrayService::on_menu_event`.
///
/// Tests can assert the dispatch result without simulating any IPC; the
/// production callback in `runtime` translates each outcome to the
/// corresponding side effect (timer command, window show, app exit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MenuAction {
    TogglePause,
    OpenSettings,
    OpenAbout,
    Quit,
    Unknown(String),
}

/// Initial menu labels passed when the production adapter builds the tray.
#[derive(Debug, Clone)]
pub(crate) struct TrayMenuLabels {
    pub(crate) pause: String,
    pub(crate) settings: String,
    pub(crate) about: String,
    pub(crate) quit: String,
    pub(crate) initial_tooltip: String,
}

/// Tauri-side tray operations the orchestration depends on.
///
/// Each method maps to one Tauri call so the orchestration can be exercised
/// in tests with a recording fake. `install_tray` is only called once at
/// service start; the rest are runtime updates.
pub(crate) trait TrayPort: Send + Sync {
    /// One-shot: create the tray icon, install the menu items, wire click
    /// callbacks. Tests can no-op this; production uses `TrayIconBuilder`.
    fn install_tray(&self, labels: TrayMenuLabels) -> Result<()>;

    /// Update the tray's tooltip text.
    fn set_tooltip(&self, text: &str) -> Result<()>;

    /// Update one menu item's text (e.g. flipping "Pause" ↔ "Resume").
    fn set_menu_text(&self, item: TrayMenuItemId, text: &str) -> Result<()>;

    /// Emit an event to a webview window (used by menu navigation).
    fn emit_to_window(&self, window: &str, event: &str, payload: &str) -> Result<()>;

    /// Restore a minimized window so it can be shown.
    fn unminimize_window(&self, label: &str) -> Result<()>;

    /// Show the named webview window (no-op if already visible).
    fn show_window(&self, label: &str) -> Result<()>;

    /// Focus the named webview window.
    fn focus_window(&self, label: &str) -> Result<()>;

    /// Exit the application.
    fn quit_app(&self);
}

pub(crate) struct TrayService {
    port: Arc<dyn TrayPort>,
    #[cfg_attr(test, allow(dead_code))]
    config_rx: watch::Receiver<Arc<Config>>,
    i18n: Arc<I18nService>,
    current_tooltip: Arc<Mutex<TrayTooltip>>,
    watch_handle: Mutex<Option<JoinHandle<()>>>,
}

impl TrayService {
    #[must_use]
    pub(crate) fn new(
        port: Arc<dyn TrayPort>,
        config_rx: watch::Receiver<Arc<Config>>,
        i18n: Arc<I18nService>,
    ) -> Self {
        Self {
            port,
            config_rx,
            i18n,
            current_tooltip: Arc::new(Mutex::new(TrayTooltip {
                state: TimerState::Working,
                remaining_secs: Some(20 * 60),
                pomodoro_progress: None,
            })),
            watch_handle: Mutex::new(None),
        }
    }

    /// Build the initial menu labels + tooltip text from the current i18n
    /// state. Production calls this once at service start to install the tray.
    pub(crate) fn initial_labels(&self) -> TrayMenuLabels {
        let tooltip = current_tooltip(&self.current_tooltip);
        TrayMenuLabels {
            pause: self.i18n.get("tray.pause").to_string(),
            settings: self.i18n.get("tray.settings").to_string(),
            about: self.i18n.get("tray.about").to_string(),
            quit: self.i18n.get("tray.quit").to_string(),
            initial_tooltip: tray_tooltip::render_tooltip(&self.i18n, tooltip),
        }
    }

    /// Dispatch a tray menu click. Pure function — production adapter
    /// translates the returned `MenuAction` into the actual side effects
    /// (timer command, window show, app exit) because those need to talk to
    /// `SharedAppServices` via `AppHandle::state`.
    pub(crate) fn on_menu_event(id: &str) -> MenuAction {
        match id {
            "pause" => MenuAction::TogglePause,
            "settings" => MenuAction::OpenSettings,
            "about" => MenuAction::OpenAbout,
            "quit" => MenuAction::Quit,
            other => MenuAction::Unknown(other.to_string()),
        }
    }

    /// Apply a timer-state update to the tray: store the new tooltip and push
    /// the rendered text to the port.
    ///
    /// # Errors
    ///
    /// Returns the error from `TrayPort::set_tooltip` so the caller can log
    /// or propagate it; tray updates are best-effort but the failure path is
    /// surfaced rather than swallowed.
    pub(crate) fn update_tooltip(&self, tooltip: TrayTooltip) -> Result<()> {
        store_tooltip(&self.current_tooltip, tooltip);
        self.port
            .set_tooltip(&tray_tooltip::render_tooltip(&self.i18n, tooltip))
    }

    /// Flip the pause↔resume menu text based on whether the timer is paused.
    ///
    /// # Errors
    ///
    /// Returns the error from `TrayPort::set_menu_text`.
    pub(crate) fn update_pause_item(&self, state: TimerState) -> Result<()> {
        let is_paused = state == TimerState::Paused;
        let mut tooltip = current_tooltip(&self.current_tooltip);
        tooltip.state = state;
        store_tooltip(&self.current_tooltip, tooltip);

        let text = tray_tooltip::pause_menu_text(&self.i18n, is_paused);
        self.port.set_menu_text(TrayMenuItemId::Pause, text)
    }

    /// Re-localise every menu item + tooltip when the user switches language.
    /// Used by the config-watch task; not directly user-facing.
    ///
    /// Production inlines this same logic inside the spawned watch task
    /// because the task moves `Arc`s instead of borrowing `&self`. This
    /// method captures the same contract so tests can exercise it without
    /// driving a tokio task.
    ///
    /// # Errors
    ///
    /// Returns the first port error encountered; subsequent items are still
    /// attempted so a transient failure on one item doesn't strand the
    /// others.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn apply_locale_change(&self) -> Result<()> {
        let tooltip = current_tooltip(&self.current_tooltip);
        let is_paused = tooltip.state == TimerState::Paused;
        let mut first_err: Option<crate::error::AppError> = None;

        let pause_text = tray_tooltip::pause_menu_text(&self.i18n, is_paused);
        if let Err(err) = self.port.set_menu_text(TrayMenuItemId::Pause, pause_text) {
            warn!("failed to update pause item text: {err}");
            first_err.get_or_insert(err);
        }
        if let Err(err) = self
            .port
            .set_menu_text(TrayMenuItemId::Settings, self.i18n.get("tray.settings"))
        {
            warn!("failed to update settings item text: {err}");
            first_err.get_or_insert(err);
        }
        if let Err(err) = self
            .port
            .set_menu_text(TrayMenuItemId::About, self.i18n.get("tray.about"))
        {
            warn!("failed to update about item text: {err}");
            first_err.get_or_insert(err);
        }
        if let Err(err) = self
            .port
            .set_menu_text(TrayMenuItemId::Quit, self.i18n.get("tray.quit"))
        {
            warn!("failed to update quit item text: {err}");
            first_err.get_or_insert(err);
        }
        if let Err(err) = self
            .port
            .set_tooltip(&tray_tooltip::render_tooltip(&self.i18n, tooltip))
        {
            warn!("failed to update tray tooltip on locale change: {err}");
            first_err.get_or_insert(err);
        }

        match first_err {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    /// Translate a `MenuAction::OpenSettings | OpenAbout` to the tab name +
    /// window operations the production adapter must perform.
    ///
    /// # Errors
    ///
    /// Returns the first port error encountered while emitting the
    /// `navigate_tab` event, restoring, showing, or focusing the main
    /// window.
    pub(crate) fn show_main_window_with_tab(&self, tab: &str) -> Result<()> {
        // User just clicked a tray menu item that expects navigation; emit
        // the tab and surface every step's failure. `unminimize` is allowed
        // to fail when the window is already restored — that path returns
        // Ok and is handled below; only true OS errors are logged.
        if let Err(err) = self
            .port
            .emit_to_window("main-window", events::NAVIGATE_TAB, tab)
        {
            warn!("failed to emit navigate_tab for '{tab}': {err}");
            return Err(err);
        }
        if let Err(err) = self.port.unminimize_window("main-window") {
            warn!("failed to unminimize main window: {err}");
            // do not bail; unminimize commonly fails when already restored
        }
        if let Err(err) = self.port.show_window("main-window") {
            warn!("failed to show main window: {err}");
            return Err(err);
        }
        if let Err(err) = self.port.focus_window("main-window") {
            warn!("failed to focus main window: {err}");
            return Err(err);
        }
        Ok(())
    }

    /// Quit the application.
    pub(crate) fn quit(&self) {
        self.port.quit_app();
    }

    /// Apply the initial start-up config + install the tray.
    ///
    /// # Errors
    ///
    /// Propagates `TrayPort::install_tray` and the initial tooltip push.
    pub(crate) fn start_with_initial_config(&self, config: &Config) -> Result<()> {
        self.i18n.set_locale(&config.display.language);
        store_tooltip(
            &self.current_tooltip,
            TrayTooltip {
                state: TimerState::Working,
                remaining_secs: Some(config.timer.work_minutes.saturating_mul(60)),
                pomodoro_progress: None,
            },
        );

        self.port.install_tray(self.initial_labels())?;
        let tooltip = current_tooltip(&self.current_tooltip);
        self.port
            .set_tooltip(&tray_tooltip::render_tooltip(&self.i18n, tooltip))?;
        Ok(())
    }
}

fn current_tooltip(slot: &Arc<Mutex<TrayTooltip>>) -> TrayTooltip {
    match slot.lock() {
        Ok(guard) => *guard,
        Err(err) => {
            warn!("tray tooltip mutex poisoned: {err}");
            *err.into_inner()
        }
    }
}

fn store_tooltip(slot: &Arc<Mutex<TrayTooltip>>, next: TrayTooltip) {
    match slot.lock() {
        Ok(mut guard) => {
            *guard = next;
        }
        Err(err) => {
            warn!("tray tooltip mutex poisoned while storing: {err}");
            *err.into_inner() = next;
        }
    }
}

impl Service for TrayService {
    async fn init(&self, app: &ServiceContext) -> Result<()> {
        if app.app_handle().is_none() {
            info!("tray service initialized without runtime context");
            return Ok(());
        }
        info!("tray service initialized");
        Ok(())
    }

    #[cfg_attr(test, allow(unused_variables))]
    async fn start(&self, app: &ServiceContext) -> Result<()> {
        // Production: install the tray, push initial tooltip, spawn the
        // config-watch task. In the test build `app_handle()` returns
        // `Option<()>` and the production code path is unreachable; tests
        // drive `start_with_initial_config` directly.
        #[cfg(not(test))]
        if let Some(app_handle) = app.app_handle() {
            return self.production_start(&app_handle);
        }
        info!("tray service started without runtime context");
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        match self.watch_handle.lock() {
            Ok(mut guard) => {
                if let Some(handle) = guard.take() {
                    handle.abort();
                }
            }
            Err(err) => {
                warn!("tray watch handle mutex poisoned during shutdown: {err}");
                if let Some(handle) = err.into_inner().take() {
                    handle.abort();
                }
            }
        }

        info!("tray service shutdown");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Production runtime: real `TrayPort` adapter + production-side `start` that
// owns config-watch spawning and tray-panel positioning. Compiled only in
// non-test builds because it references Tauri runtime types.
// ---------------------------------------------------------------------------

#[cfg(not(test))]
mod runtime {
    use super::{
        current_tooltip, MenuAction, TrayMenuItemId, TrayMenuLabels, TrayPort, TrayService,
    };
    use crate::error::{AppError, Result};
    use crate::services::timer::{TimerState, TrayTooltip, UserEvent};
    use crate::services::tray_tooltip;
    use crate::services::SharedAppServices;
    use std::sync::{Arc, Mutex};
    use tauri::{
        menu::{Menu, MenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
        AppHandle, Emitter, Manager, Wry,
    };
    use tracing::{info, warn};

    type SharedMenuItems = Arc<Mutex<MenuItemSlots>>;

    #[derive(Clone, Default)]
    struct MenuItemSlots {
        pause: Option<MenuItem<Wry>>,
        settings: Option<MenuItem<Wry>>,
        about: Option<MenuItem<Wry>>,
        quit: Option<MenuItem<Wry>>,
    }

    /// `TrayPort` implementation backed by a live Tauri `AppHandle`.
    pub(crate) struct TauriTrayPort {
        app: AppHandle,
        menu_items: SharedMenuItems,
    }

    impl TauriTrayPort {
        #[must_use]
        pub(crate) fn new(app: AppHandle) -> Self {
            Self {
                app,
                menu_items: Arc::new(Mutex::new(MenuItemSlots::default())),
            }
        }

        fn io_error(err: impl std::fmt::Display) -> AppError {
            AppError::IoError {
                message: err.to_string(),
            }
        }

        fn store_items(slots: &SharedMenuItems, next: MenuItemSlots) {
            match slots.lock() {
                Ok(mut guard) => *guard = next,
                Err(err) => {
                    warn!("tray menu items mutex poisoned while storing: {err}");
                    *err.into_inner() = next;
                }
            }
        }

        fn snapshot_items(slots: &SharedMenuItems) -> MenuItemSlots {
            match slots.lock() {
                Ok(guard) => guard.clone(),
                Err(err) => {
                    warn!("tray menu items mutex poisoned: {err}");
                    err.into_inner().clone()
                }
            }
        }
    }

    impl TrayPort for TauriTrayPort {
        fn install_tray(&self, labels: TrayMenuLabels) -> Result<()> {
            let pause_item =
                MenuItem::with_id(&self.app, "pause", &labels.pause, true, None::<&str>)
                    .map_err(Self::io_error)?;
            let settings_item =
                MenuItem::with_id(&self.app, "settings", &labels.settings, true, None::<&str>)
                    .map_err(Self::io_error)?;
            let about_item =
                MenuItem::with_id(&self.app, "about", &labels.about, true, None::<&str>)
                    .map_err(Self::io_error)?;
            let quit_item = MenuItem::with_id(&self.app, "quit", &labels.quit, true, None::<&str>)
                .map_err(Self::io_error)?;

            let menu = Menu::with_items(
                &self.app,
                &[&pause_item, &settings_item, &about_item, &quit_item],
            )
            .map_err(Self::io_error)?;

            Self::store_items(
                &self.menu_items,
                MenuItemSlots {
                    pause: Some(pause_item.clone()),
                    settings: Some(settings_item.clone()),
                    about: Some(about_item.clone()),
                    quit: Some(quit_item.clone()),
                },
            );

            let app_for_menu = self.app.clone();

            TrayIconBuilder::with_id("main")
                .tooltip(&labels.initial_tooltip)
                .menu(&menu)
                .on_menu_event(move |_app, event| {
                    dispatch_menu_event(&app_for_menu, event.id().as_ref());
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        toggle_tray_panel(tray.app_handle(), &rect);
                    }
                })
                .build(&self.app)
                .map_err(Self::io_error)?;

            Ok(())
        }

        fn set_tooltip(&self, text: &str) -> Result<()> {
            let Some(tray) = self.app.tray_by_id("main") else {
                warn!("tray icon 'main' not found");
                return Ok(());
            };
            tray.set_tooltip(Some(text.to_string()))
                .map_err(Self::io_error)
        }

        fn set_menu_text(&self, item: TrayMenuItemId, text: &str) -> Result<()> {
            let snapshot = Self::snapshot_items(&self.menu_items);
            let menu_item = match item {
                TrayMenuItemId::Pause => snapshot.pause,
                TrayMenuItemId::Settings => snapshot.settings,
                TrayMenuItemId::About => snapshot.about,
                TrayMenuItemId::Quit => snapshot.quit,
            };
            let Some(menu_item) = menu_item else {
                warn!("tray menu item {:?} not yet installed", item);
                return Ok(());
            };
            menu_item.set_text(text).map_err(Self::io_error)
        }

        fn emit_to_window(&self, window: &str, event: &str, payload: &str) -> Result<()> {
            let Some(target) = self.app.get_webview_window(window) else {
                warn!("window '{window}' not found for emit");
                return Err(AppError::InvalidOperation {
                    operation: format!("emit {event} to {window}"),
                    reason: "window not found".to_string(),
                });
            };
            target.emit(event, payload).map_err(Self::io_error)
        }

        fn unminimize_window(&self, label: &str) -> Result<()> {
            let Some(window) = self.app.get_webview_window(label) else {
                warn!("window '{label}' not found for unminimize");
                return Ok(());
            };
            window.unminimize().map_err(Self::io_error)
        }

        fn show_window(&self, label: &str) -> Result<()> {
            let Some(window) = self.app.get_webview_window(label) else {
                warn!("window '{label}' not found for show");
                return Err(AppError::InvalidOperation {
                    operation: format!("show {label}"),
                    reason: "window not found".to_string(),
                });
            };
            window.show().map_err(Self::io_error)
        }

        fn focus_window(&self, label: &str) -> Result<()> {
            let Some(window) = self.app.get_webview_window(label) else {
                warn!("window '{label}' not found for focus");
                return Err(AppError::InvalidOperation {
                    operation: format!("focus {label}"),
                    reason: "window not found".to_string(),
                });
            };
            window.set_focus().map_err(Self::io_error)
        }

        fn quit_app(&self) {
            self.app.exit(0);
        }
    }

    /// Dispatch a tray-menu click. Reads the timer snapshot and translates
    /// the click into the appropriate service call. Mirrors what
    /// `TrayService::on_menu_event` reports, but with actual side effects.
    fn dispatch_menu_event(app: &AppHandle, id: &str) {
        match TrayService::on_menu_event(id) {
            MenuAction::TogglePause => {
                info!("tray: pause/resume clicked");
                let Some(services) = app.try_state::<SharedAppServices>() else {
                    warn!("shared services unavailable while handling tray pause");
                    return;
                };
                let services = services.inner().clone();
                tauri::async_runtime::spawn(async move {
                    let snapshot = services.timer.state_snapshot().await;
                    let event = if snapshot.state == "paused" {
                        UserEvent::Resume
                    } else {
                        UserEvent::Pause
                    };
                    if let Err(err) = services.timer.handle_user_event(event).await {
                        warn!("tray pause/resume action failed: {err}");
                    }
                });
            }
            MenuAction::OpenSettings => {
                info!("tray: settings clicked");
                let Some(services) = app.try_state::<SharedAppServices>() else {
                    warn!("shared services unavailable while opening settings");
                    return;
                };
                if let Err(err) = services.tray.show_main_window_with_tab("Settings") {
                    warn!("failed to show settings via tray: {err}");
                }
            }
            MenuAction::OpenAbout => {
                info!("tray: about clicked");
                let Some(services) = app.try_state::<SharedAppServices>() else {
                    warn!("shared services unavailable while opening about");
                    return;
                };
                if let Err(err) = services.tray.show_main_window_with_tab("About") {
                    warn!("failed to show about via tray: {err}");
                }
            }
            MenuAction::Quit => {
                info!("tray: quit clicked");
                // SharedAppServices is registered before the tray callback can
                // fire (install_tray runs after app.manage(services) in
                // lib.rs::setup). If state lookup ever fails, the timing
                // invariant has been violated — log and skip rather than
                // double-quit through `app.exit(0)`.
                let Some(services) = app.try_state::<SharedAppServices>() else {
                    warn!("shared services unavailable while handling tray quit");
                    return;
                };
                services.tray.quit();
            }
            MenuAction::Unknown(other) => warn!("tray: unknown menu item: {other}"),
        }
    }

    /// Position the tray panel near the system tray icon.
    ///
    /// - Finds the monitor containing the tray icon via `available_monitors()`
    /// - Uses monitor origin offset (correct for secondary monitors)
    /// - Windows (taskbar bottom): panel above icon, horizontally centered
    /// - macOS (menu bar top): panel below icon, horizontally centered
    /// - Clamps both axes to monitor bounds to prevent off-screen rendering
    fn position_near_tray(panel: &tauri::WebviewWindow, icon_rect: &tauri::Rect) {
        let Ok(panel_size) = panel.outer_size() else {
            return;
        };

        let scale = panel.scale_factor().unwrap_or(1.0);

        let (icon_x, icon_y) = match &icon_rect.position {
            tauri::Position::Physical(p) => (f64::from(p.x), f64::from(p.y)),
            tauri::Position::Logical(p) => (p.x * scale, p.y * scale),
        };

        let (icon_w, icon_h) = match &icon_rect.size {
            tauri::Size::Physical(s) => (f64::from(s.width), f64::from(s.height)),
            tauri::Size::Logical(s) => (s.width * scale, s.height * scale),
        };

        let panel_w = f64::from(panel_size.width);
        let panel_h = f64::from(panel_size.height);

        #[allow(clippy::option_if_let_else, clippy::map_unwrap_or)]
        let (mon_x, mon_y, mon_w, mon_h) = panel
            .available_monitors()
            .ok()
            .and_then(|monitors| {
                monitors.into_iter().find(|m| {
                    let pos = m.position();
                    let size = m.size();
                    let mx = f64::from(pos.x);
                    let my = f64::from(pos.y);
                    let mw = f64::from(size.width);
                    let mh = f64::from(size.height);
                    icon_x >= mx && icon_x < mx + mw && icon_y >= my && icon_y < my + mh
                })
            })
            .map(|m| {
                let pos = m.position();
                let size = m.size();
                (
                    f64::from(pos.x),
                    f64::from(pos.y),
                    f64::from(size.width),
                    f64::from(size.height),
                )
            })
            .unwrap_or_else(|| {
                panel
                    .primary_monitor()
                    .ok()
                    .flatten()
                    .map_or((0.0, 0.0, 1920.0, 1080.0), |m| {
                        let pos = m.position();
                        let size = m.size();
                        (
                            f64::from(pos.x),
                            f64::from(pos.y),
                            f64::from(size.width),
                            f64::from(size.height),
                        )
                    })
            });

        let mut x = icon_x + (icon_w / 2.0) - (panel_w / 2.0);
        let mut y = if icon_y - mon_y > mon_h / 2.0 {
            icon_y - panel_h - 4.0
        } else {
            icon_y + icon_h + 4.0
        };

        let margin = 8.0;
        x = x.clamp(mon_x + margin, mon_x + mon_w - panel_w - margin);
        y = y.clamp(mon_y + margin, mon_y + mon_h - panel_h - margin);

        #[allow(clippy::cast_possible_truncation)]
        if let Err(err) = panel.set_position(tauri::PhysicalPosition::new(x as i32, y as i32)) {
            warn!("failed to position tray panel: {err}");
        }
    }

    fn toggle_tray_panel(app: &AppHandle, icon_rect: &tauri::Rect) {
        let Some(panel) = app.get_webview_window("tray-panel") else {
            warn!("tray-panel window not found");
            return;
        };
        match panel.is_visible() {
            Ok(true) => {
                if let Err(err) = panel.hide() {
                    warn!("failed to hide tray panel: {err}");
                }
            }
            Ok(false) => {
                position_near_tray(&panel, icon_rect);
                if let Err(err) = panel.show() {
                    warn!("failed to show tray panel: {err}");
                    return;
                }
                if let Err(err) = panel.set_focus() {
                    warn!("failed to focus tray panel: {err}");
                }
            }
            Err(err) => warn!("failed to read tray panel visibility: {err}"),
        }
    }

    impl TrayService {
        /// Production-side start: install tray, push initial tooltip, spawn
        /// the config-watch task that re-localises menu items + tooltip on
        /// language changes.
        ///
        /// # Errors
        ///
        /// Propagates port-level failures from installing the tray icon or
        /// pushing the initial tooltip.
        pub(crate) fn production_start(&self, app: &AppHandle) -> Result<()> {
            let mut config_rx = self.config_rx.clone();
            let config = Arc::clone(&config_rx.borrow_and_update());
            self.start_with_initial_config(&config)?;

            self.spawn_config_watch(app);
            info!("tray service started");
            Ok(())
        }

        fn spawn_config_watch(&self, _app: &AppHandle) {
            let mut rx = self.config_rx.clone();
            let i18n = Arc::clone(&self.i18n);
            let port = Arc::clone(&self.port);
            let tooltip_slot = Arc::clone(&self.current_tooltip);

            let handle = tokio::spawn(async move {
                loop {
                    if rx.changed().await.is_err() {
                        info!("tray config watch closed");
                        break;
                    }

                    let config = Arc::clone(&rx.borrow_and_update());
                    i18n.set_locale(&config.display.language);

                    // Inline the locale-change re-text here rather than
                    // routing through `TrayService::apply_locale_change` so
                    // the task only borrows the data it actually needs.
                    let tooltip = current_tooltip(&tooltip_slot);
                    let is_paused = tooltip.state == TimerState::Paused;
                    let pause_text = tray_tooltip::pause_menu_text(&i18n, is_paused);
                    if let Err(err) = port.set_menu_text(TrayMenuItemId::Pause, pause_text) {
                        warn!("failed to update pause item text: {err}");
                    }
                    if let Err(err) =
                        port.set_menu_text(TrayMenuItemId::Settings, i18n.get("tray.settings"))
                    {
                        warn!("failed to update settings item text: {err}");
                    }
                    if let Err(err) =
                        port.set_menu_text(TrayMenuItemId::About, i18n.get("tray.about"))
                    {
                        warn!("failed to update about item text: {err}");
                    }
                    if let Err(err) =
                        port.set_menu_text(TrayMenuItemId::Quit, i18n.get("tray.quit"))
                    {
                        warn!("failed to update quit item text: {err}");
                    }
                    if let Err(err) =
                        port.set_tooltip(&tray_tooltip::render_tooltip(&i18n, tooltip))
                    {
                        warn!("failed to update tray tooltip on locale change: {err}");
                    }
                }
            });

            match self.watch_handle.lock() {
                Ok(mut guard) => {
                    if let Some(existing) = guard.replace(handle) {
                        existing.abort();
                    }
                }
                Err(err) => {
                    warn!("tray watch handle mutex poisoned: {err}");
                    if let Some(existing) = err.into_inner().replace(handle) {
                        existing.abort();
                    }
                }
            }
        }

        // Helper for production code that wants to update the tooltip given a
        // raw `TrayTooltip` and ignore the result if it fails (only used by
        // `EffectSink` glue in `context.rs`).
        pub(crate) fn update_tooltip_best_effort(&self, tooltip: TrayTooltip) {
            if let Err(err) = self.update_tooltip(tooltip) {
                warn!("failed to update tray tooltip: {err}");
            }
        }

        pub(crate) fn update_pause_item_best_effort(&self, state: TimerState) {
            if let Err(err) = self.update_pause_item(state) {
                warn!("failed to update pause menu item: {err}");
            }
        }
    }
}

#[cfg(not(test))]
pub(crate) use runtime::TauriTrayPort;

// ---------------------------------------------------------------------------
// Tests use a recording fake port to validate orchestration without Tauri.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::Config;
    use crate::services::timer::effect::PomodoroProgress;
    use tokio::sync::watch;

    /// Recorded port operations. Ordering reflects production call order so
    /// tests can assert sequences like (`set_menu_text` Pause, `set_tooltip`).
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Call {
        InstallTray(TrayMenuLabelsKey),
        SetTooltip(String),
        SetMenuText(TrayMenuItemId, String),
        EmitToWindow(String, String, String),
        UnminimizeWindow(String),
        ShowWindow(String),
        FocusWindow(String),
        Quit,
    }

    /// Snapshot of `TrayMenuLabels` used for assertions; the real struct
    /// owns `String`s but `Eq` makes diagnostics nicer.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TrayMenuLabelsKey {
        pause: String,
        settings: String,
        about: String,
        quit: String,
        initial_tooltip: String,
    }

    impl From<TrayMenuLabels> for TrayMenuLabelsKey {
        fn from(value: TrayMenuLabels) -> Self {
            Self {
                pause: value.pause,
                settings: value.settings,
                about: value.about,
                quit: value.quit,
                initial_tooltip: value.initial_tooltip,
            }
        }
    }

    /// Operations the fake port should fail on. Stored as a set so the
    /// fake can stay below clippy's struct-bool ceiling while still
    /// letting each test opt in independently.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum FailOp {
        Install,
        Tooltip,
        Emit,
        Show,
        Focus,
    }

    #[derive(Debug, Default, Clone)]
    struct FakePortErrors {
        ops: std::collections::HashSet<FailOp>,
        menu_text: Option<TrayMenuItemId>,
    }

    impl FakePortErrors {
        fn fails(&self, op: FailOp) -> bool {
            self.ops.contains(&op)
        }
    }

    struct FakeTrayPort {
        calls: Mutex<Vec<Call>>,
        errors: FakePortErrors,
    }

    impl FakeTrayPort {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                errors: FakePortErrors::default(),
            }
        }

        fn with_install_err(mut self) -> Self {
            self.errors.ops.insert(FailOp::Install);
            self
        }

        fn with_tooltip_err(mut self) -> Self {
            self.errors.ops.insert(FailOp::Tooltip);
            self
        }

        fn with_menu_text_err(mut self, item: TrayMenuItemId) -> Self {
            self.errors.menu_text = Some(item);
            self
        }

        fn with_emit_err(mut self) -> Self {
            self.errors.ops.insert(FailOp::Emit);
            self
        }

        fn with_show_err(mut self) -> Self {
            self.errors.ops.insert(FailOp::Show);
            self
        }

        fn with_focus_err(mut self) -> Self {
            self.errors.ops.insert(FailOp::Focus);
            self
        }

        fn calls(&self) -> Vec<Call> {
            self.calls.lock().unwrap().clone()
        }

        fn err() -> crate::error::AppError {
            crate::error::AppError::IoError {
                message: "fake tray port error".to_string(),
            }
        }
    }

    impl TrayPort for FakeTrayPort {
        fn install_tray(&self, labels: TrayMenuLabels) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(Call::InstallTray(labels.into()));
            if self.errors.fails(FailOp::Install) {
                return Err(Self::err());
            }
            Ok(())
        }

        fn set_tooltip(&self, text: &str) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(Call::SetTooltip(text.to_string()));
            if self.errors.fails(FailOp::Tooltip) {
                return Err(Self::err());
            }
            Ok(())
        }

        fn set_menu_text(&self, item: TrayMenuItemId, text: &str) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(Call::SetMenuText(item, text.to_string()));
            if self.errors.menu_text == Some(item) {
                return Err(Self::err());
            }
            Ok(())
        }

        fn emit_to_window(&self, window: &str, event: &str, payload: &str) -> Result<()> {
            self.calls.lock().unwrap().push(Call::EmitToWindow(
                window.to_string(),
                event.to_string(),
                payload.to_string(),
            ));
            if self.errors.fails(FailOp::Emit) {
                return Err(Self::err());
            }
            Ok(())
        }

        fn unminimize_window(&self, label: &str) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(Call::UnminimizeWindow(label.to_string()));
            Ok(())
        }

        fn show_window(&self, label: &str) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(Call::ShowWindow(label.to_string()));
            if self.errors.fails(FailOp::Show) {
                return Err(Self::err());
            }
            Ok(())
        }

        fn focus_window(&self, label: &str) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(Call::FocusWindow(label.to_string()));
            if self.errors.fails(FailOp::Focus) {
                return Err(Self::err());
            }
            Ok(())
        }

        fn quit_app(&self) {
            self.calls.lock().unwrap().push(Call::Quit);
        }
    }

    fn make_service(
        port: Arc<FakeTrayPort>,
        locale: &str,
    ) -> (TrayService, watch::Sender<Arc<Config>>) {
        let mut config = Config::default();
        config.display.language = locale.to_string();
        let (tx, rx) = watch::channel(Arc::new(config));
        let i18n = Arc::new(I18nService::new(locale));
        let service = TrayService::new(port, rx, i18n);
        (service, tx)
    }

    #[test]
    fn on_menu_event_dispatches_known_ids() {
        assert_eq!(TrayService::on_menu_event("pause"), MenuAction::TogglePause);
        assert_eq!(
            TrayService::on_menu_event("settings"),
            MenuAction::OpenSettings
        );
        assert_eq!(TrayService::on_menu_event("about"), MenuAction::OpenAbout);
        assert_eq!(TrayService::on_menu_event("quit"), MenuAction::Quit);
    }

    #[test]
    fn on_menu_event_falls_back_to_unknown_for_unrecognised_id() {
        assert_eq!(
            TrayService::on_menu_event("garbage"),
            MenuAction::Unknown("garbage".to_string())
        );
    }

    #[test]
    fn tray_menu_item_id_as_str_is_stable() {
        // Production tray-icon wiring uses these strings as menu-item IDs;
        // changing them silently would break the on-click dispatch.
        assert_eq!(TrayMenuItemId::Pause.as_str(), "pause");
        assert_eq!(TrayMenuItemId::Settings.as_str(), "settings");
        assert_eq!(TrayMenuItemId::About.as_str(), "about");
        assert_eq!(TrayMenuItemId::Quit.as_str(), "quit");
    }

    #[test]
    fn initial_labels_uses_current_locale() {
        let port = Arc::new(FakeTrayPort::new());
        let (service, _tx) = make_service(port, "en");
        let labels = service.initial_labels();
        assert_eq!(labels.pause, "Pause");
        assert_eq!(labels.settings, "Settings");
        assert_eq!(labels.about, "About");
        assert_eq!(labels.quit, "Quit");
        assert!(
            labels.initial_tooltip.contains("Working"),
            "initial tooltip should reflect default Working state: {}",
            labels.initial_tooltip
        );
    }

    #[test]
    fn update_tooltip_pushes_rendered_text_to_port() {
        let port = Arc::new(FakeTrayPort::new());
        let (service, _tx) = make_service(port.clone(), "en");

        service
            .update_tooltip(TrayTooltip {
                state: TimerState::Working,
                remaining_secs: Some(20 * 60),
                pomodoro_progress: None,
            })
            .expect("happy path");

        let calls = port.calls();
        assert_eq!(calls.len(), 1);
        match &calls[0] {
            Call::SetTooltip(text) => {
                assert!(text.contains("20:00"));
                assert!(text.contains("Working"));
            }
            other => panic!("expected SetTooltip, got {other:?}"),
        }
    }

    #[test]
    fn update_tooltip_propagates_port_error() {
        let port = Arc::new(FakeTrayPort::new().with_tooltip_err());
        let (service, _tx) = make_service(port, "en");

        let result = service.update_tooltip(TrayTooltip {
            state: TimerState::Resting,
            remaining_secs: Some(20),
            pomodoro_progress: None,
        });

        assert!(result.is_err(), "tooltip failure should propagate");
    }

    #[test]
    fn update_tooltip_includes_pomodoro_suffix() {
        let port = Arc::new(FakeTrayPort::new());
        let (service, _tx) = make_service(port.clone(), "en");

        service
            .update_tooltip(TrayTooltip {
                state: TimerState::Working,
                remaining_secs: Some(25 * 60),
                pomodoro_progress: Some(PomodoroProgress {
                    current: 2,
                    total: 4,
                }),
            })
            .unwrap();

        match port.calls().pop().expect("at least one call") {
            Call::SetTooltip(text) => {
                assert!(
                    text.contains("(Pomo 2/4)"),
                    "missing pomodoro suffix: {text}"
                );
                assert!(text.contains("25:00"));
            }
            other => panic!("expected SetTooltip, got {other:?}"),
        }
    }

    #[test]
    fn update_pause_item_flips_label_between_pause_and_resume() {
        let port = Arc::new(FakeTrayPort::new());
        let (service, _tx) = make_service(port.clone(), "en");

        service.update_pause_item(TimerState::Paused).unwrap();
        service.update_pause_item(TimerState::Working).unwrap();

        let menu_text_calls: Vec<(TrayMenuItemId, String)> = port
            .calls()
            .into_iter()
            .filter_map(|c| match c {
                Call::SetMenuText(item, text) => Some((item, text)),
                _ => None,
            })
            .collect();

        assert_eq!(menu_text_calls.len(), 2);
        assert_eq!(menu_text_calls[0].0, TrayMenuItemId::Pause);
        assert_eq!(menu_text_calls[0].1, "Resume");
        assert_eq!(menu_text_calls[1].0, TrayMenuItemId::Pause);
        assert_eq!(menu_text_calls[1].1, "Pause");
    }

    #[test]
    fn update_pause_item_propagates_port_error() {
        let port = Arc::new(FakeTrayPort::new().with_menu_text_err(TrayMenuItemId::Pause));
        let (service, _tx) = make_service(port, "en");

        let result = service.update_pause_item(TimerState::Paused);
        assert!(
            result.is_err(),
            "pause menu set_text failure must propagate"
        );
    }

    #[test]
    fn apply_locale_change_retexts_every_item_and_tooltip() {
        let port = Arc::new(FakeTrayPort::new());
        let (service, _tx) = make_service(port.clone(), "en");

        service.apply_locale_change().expect("happy path");

        let calls = port.calls();
        // Expect: 4 set_menu_text + 1 set_tooltip
        let set_menu_text: Vec<TrayMenuItemId> = calls
            .iter()
            .filter_map(|c| match c {
                Call::SetMenuText(item, _) => Some(*item),
                _ => None,
            })
            .collect();
        assert_eq!(
            set_menu_text,
            vec![
                TrayMenuItemId::Pause,
                TrayMenuItemId::Settings,
                TrayMenuItemId::About,
                TrayMenuItemId::Quit
            ]
        );
        assert!(matches!(calls.last(), Some(Call::SetTooltip(_))));
    }

    #[test]
    fn apply_locale_change_continues_after_individual_failure() {
        let port = Arc::new(FakeTrayPort::new().with_menu_text_err(TrayMenuItemId::Settings));
        let (service, _tx) = make_service(port.clone(), "en");

        let result = service.apply_locale_change();
        assert!(result.is_err(), "first error must surface");
        let calls = port.calls();
        // Even after Settings fails, About + Quit should still be attempted
        // (and so should the tooltip).
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, Call::SetMenuText(TrayMenuItemId::About, _))),
            "About item must still be attempted after Settings fails"
        );
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, Call::SetMenuText(TrayMenuItemId::Quit, _))),
            "Quit item must still be attempted after Settings fails"
        );
        assert!(
            calls.iter().any(|c| matches!(c, Call::SetTooltip(_))),
            "tooltip must still be attempted even after a menu item fails"
        );
    }

    #[test]
    fn show_main_window_with_tab_runs_full_sequence_on_happy_path() {
        let port = Arc::new(FakeTrayPort::new());
        let (service, _tx) = make_service(port.clone(), "en");

        service.show_main_window_with_tab("Settings").unwrap();

        let calls = port.calls();
        assert_eq!(
            calls,
            vec![
                Call::EmitToWindow(
                    "main-window".to_string(),
                    events::NAVIGATE_TAB.to_string(),
                    "Settings".to_string()
                ),
                Call::UnminimizeWindow("main-window".to_string()),
                Call::ShowWindow("main-window".to_string()),
                Call::FocusWindow("main-window".to_string()),
            ]
        );
    }

    #[test]
    fn show_main_window_propagates_emit_error_and_short_circuits() {
        let port = Arc::new(FakeTrayPort::new().with_emit_err());
        let (service, _tx) = make_service(port.clone(), "en");

        let result = service.show_main_window_with_tab("Settings");
        assert!(result.is_err());
        let calls = port.calls();
        // Emit should be attempted; unminimize/show/focus should NOT run after
        // emit fails (we short-circuit on emit failure).
        assert!(matches!(calls[0], Call::EmitToWindow(_, _, _)));
        assert!(
            !calls
                .iter()
                .any(|c| matches!(c, Call::ShowWindow(_) | Call::FocusWindow(_))),
            "show/focus must not run after emit fails: {calls:?}"
        );
    }

    #[test]
    fn show_main_window_propagates_show_error() {
        let port = Arc::new(FakeTrayPort::new().with_show_err());
        let (service, _tx) = make_service(port.clone(), "en");

        let result = service.show_main_window_with_tab("About");
        assert!(result.is_err());
        let calls = port.calls();
        assert!(
            !calls.iter().any(|c| matches!(c, Call::FocusWindow(_))),
            "focus must not run after show fails: {calls:?}"
        );
    }

    #[test]
    fn show_main_window_propagates_focus_error() {
        let port = Arc::new(FakeTrayPort::new().with_focus_err());
        let (service, _tx) = make_service(port.clone(), "en");

        let result = service.show_main_window_with_tab("About");
        assert!(result.is_err());
    }

    #[test]
    fn show_main_window_continues_when_unminimize_fails() {
        // Unminimize commonly fails when window is already restored; we
        // should NOT bail in that case — show + focus must still run.
        struct UnminimizeFailPort {
            inner: FakeTrayPort,
        }
        impl TrayPort for UnminimizeFailPort {
            fn install_tray(&self, l: TrayMenuLabels) -> Result<()> {
                self.inner.install_tray(l)
            }
            fn set_tooltip(&self, t: &str) -> Result<()> {
                self.inner.set_tooltip(t)
            }
            fn set_menu_text(&self, i: TrayMenuItemId, t: &str) -> Result<()> {
                self.inner.set_menu_text(i, t)
            }
            fn emit_to_window(&self, w: &str, e: &str, p: &str) -> Result<()> {
                self.inner.emit_to_window(w, e, p)
            }
            fn unminimize_window(&self, label: &str) -> Result<()> {
                self.inner
                    .calls
                    .lock()
                    .unwrap()
                    .push(Call::UnminimizeWindow(label.to_string()));
                Err(FakeTrayPort::err())
            }
            fn show_window(&self, l: &str) -> Result<()> {
                self.inner.show_window(l)
            }
            fn focus_window(&self, l: &str) -> Result<()> {
                self.inner.focus_window(l)
            }
            fn quit_app(&self) {
                self.inner.quit_app();
            }
        }
        let port = Arc::new(UnminimizeFailPort {
            inner: FakeTrayPort::new(),
        });
        let mut config = Config::default();
        config.display.language = "en".to_string();
        let (_tx, rx) = watch::channel(Arc::new(config));
        let i18n = Arc::new(I18nService::new("en"));
        let service = TrayService::new(port.clone(), rx, i18n);

        service.show_main_window_with_tab("Settings").unwrap();

        let calls = port.inner.calls();
        assert!(
            calls.iter().any(|c| matches!(c, Call::ShowWindow(_))),
            "show must still run after unminimize fails"
        );
        assert!(
            calls.iter().any(|c| matches!(c, Call::FocusWindow(_))),
            "focus must still run after unminimize fails"
        );
    }

    #[test]
    fn quit_routes_to_port_quit_app() {
        let port = Arc::new(FakeTrayPort::new());
        let (service, _tx) = make_service(port.clone(), "en");

        service.quit();

        assert_eq!(port.calls(), vec![Call::Quit]);
    }

    #[test]
    fn start_with_initial_config_applies_locale_installs_tray_and_sets_tooltip() {
        let port = Arc::new(FakeTrayPort::new());
        let (service, _tx) = make_service(port.clone(), "zh-CN");

        let mut config = Config::default();
        config.display.language = "en".to_string();
        config.timer.work_minutes = 30;
        service.start_with_initial_config(&config).unwrap();

        let calls = port.calls();
        // Expect: install_tray first, then set_tooltip
        assert!(
            matches!(calls.first(), Some(Call::InstallTray(_))),
            "first call must be install_tray, got: {calls:?}"
        );
        match calls.first() {
            Some(Call::InstallTray(labels)) => {
                // Locale switched to en in the process
                assert_eq!(labels.pause, "Pause");
                assert_eq!(labels.settings, "Settings");
            }
            _ => unreachable!(),
        }
        assert!(
            calls
                .iter()
                .any(|c| matches!(c, Call::SetTooltip(t) if t.contains("30:00"))),
            "tooltip should reflect 30-minute work duration: {calls:?}"
        );
    }

    #[test]
    fn start_with_initial_config_propagates_install_failure() {
        let port = Arc::new(FakeTrayPort::new().with_install_err());
        let (service, _tx) = make_service(port, "en");
        let config = Config::default();
        assert!(service.start_with_initial_config(&config).is_err());
    }

    #[tokio::test]
    async fn shutdown_handles_no_running_watch() {
        let port = Arc::new(FakeTrayPort::new());
        let (service, _tx) = make_service(port, "en");
        // No watch ever spawned; shutdown must still succeed.
        service.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn init_and_start_lifecycle_no_op_without_runtime_context() {
        let port = Arc::new(FakeTrayPort::new());
        let (service, _tx) = make_service(port.clone(), "en");
        let ctx = ServiceContext::default();
        service.init(&ctx).await.unwrap();
        service.start(&ctx).await.unwrap();
        // Without an app handle, neither install_tray nor any other port
        // method runs from the Service-trait lifecycle.
        assert!(
            port.calls().is_empty(),
            "Service::init/start must not call port without runtime ctx: {:?}",
            port.calls()
        );
    }
}
