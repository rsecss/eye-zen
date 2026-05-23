#![allow(clippy::module_name_repetitions)]

//! `WindowService` manages tip-window lifecycle across all monitors.
//!
//! Tauri's runtime APIs (`AppHandle::available_monitors`, `WebviewWindowBuilder`,
//! `WebviewWindow::close`) are wrapped behind the [`WindowPort`] trait so the
//! orchestration logic — primary-monitor selection, label scheme, fullscreen →
//! maximized retry on failure — can be exercised in the test build with a fake
//! port. The production adapter [`TauriWindowPort`] is compiled only in non-test
//! builds.

use std::sync::Arc;

use tracing::{error, info, warn};

use crate::error::Result;
use crate::services::{Service, ServiceContext};

use super::window_layout;

/// Monitor identity + geometry used by the tip-window layout helpers.
///
/// Mirrors the subset of `tauri::Monitor` that the orchestration actually
/// needs: a stable name (or `None` for unnamed monitors), top-left position,
/// and physical size. Keeping it as a plain data struct lets fakes construct
/// monitors without touching any Tauri types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MonitorInfo {
    pub(crate) name: Option<String>,
    pub(crate) position_x: i32,
    pub(crate) position_y: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// Inputs the port needs to build one tip window.
///
/// The orchestration code (`WindowService::show_tip_windows`) decides the
/// label, URL, geometry and whether to focus before calling
/// `WindowPort::create_tip_window`; the port itself only knows how to talk to
/// Tauri's window builder.
#[derive(Debug, Clone)]
pub(crate) struct TipWindowSpec {
    pub(crate) label: String,
    pub(crate) url: &'static str,
    pub(crate) position_x: i32,
    pub(crate) position_y: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) focused: bool,
}

/// Tauri-side window operations the tip-window orchestration depends on.
///
/// Each method maps one-to-one to a Tauri API call so the orchestration code
/// can be tested with a recording fake without compiling any Tauri runtime
/// dependencies.
pub(crate) trait WindowPort: Send + Sync {
    /// Enumerate all monitors currently reported by the OS.
    fn list_monitors(&self) -> Result<Vec<MonitorInfo>>;

    /// Return the monitor flagged as primary, if the OS reports one.
    fn primary_monitor(&self) -> Option<MonitorInfo>;

    /// `true` when a webview window with the given label is already alive.
    fn webview_exists(&self, label: &str) -> bool;

    /// Create a fullscreen tip window using the supplied spec. The
    /// implementation is responsible for the fullscreen → maximized retry on
    /// failure (Linux compositor quirks, Windows monitor-disconnect race).
    fn create_tip_window(&self, spec: TipWindowSpec) -> Result<()>;

    /// Close the webview window with the given label (no-op if absent).
    fn close_window(&self, label: &str) -> Result<()>;

    /// List the labels of every currently open webview window. Used to find
    /// dynamically created tip windows for bulk-close.
    fn list_window_labels(&self) -> Vec<String>;
}

/// Manages tip-window lifecycle across all monitors.
pub(crate) struct WindowService {
    port: Arc<dyn WindowPort>,
}

impl WindowService {
    #[must_use]
    pub(crate) fn new(port: Arc<dyn WindowPort>) -> Self {
        Self { port }
    }

    /// Create fullscreen tip windows on every monitor.
    pub(crate) fn show_tip_windows(&self) {
        let monitors = match self.port.list_monitors() {
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

        let primary = self.port.primary_monitor();
        let primary_name: Option<window_layout::MonitorName<'_>> =
            primary.as_ref().map(|m| m.name.as_deref());

        for (index, monitor) in monitors.iter().enumerate() {
            let monitor_name: window_layout::MonitorName<'_> = monitor.name.as_deref();
            let is_primary = window_layout::is_primary_monitor(index, monitor_name, primary_name);

            let label = window_layout::tip_window_label(index, is_primary);

            if self.port.webview_exists(&label) {
                info!("window {label} already exists, skipping creation");
                continue;
            }

            let spec = TipWindowSpec {
                label: label.clone(),
                url: window_layout::tip_window_url(is_primary),
                position_x: monitor.position_x,
                position_y: monitor.position_y,
                width: monitor.width,
                height: monitor.height,
                focused: is_primary,
            };

            if let Err(err) = self.port.create_tip_window(spec) {
                // Builder errors here always surface real failures (we already
                // delegate Linux/Windows fullscreen retries inside the port);
                // log and continue so the rest of the monitors are still
                // attempted.
                warn!("failed to create tip window {label}: {err}");
            } else {
                info!("created {label} on monitor {:?}", monitor.name);
            }
        }
    }

    /// Close every dynamically created tip window.
    pub(crate) fn hide_tip_windows(&self) {
        let labels_to_close: Vec<String> = self
            .port
            .list_window_labels()
            .into_iter()
            .filter(|label| window_layout::is_tip_window_label(label))
            .collect();

        if labels_to_close.is_empty() {
            info!("no tip windows to close");
            return;
        }

        for label in &labels_to_close {
            if let Err(err) = self.port.close_window(label) {
                warn!("failed to close {label}: {err}");
            } else {
                info!("closed {label}");
            }
        }
    }
}

impl Service for WindowService {
    async fn init(&self, _app: &ServiceContext) -> Result<()> {
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

// ---------------------------------------------------------------------------
// Production adapter: bridges WindowPort to the real Tauri APIs. Compiled
// only outside the test build because it pulls in tauri::AppHandle and the
// WebviewWindowBuilder runtime types.
// ---------------------------------------------------------------------------

#[cfg(not(test))]
mod runtime {
    use super::{MonitorInfo, Result, TipWindowSpec, WindowPort};
    use tauri::{
        AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
    };
    use tracing::{error, info, warn};

    /// `WindowPort` impl that drives Tauri's real webview / monitor APIs.
    pub(crate) struct TauriWindowPort {
        app: AppHandle,
    }

    impl TauriWindowPort {
        #[must_use]
        pub(crate) const fn new(app: AppHandle) -> Self {
            Self { app }
        }

        fn monitor_to_info(monitor: &tauri::Monitor) -> MonitorInfo {
            let pos = monitor.position();
            let size = monitor.size();
            MonitorInfo {
                name: monitor.name().cloned(),
                position_x: pos.x,
                position_y: pos.y,
                width: size.width,
                height: size.height,
            }
        }

        fn build_fullscreen(&self, spec: &TipWindowSpec) -> Result<()> {
            let result =
                WebviewWindowBuilder::new(&self.app, &spec.label, WebviewUrl::App(spec.url.into()))
                    .title("Eyezen")
                    .position(f64::from(spec.position_x), f64::from(spec.position_y))
                    .inner_size(f64::from(spec.width), f64::from(spec.height))
                    .visible_on_all_workspaces(true)
                    .always_on_top(true)
                    .decorations(false)
                    .skip_taskbar(true)
                    .fullscreen(true)
                    .focused(spec.focused)
                    .build();

            result
                .map(|_| ())
                .map_err(|err| crate::error::AppError::IoError {
                    message: err.to_string(),
                })
        }

        fn build_maximized(&self, spec: &TipWindowSpec) -> Result<()> {
            let position = PhysicalPosition::new(spec.position_x, spec.position_y);
            let size = PhysicalSize::new(spec.width, spec.height);
            let result =
                WebviewWindowBuilder::new(&self.app, &spec.label, WebviewUrl::App(spec.url.into()))
                    .title("Eyezen")
                    .position(f64::from(position.x), f64::from(position.y))
                    .inner_size(f64::from(size.width), f64::from(size.height))
                    .visible_on_all_workspaces(true)
                    .always_on_top(true)
                    .decorations(false)
                    .skip_taskbar(true)
                    .maximized(true)
                    .focused(spec.focused)
                    .build();

            result
                .map(|_| ())
                .map_err(|err| crate::error::AppError::IoError {
                    message: err.to_string(),
                })
        }
    }

    impl WindowPort for TauriWindowPort {
        fn list_monitors(&self) -> Result<Vec<MonitorInfo>> {
            self.app
                .available_monitors()
                .map(|monitors| monitors.iter().map(Self::monitor_to_info).collect())
                .map_err(|err| crate::error::AppError::IoError {
                    message: err.to_string(),
                })
        }

        fn primary_monitor(&self) -> Option<MonitorInfo> {
            self.app
                .primary_monitor()
                .ok()
                .flatten()
                .as_ref()
                .map(Self::monitor_to_info)
        }

        fn webview_exists(&self, label: &str) -> bool {
            self.app.get_webview_window(label).is_some()
        }

        fn create_tip_window(&self, spec: TipWindowSpec) -> Result<()> {
            if let Err(err) = self.build_fullscreen(&spec) {
                // Fullscreen builder can fail on some Linux compositors and on
                // Windows when the target monitor was just disconnected; retry
                // without the fullscreen flag so the user still gets a
                // maximized tip window instead of silent skip.
                warn!(
                    "failed to create fullscreen window {}, retrying maximized: {err}",
                    spec.label
                );
                if let Err(retry_err) = self.build_maximized(&spec) {
                    error!(
                        "failed to create maximized window {}: {retry_err}",
                        spec.label
                    );
                    return Err(retry_err);
                }
                info!("created maximized window {}", spec.label);
            }
            Ok(())
        }

        fn close_window(&self, label: &str) -> Result<()> {
            let Some(window) = self.app.get_webview_window(label) else {
                return Ok(());
            };
            window
                .close()
                .map_err(|err| crate::error::AppError::IoError {
                    message: err.to_string(),
                })
        }

        fn list_window_labels(&self) -> Vec<String> {
            self.app.webview_windows().keys().cloned().collect()
        }
    }
}

#[cfg(not(test))]
pub(crate) use runtime::TauriWindowPort;

// ---------------------------------------------------------------------------
// Tests use a recording fake port to validate orchestration without Tauri.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Operations recorded by `FakeWindowPort` so tests can assert what the
    /// orchestration did, in order.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Call {
        ListMonitors,
        PrimaryMonitor,
        WebviewExists(String),
        CreateTipWindow(String),
        CloseWindow(String),
        ListWindowLabels,
    }

    struct FakeWindowPort {
        monitors: Vec<MonitorInfo>,
        primary: Option<MonitorInfo>,
        existing_labels: Mutex<Vec<String>>,
        calls: Mutex<Vec<Call>>,
        list_monitors_err: bool,
        create_err_for: Option<String>,
        close_err_for: Option<String>,
    }

    impl FakeWindowPort {
        fn new(monitors: Vec<MonitorInfo>, primary: Option<MonitorInfo>) -> Self {
            Self {
                monitors,
                primary,
                existing_labels: Mutex::new(Vec::new()),
                calls: Mutex::new(Vec::new()),
                list_monitors_err: false,
                create_err_for: None,
                close_err_for: None,
            }
        }

        fn with_existing(mut self, label: &str) -> Self {
            self.existing_labels
                .get_mut()
                .unwrap()
                .push(label.to_string());
            self
        }

        fn with_list_monitors_err(mut self) -> Self {
            self.list_monitors_err = true;
            self
        }

        fn with_create_err_for(mut self, label: &str) -> Self {
            self.create_err_for = Some(label.to_string());
            self
        }

        fn with_close_err_for(mut self, label: &str) -> Self {
            self.close_err_for = Some(label.to_string());
            self
        }

        fn calls(&self) -> Vec<Call> {
            self.calls.lock().unwrap().clone()
        }

        fn created_labels(&self) -> Vec<String> {
            self.calls()
                .into_iter()
                .filter_map(|c| match c {
                    Call::CreateTipWindow(label) => Some(label),
                    _ => None,
                })
                .collect()
        }
    }

    impl WindowPort for FakeWindowPort {
        fn list_monitors(&self) -> Result<Vec<MonitorInfo>> {
            self.calls.lock().unwrap().push(Call::ListMonitors);
            if self.list_monitors_err {
                return Err(crate::error::AppError::IoError {
                    message: "fake: list_monitors failure".to_string(),
                });
            }
            Ok(self.monitors.clone())
        }

        fn primary_monitor(&self) -> Option<MonitorInfo> {
            self.calls.lock().unwrap().push(Call::PrimaryMonitor);
            self.primary.clone()
        }

        fn webview_exists(&self, label: &str) -> bool {
            self.calls
                .lock()
                .unwrap()
                .push(Call::WebviewExists(label.to_string()));
            self.existing_labels
                .lock()
                .unwrap()
                .iter()
                .any(|l| l == label)
        }

        fn create_tip_window(&self, spec: TipWindowSpec) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(Call::CreateTipWindow(spec.label.clone()));
            if self.create_err_for.as_deref() == Some(spec.label.as_str()) {
                return Err(crate::error::AppError::IoError {
                    message: format!("fake: create_tip_window failure for {}", spec.label),
                });
            }
            self.existing_labels.lock().unwrap().push(spec.label);
            Ok(())
        }

        fn close_window(&self, label: &str) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(Call::CloseWindow(label.to_string()));
            if self.close_err_for.as_deref() == Some(label) {
                return Err(crate::error::AppError::IoError {
                    message: format!("fake: close_window failure for {label}"),
                });
            }
            self.existing_labels.lock().unwrap().retain(|l| l != label);
            Ok(())
        }

        fn list_window_labels(&self) -> Vec<String> {
            self.calls.lock().unwrap().push(Call::ListWindowLabels);
            self.existing_labels.lock().unwrap().clone()
        }
    }

    fn monitor(name: &str, x: i32, y: i32) -> MonitorInfo {
        MonitorInfo {
            name: Some(name.to_string()),
            position_x: x,
            position_y: y,
            width: 1920,
            height: 1080,
        }
    }

    #[test]
    fn show_tip_windows_creates_one_per_monitor_with_primary_url_for_primary() {
        let primary = monitor("DP-1", 0, 0);
        let secondary = monitor("HDMI-1", 1920, 0);
        let port = Arc::new(FakeWindowPort::new(
            vec![primary.clone(), secondary.clone()],
            Some(primary.clone()),
        ));
        let service = WindowService::new(port.clone());

        service.show_tip_windows();

        let labels = port.created_labels();
        assert_eq!(
            labels,
            vec![
                "tip-window-0".to_string(),
                "tip-window-minimal-1".to_string()
            ]
        );
    }

    #[test]
    fn show_tip_windows_no_op_when_monitor_enumeration_fails() {
        let port = Arc::new(FakeWindowPort::new(vec![], None).with_list_monitors_err());
        let service = WindowService::new(port.clone());

        service.show_tip_windows();

        assert!(
            port.created_labels().is_empty(),
            "no windows should be created when monitor listing fails"
        );
    }

    #[test]
    fn show_tip_windows_no_op_when_no_monitors() {
        let port = Arc::new(FakeWindowPort::new(vec![], None));
        let service = WindowService::new(port.clone());

        service.show_tip_windows();

        assert!(port.created_labels().is_empty());
    }

    #[test]
    fn show_tip_windows_skips_label_already_alive() {
        let primary = monitor("DP-1", 0, 0);
        let port = Arc::new(
            FakeWindowPort::new(vec![primary.clone()], Some(primary)).with_existing("tip-window-0"),
        );
        let service = WindowService::new(port.clone());

        service.show_tip_windows();

        assert!(
            port.created_labels().is_empty(),
            "should not recreate when window with same label already exists"
        );
    }

    #[test]
    fn show_tip_windows_continues_after_create_error() {
        let primary = monitor("DP-1", 0, 0);
        let secondary = monitor("HDMI-1", 1920, 0);
        let port = Arc::new(
            FakeWindowPort::new(vec![primary.clone(), secondary.clone()], Some(primary))
                .with_create_err_for("tip-window-0"),
        );
        let service = WindowService::new(port.clone());

        service.show_tip_windows();

        let labels = port.created_labels();
        assert_eq!(
            labels.len(),
            2,
            "both windows attempted even when first fails: {labels:?}"
        );
        assert!(labels.contains(&"tip-window-minimal-1".to_string()));
    }

    #[test]
    fn hide_tip_windows_closes_only_tip_window_labels() {
        let port = Arc::new(
            FakeWindowPort::new(vec![], None)
                .with_existing("tip-window-0")
                .with_existing("tip-window-minimal-1")
                .with_existing("main-window")
                .with_existing("tray-panel"),
        );
        let service = WindowService::new(port.clone());

        service.hide_tip_windows();

        let closed: Vec<String> = port
            .calls()
            .into_iter()
            .filter_map(|c| match c {
                Call::CloseWindow(label) => Some(label),
                _ => None,
            })
            .collect();
        assert_eq!(closed.len(), 2);
        assert!(closed.contains(&"tip-window-0".to_string()));
        assert!(closed.contains(&"tip-window-minimal-1".to_string()));
        assert!(
            !closed.contains(&"main-window".to_string()),
            "main-window must never be closed by hide_tip_windows"
        );
        assert!(
            !closed.contains(&"tray-panel".to_string()),
            "tray-panel must never be closed by hide_tip_windows"
        );
    }

    #[test]
    fn hide_tip_windows_no_op_when_none_open() {
        let port = Arc::new(FakeWindowPort::new(vec![], None));
        let service = WindowService::new(port.clone());

        service.hide_tip_windows();

        assert!(
            !port
                .calls()
                .iter()
                .any(|c| matches!(c, Call::CloseWindow(_))),
            "no close calls should occur when no tip windows are open"
        );
    }

    #[test]
    fn hide_tip_windows_propagates_close_failure_via_log_and_continues() {
        let port = Arc::new(
            FakeWindowPort::new(vec![], None)
                .with_existing("tip-window-0")
                .with_existing("tip-window-minimal-1")
                .with_close_err_for("tip-window-0"),
        );
        let service = WindowService::new(port.clone());

        service.hide_tip_windows();

        let closed: Vec<String> = port
            .calls()
            .into_iter()
            .filter_map(|c| match c {
                Call::CloseWindow(label) => Some(label),
                _ => None,
            })
            .collect();
        assert_eq!(
            closed.len(),
            2,
            "second window must still be attempted after first errors"
        );
    }

    #[test]
    fn show_tip_windows_picks_primary_by_name_not_index() {
        // Primary is the second monitor by index. The first one gets the
        // minimal label; the second one (matching primary name) gets tip.html.
        let alt = monitor("DP-1", 0, 0);
        let primary = monitor("HDMI-2", 1920, 0);
        let port = Arc::new(FakeWindowPort::new(
            vec![alt.clone(), primary.clone()],
            Some(primary.clone()),
        ));
        let service = WindowService::new(port.clone());

        service.show_tip_windows();

        let labels = port.created_labels();
        // Index 0 (alt monitor) is NOT primary, so minimal label.
        assert_eq!(labels[0], "tip-window-minimal-0");
        // Index 1 (matches primary name) IS primary, so the canonical label.
        assert_eq!(labels[1], "tip-window-0");
    }

    #[test]
    fn show_tip_windows_falls_back_to_index_zero_when_os_reports_no_primary() {
        // Linux with no primary reported: first monitor wins.
        let first = monitor("eDP-1", 0, 0);
        let second = monitor("HDMI-1", 1920, 0);
        let port = Arc::new(FakeWindowPort::new(
            vec![first.clone(), second.clone()],
            None,
        ));
        let service = WindowService::new(port.clone());

        service.show_tip_windows();

        let labels = port.created_labels();
        assert_eq!(labels[0], "tip-window-0");
        assert_eq!(labels[1], "tip-window-minimal-1");
    }

    #[tokio::test]
    async fn service_lifecycle_methods_are_no_ops() {
        let port = Arc::new(FakeWindowPort::new(vec![], None));
        let service = WindowService::new(port);
        let ctx = ServiceContext::default();
        service.init(&ctx).await.unwrap();
        service.start(&ctx).await.unwrap();
        service.shutdown().await.unwrap();
    }
}
