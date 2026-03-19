#![allow(clippy::module_name_repetitions)]

use std::sync::Arc;
use std::sync::Mutex;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};
use tokio::sync::watch;
use tracing::{info, warn};

use crate::error::{AppError, Result};
use crate::models::config::Config;
use crate::services::timer::UserEvent;
use crate::services::{Service, ServiceContext, SharedAppServices};

pub(crate) struct TrayService {
    #[cfg_attr(test, allow(dead_code))]
    config_rx: watch::Receiver<Arc<Config>>,
    pause_item: Mutex<Option<MenuItem<Wry>>>,
}

impl TrayService {
    #[must_use]
    pub(crate) fn new(config_rx: watch::Receiver<Arc<Config>>) -> Self {
        Self {
            config_rx,
            pause_item: Mutex::new(None),
        }
    }

    /// Create the single MVP tray icon.
    ///
    /// # Errors
    ///
    /// Returns an error when tray menu items or the tray icon cannot be created.
    pub(crate) fn create_tray(&self, app: &AppHandle) -> Result<()> {
        let pause_item =
            MenuItem::with_id(app, "pause", "Pause", true, None::<&str>).map_err(Self::io_error)?;
        let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)
            .map_err(Self::io_error)?;
        let about_item =
            MenuItem::with_id(app, "about", "About", true, None::<&str>).map_err(Self::io_error)?;
        let quit_item =
            MenuItem::with_id(app, "quit", "Quit", true, None::<&str>).map_err(Self::io_error)?;

        let menu = Menu::with_items(app, &[&pause_item, &settings_item, &about_item, &quit_item])
            .map_err(Self::io_error)?;

        if let Ok(mut item) = self.pause_item.lock() {
            *item = Some(pause_item.clone());
        }

        let app_for_menu = app.clone();

        TrayIconBuilder::with_id("main")
            .tooltip("Eyezen - Working")
            .menu(&menu)
            .on_menu_event(move |_app, event| {
                Self::handle_menu_event(&app_for_menu, event.id().as_ref());
            })
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    Self::toggle_tray_panel(tray.app_handle());
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
                Self::show_main_window(app);
            }
            "about" => {
                info!("tray: about clicked");
                Self::show_main_window(app);
            }
            "quit" => {
                info!("tray: quit clicked");
                app.exit(0);
            }
            _ => warn!("tray: unknown menu item: {id}"),
        }
    }

    pub(crate) fn toggle_tray_panel(app: &AppHandle) {
        if let Some(panel) = app.get_webview_window("tray-panel") {
            match panel.is_visible() {
                Ok(true) => {
                    if let Err(err) = panel.hide() {
                        warn!("failed to hide tray panel: {err}");
                    }
                }
                Ok(false) => {
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

    pub(crate) fn update_tooltip(&self, app: &AppHandle, text: &str) {
        if let Some(tray) = app.tray_by_id("main") {
            if let Err(err) = tray.set_tooltip(Some(text)) {
                warn!("failed to update tray tooltip: {err}");
            }
        } else {
            warn!("tray icon 'main' not found");
        }
    }

    pub(crate) fn update_pause_item(&self, app: &AppHandle, is_paused: bool) {
        let text = if is_paused { "Resume" } else { "Pause" };
        let _ = app;

        if let Ok(guard) = self.pause_item.lock() {
            if let Some(item) = guard.as_ref() {
                if let Err(err) = item.set_text(text) {
                    warn!("failed to update pause item text: {err}");
                }
            }
        }
    }

    fn show_main_window(app: &AppHandle) {
        if let Some(window) = app.get_webview_window("main-window") {
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

        self.create_tray(&app_handle)?;
        let config = self.config_rx.borrow();
        self.update_tooltip(
            &app_handle,
            &format!("Eyezen - {} min", config.timer.work_minutes),
        );
        info!("tray service started");
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        info!("tray service shutdown");
        Ok(())
    }
}
