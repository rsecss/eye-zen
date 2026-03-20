#![allow(clippy::module_name_repetitions)]

use std::sync::{Arc, Mutex};

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Wry,
};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::error::{AppError, Result};
use crate::models::config::Config;
use crate::services::i18n::I18nService;
use crate::services::timer::{TimerState, TrayTooltip, UserEvent};
use crate::services::{Service, ServiceContext, SharedAppServices};

#[derive(Clone, Default)]
struct MenuItems {
    pause: Option<MenuItem<Wry>>,
    settings: Option<MenuItem<Wry>>,
    about: Option<MenuItem<Wry>>,
    quit: Option<MenuItem<Wry>>,
}

pub(crate) struct TrayService {
    #[cfg_attr(test, allow(dead_code))]
    config_rx: watch::Receiver<Arc<Config>>,
    i18n: Arc<I18nService>,
    menu_items: Arc<Mutex<MenuItems>>,
    current_tooltip: Arc<Mutex<TrayTooltip>>,
    watch_handle: Mutex<Option<JoinHandle<()>>>,
}

impl TrayService {
    #[must_use]
    pub(crate) fn new(config_rx: watch::Receiver<Arc<Config>>, i18n: Arc<I18nService>) -> Self {
        Self {
            config_rx,
            i18n,
            menu_items: Arc::new(Mutex::new(MenuItems::default())),
            current_tooltip: Arc::new(Mutex::new(TrayTooltip {
                state: TimerState::Working,
                remaining_secs: Some(20 * 60),
            })),
            watch_handle: Mutex::new(None),
        }
    }

    /// Create the single MVP tray icon.
    ///
    /// # Errors
    ///
    /// Returns an error when tray menu items or the tray icon cannot be created.
    pub(crate) fn create_tray(&self, app: &AppHandle) -> Result<()> {
        let pause_item = MenuItem::with_id(
            app,
            "pause",
            self.i18n.get("tray.pause"),
            true,
            None::<&str>,
        )
        .map_err(Self::io_error)?;
        let settings_item = MenuItem::with_id(
            app,
            "settings",
            self.i18n.get("tray.settings"),
            true,
            None::<&str>,
        )
        .map_err(Self::io_error)?;
        let about_item = MenuItem::with_id(
            app,
            "about",
            self.i18n.get("tray.about"),
            true,
            None::<&str>,
        )
        .map_err(Self::io_error)?;
        let quit_item =
            MenuItem::with_id(app, "quit", self.i18n.get("tray.quit"), true, None::<&str>)
                .map_err(Self::io_error)?;

        let menu = Menu::with_items(app, &[&pause_item, &settings_item, &about_item, &quit_item])
            .map_err(Self::io_error)?;

        Self::store_menu_items(
            &self.menu_items,
            MenuItems {
                pause: Some(pause_item.clone()),
                settings: Some(settings_item.clone()),
                about: Some(about_item.clone()),
                quit: Some(quit_item.clone()),
            },
        );

        let tooltip = Self::current_tooltip(&self.current_tooltip);
        let app_for_menu = app.clone();

        TrayIconBuilder::with_id("main")
            .tooltip(self.render_tooltip_text(tooltip))
            .menu(&menu)
            .on_menu_event(move |_app, event| {
                Self::handle_menu_event(&app_for_menu, event.id().as_ref());
            })
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    rect,
                    ..
                } = event
                {
                    Self::toggle_tray_panel(tray.app_handle(), &rect);
                }
            })
            .build(app)
            .map_err(Self::io_error)?;

        Ok(())
    }

    pub(crate) fn handle_menu_event(app: &AppHandle, id: &str) {
        match id {
            "pause" => {
                info!("tray: pause/resume clicked");
                let services = app.state::<SharedAppServices>().inner().clone();
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
            "settings" => {
                info!("tray: settings clicked");
                Self::show_main_window(app, "Settings");
            }
            "about" => {
                info!("tray: about clicked");
                Self::show_main_window(app, "About");
            }
            "quit" => {
                info!("tray: quit clicked");
                app.exit(0);
            }
            _ => warn!("tray: unknown menu item: {id}"),
        }
    }

    pub(crate) fn toggle_tray_panel(app: &AppHandle, icon_rect: &tauri::Rect) {
        if let Some(panel) = app.get_webview_window("tray-panel") {
            match panel.is_visible() {
                Ok(true) => {
                    if let Err(err) = panel.hide() {
                        warn!("failed to hide tray panel: {err}");
                    }
                }
                Ok(false) => {
                    Self::position_near_tray(&panel, icon_rect);
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
        } else {
            warn!("tray-panel window not found");
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
                    .unwrap_or((0.0, 0.0, 1920.0, 1080.0))
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

        if let Err(err) = panel.set_position(tauri::PhysicalPosition::new(x as i32, y as i32)) {
            warn!("failed to position tray panel: {err}");
        }
    }

    pub(crate) fn update_tooltip(&self, app: &AppHandle, tooltip: TrayTooltip) {
        Self::store_tooltip(&self.current_tooltip, tooltip);

        if let Some(tray) = app.tray_by_id("main") {
            if let Err(err) = tray.set_tooltip(Some(self.render_tooltip_text(tooltip))) {
                warn!("failed to update tray tooltip: {err}");
            }
        } else {
            warn!("tray icon 'main' not found");
        }
    }

    pub(crate) fn update_pause_item(&self, state: TimerState) {
        let is_paused = state == TimerState::Paused;
        let mut tooltip = Self::current_tooltip(&self.current_tooltip);
        tooltip.state = state;
        Self::store_tooltip(&self.current_tooltip, tooltip);

        let text = if is_paused {
            self.i18n.get("tray.resume")
        } else {
            self.i18n.get("tray.pause")
        };

        let items = Self::menu_items_snapshot(&self.menu_items);
        if let Some(item) = items.pause {
            if let Err(err) = item.set_text(text) {
                warn!("failed to update pause item text: {err}");
            }
        }
    }

    fn spawn_config_watch(&self, app: &AppHandle) {
        let mut rx = self.config_rx.clone();
        let app_handle = app.clone();
        let i18n = Arc::clone(&self.i18n);
        let menu_items = Arc::clone(&self.menu_items);
        let current_tooltip = Arc::clone(&self.current_tooltip);

        let handle = tokio::spawn(async move {
            loop {
                if rx.changed().await.is_err() {
                    info!("tray config watch closed");
                    break;
                }

                let config = Arc::clone(&rx.borrow_and_update());
                i18n.set_locale(&config.display.language);

                let tooltip = Self::current_tooltip(&current_tooltip);
                let is_paused = tooltip.state == TimerState::Paused;
                Self::apply_menu_texts(&menu_items, &i18n, is_paused);
                Self::apply_tooltip(&app_handle, &i18n, tooltip);
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

    fn render_tooltip_text(&self, tooltip: TrayTooltip) -> String {
        Self::render_tooltip(&self.i18n, tooltip)
    }

    fn render_tooltip(i18n: &I18nService, tooltip: TrayTooltip) -> String {
        let app_name = i18n.get("tray.tooltip.app_name");
        let label = match tooltip.state {
            TimerState::Working => i18n.get("tray.tooltip.working"),
            TimerState::PreAlert => i18n.get("tray.tooltip.pre_alert"),
            TimerState::Alerting => i18n.get("tray.tooltip.alerting"),
            TimerState::Resting => i18n.get("tray.tooltip.resting"),
            TimerState::Paused => i18n.get("tray.tooltip.paused"),
        };

        match tooltip.remaining_secs {
            Some(remaining_secs) => format!(
                "{app_name} - {label} {}",
                Self::format_remaining(remaining_secs)
            ),
            None => format!("{app_name} - {label}"),
        }
    }

    fn format_remaining(total_secs: u32) -> String {
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{minutes:02}:{seconds:02}")
    }

    fn apply_tooltip(app: &AppHandle, i18n: &I18nService, tooltip: TrayTooltip) {
        if let Some(tray) = app.tray_by_id("main") {
            if let Err(err) = tray.set_tooltip(Some(Self::render_tooltip(i18n, tooltip))) {
                warn!("failed to update tray tooltip: {err}");
            }
        }
    }

    fn apply_menu_texts(menu_items: &Arc<Mutex<MenuItems>>, i18n: &I18nService, is_paused: bool) {
        let items = Self::menu_items_snapshot(menu_items);

        if let Some(item) = items.pause {
            let text = if is_paused {
                i18n.get("tray.resume")
            } else {
                i18n.get("tray.pause")
            };
            if let Err(err) = item.set_text(text) {
                warn!("failed to update pause item text: {err}");
            }
        }
        if let Some(item) = items.settings {
            if let Err(err) = item.set_text(i18n.get("tray.settings")) {
                warn!("failed to update settings item text: {err}");
            }
        }
        if let Some(item) = items.about {
            if let Err(err) = item.set_text(i18n.get("tray.about")) {
                warn!("failed to update about item text: {err}");
            }
        }
        if let Some(item) = items.quit {
            if let Err(err) = item.set_text(i18n.get("tray.quit")) {
                warn!("failed to update quit item text: {err}");
            }
        }
    }

    fn menu_items_snapshot(menu_items: &Arc<Mutex<MenuItems>>) -> MenuItems {
        match menu_items.lock() {
            Ok(guard) => guard.clone(),
            Err(err) => {
                warn!("tray menu items mutex poisoned: {err}");
                err.into_inner().clone()
            }
        }
    }

    fn current_tooltip(current_tooltip: &Arc<Mutex<TrayTooltip>>) -> TrayTooltip {
        match current_tooltip.lock() {
            Ok(guard) => *guard,
            Err(err) => {
                warn!("tray tooltip mutex poisoned: {err}");
                *err.into_inner()
            }
        }
    }

    fn store_menu_items(menu_items: &Arc<Mutex<MenuItems>>, next: MenuItems) {
        match menu_items.lock() {
            Ok(mut guard) => {
                *guard = next;
            }
            Err(err) => {
                warn!("tray menu items mutex poisoned while storing: {err}");
                *err.into_inner() = next;
            }
        }
    }

    fn store_tooltip(current_tooltip: &Arc<Mutex<TrayTooltip>>, next: TrayTooltip) {
        match current_tooltip.lock() {
            Ok(mut guard) => {
                *guard = next;
            }
            Err(err) => {
                warn!("tray tooltip mutex poisoned while storing: {err}");
                *err.into_inner() = next;
            }
        }
    }

    fn show_main_window(app: &AppHandle, tab: &str) {
        if let Some(window) = app.get_webview_window("main-window") {
            let _ = window.emit("navigate_tab", tab);
            let _ = window.unminimize();
            if let Err(err) = window.show() {
                warn!("failed to show main window: {err}");
                return;
            }
            if let Err(err) = window.set_focus() {
                warn!("failed to focus main window: {err}");
            }
        } else {
            warn!("main-window not found");
        }
    }

    fn io_error(err: impl std::fmt::Display) -> AppError {
        AppError::IoError {
            message: err.to_string(),
        }
    }
}

impl Service for TrayService {
    async fn init(&self, app: &ServiceContext) -> Result<()> {
        let Some(app_handle) = app.app_handle() else {
            info!("tray service initialized without runtime context");
            return Ok(());
        };

        if app_handle.tray_by_id("main").is_some() {
            info!("tray service already initialized");
        }
        Ok(())
    }

    async fn start(&self, app: &ServiceContext) -> Result<()> {
        let Some(app_handle) = app.app_handle() else {
            info!("tray service started without runtime context");
            return Ok(());
        };

        let mut config_rx = self.config_rx.clone();
        let config = Arc::clone(&config_rx.borrow_and_update());
        self.i18n.set_locale(&config.display.language);
        Self::store_tooltip(
            &self.current_tooltip,
            TrayTooltip {
                state: TimerState::Working,
                remaining_secs: Some(config.timer.work_minutes.saturating_mul(60)),
            },
        );

        self.create_tray(&app_handle)?;
        self.update_tooltip(&app_handle, Self::current_tooltip(&self.current_tooltip));
        self.spawn_config_watch(&app_handle);

        info!("tray service started");
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
