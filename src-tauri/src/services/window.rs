#![allow(clippy::module_name_repetitions)]

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
use tracing::{error, info, warn};

use crate::error::Result;
use crate::services::{Service, ServiceContext};

/// Manages tip-window lifecycle across all monitors.
pub(crate) struct WindowService;

impl WindowService {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Create fullscreen tip windows on every monitor.
    #[allow(clippy::unused_self)]
    pub(crate) fn show_tip_windows(&self, app: &AppHandle) {
        let monitors = match app.available_monitors() {
            Ok(monitors) => monitors,
            Err(err) => {
                error!("failed to enumerate monitors, no tip windows shown: {err}");
                return;
            }
        };

        if monitors.is_empty() {
            error!("no monitors detected, no tip windows shown");
            return;
        }

        let primary = app.primary_monitor().ok().flatten();

        for (index, monitor) in monitors.iter().enumerate() {
            let is_primary = primary.as_ref().map_or(index == 0, |primary_monitor| {
                primary_monitor.name() == monitor.name()
            });

            let label = if is_primary {
                "tip-window-0".to_string()
            } else {
                format!("tip-window-minimal-{index}")
            };

            if app.get_webview_window(&label).is_some() {
                info!("window {label} already exists, skipping creation");
                continue;
            }

            let url = if is_primary {
                "tip.html"
            } else {
                "tip-minimal.html"
            };

            let position = monitor.position();
            let size = monitor.size();

            let result = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(url.into()))
                .title("Eyezen")
                .position(f64::from(position.x), f64::from(position.y))
                .inner_size(f64::from(size.width), f64::from(size.height))
                .visible_on_all_workspaces(true)
                .always_on_top(true)
                .decorations(false)
                .skip_taskbar(true)
                .fullscreen(true)
                .focused(is_primary)
                .build();

            match result {
                Ok(_) => info!("created {label} on monitor {:?}", monitor.name()),
                Err(err) => {
                    // Fullscreen builder can fail on some Linux compositors and on
                    // Windows when the target monitor was just disconnected; retry
                    // without the fullscreen flag so the user still gets a maximized
                    // tip window instead of silent skip.
                    warn!("failed to create fullscreen window {label}, retrying maximized: {err}");
                    Self::create_maximized_window(app, &label, url, *position, *size, is_primary);
                }
            }
        }
    }

    /// Close every dynamically created tip window.
    #[allow(clippy::unused_self)]
    pub(crate) fn hide_tip_windows(&self, app: &AppHandle) {
        let labels_to_close: Vec<String> = app
            .webview_windows()
            .keys()
            .filter(|label| label.starts_with("tip-window"))
            .cloned()
            .collect();

        if labels_to_close.is_empty() {
            info!("no tip windows to close");
            return;
        }

        for label in &labels_to_close {
            if let Some(window) = app.get_webview_window(label) {
                if let Err(err) = window.close() {
                    warn!("failed to close {label}: {err}");
                } else {
                    info!("closed {label}");
                }
            }
        }
    }

    fn create_maximized_window(
        app: &AppHandle,
        label: &str,
        url: &str,
        position: PhysicalPosition<i32>,
        size: PhysicalSize<u32>,
        focused: bool,
    ) {
        let result = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
            .title("Eyezen")
            .position(f64::from(position.x), f64::from(position.y))
            .inner_size(f64::from(size.width), f64::from(size.height))
            .visible_on_all_workspaces(true)
            .always_on_top(true)
            .decorations(false)
            .skip_taskbar(true)
            .maximized(true)
            .focused(focused)
            .build();

        if let Err(err) = result {
            error!("failed to create maximized window {label}: {err}");
        } else {
            info!("created maximized window {label}");
        }
    }
}

impl Default for WindowService {
    fn default() -> Self {
        Self::new()
    }
}

impl Service for WindowService {
    async fn init(&self, app: &ServiceContext) -> Result<()> {
        if app.app_handle().is_none() {
            info!("window service initialized without runtime context");
            return Ok(());
        }

        info!("window service initialized");
        Ok(())
    }

    async fn start(&self, _app: &ServiceContext) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        info!("window service shutdown");
        Ok(())
    }
}
