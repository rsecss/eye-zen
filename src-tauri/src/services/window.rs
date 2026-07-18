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
use crate::models::channels::PresentationReceipt;
use crate::services::{Service, ServiceContext};

use super::window_layout;

#[must_use]
fn tip_window_focused() -> bool {
    false
}

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
/// label, URL and geometry before calling `WindowPort::create_tip_window`;
/// the port itself only knows how to talk to Tauri's window builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TipWindowSpec {
    pub(crate) label: String,
    pub(crate) url: &'static str,
    pub(crate) position_x: i32,
    pub(crate) position_y: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
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

    /// Return the physical top-left position of an existing webview window.
    fn window_outer_position(&self, label: &str) -> Result<Option<(i32, i32)>>;

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

#[derive(Clone, Copy)]
enum InteractivePlacement {
    NotShown,
    Known(usize),
    Unresolved,
}

impl WindowService {
    #[must_use]
    pub(crate) fn new(port: Arc<dyn WindowPort>) -> Self {
        Self { port }
    }

    /// Create tip windows on every monitor with one interactive surface.
    pub(crate) fn show_tip_windows(&self) -> PresentationReceipt {
        let monitors = match self.port.list_monitors() {
            Ok(monitors) => monitors,
            Err(err) => {
                error!("failed to enumerate monitors, no tip windows shown: {err}");
                return PresentationReceipt::default();
            }
        };

        if monitors.is_empty() {
            error!("no monitors detected, no tip windows shown");
            return PresentationReceipt::default();
        }

        let primary = self.port.primary_monitor();
        let primary_name: Option<window_layout::MonitorName<'_>> =
            primary.as_ref().map(|m| m.name.as_deref());
        let primary_index = monitors
            .iter()
            .enumerate()
            .find(|(index, monitor)| {
                window_layout::is_primary_monitor(*index, monitor.name.as_deref(), primary_name)
            })
            .map_or(0, |(index, _)| index);

        let mut receipt = PresentationReceipt::default();
        let interactive_placement =
            self.show_interactive_window(&monitors, primary_index, &mut receipt);
        self.show_minimal_windows(&monitors, interactive_placement, &mut receipt);
        receipt
    }

    fn show_interactive_window(
        &self,
        monitors: &[MonitorInfo],
        primary_index: usize,
        receipt: &mut PresentationReceipt,
    ) -> InteractivePlacement {
        let label = window_layout::tip_window_label(primary_index, true);
        match self.port.window_outer_position(&label) {
            Ok(Some((position_x, position_y))) => {
                info!("interactive window {label} already exists");
                receipt.windows_shown += 1;
                receipt.interactive_shown = true;
                if let Some(index) = monitor_index_at(monitors, position_x, position_y) {
                    return InteractivePlacement::Known(index);
                }
                warn!(
                    "existing interactive {label} at ({position_x}, {position_y}) does not match any monitor"
                );
                return InteractivePlacement::Unresolved;
            }
            Ok(None) => {}
            Err(err) => {
                warn!("failed to locate existing interactive {label}: {err}");
                receipt.windows_shown += 1;
                receipt.interactive_shown = true;
                return InteractivePlacement::Unresolved;
            }
        }

        let candidates = std::iter::once(primary_index)
            .chain((0..monitors.len()).filter(|index| *index != primary_index));
        for index in candidates {
            let monitor = &monitors[index];
            let spec = TipWindowSpec {
                label: label.clone(),
                url: window_layout::tip_window_url(true),
                position_x: monitor.position_x,
                position_y: monitor.position_y,
                width: monitor.width,
                height: monitor.height,
            };

            match self.port.create_tip_window(spec) {
                Ok(()) => {
                    info!("created interactive {label} on monitor {:?}", monitor.name);
                    receipt.windows_shown += 1;
                    receipt.interactive_shown = true;
                    return InteractivePlacement::Known(index);
                }
                Err(err) => {
                    warn!(
                        "failed to create interactive {label} on monitor {:?}: {err}",
                        monitor.name
                    );
                }
            }
        }

        InteractivePlacement::NotShown
    }

    fn show_minimal_windows(
        &self,
        monitors: &[MonitorInfo],
        interactive_placement: InteractivePlacement,
        receipt: &mut PresentationReceipt,
    ) {
        let interactive_index = match interactive_placement {
            InteractivePlacement::NotShown => None,
            InteractivePlacement::Known(index) => Some(index),
            InteractivePlacement::Unresolved => {
                warn!("skipping minimal window creation because interactive monitor is unresolved");
                return;
            }
        };
        for (index, monitor) in monitors.iter().enumerate() {
            let label = window_layout::tip_window_label(index, false);

            if self.port.webview_exists(&label) {
                info!("minimal window {label} already exists");
                receipt.windows_shown += 1;
                continue;
            }

            if interactive_index == Some(index) {
                continue;
            }

            let spec = TipWindowSpec {
                label: label.clone(),
                url: window_layout::tip_window_url(false),
                position_x: monitor.position_x,
                position_y: monitor.position_y,
                width: monitor.width,
                height: monitor.height,
            };

            if let Err(err) = self.port.create_tip_window(spec) {
                warn!("failed to create minimal window {label}: {err}");
            } else {
                info!("created minimal {label} on monitor {:?}", monitor.name);
                receipt.windows_shown += 1;
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

fn monitor_index_at(monitors: &[MonitorInfo], position_x: i32, position_y: i32) -> Option<usize> {
    let x = i64::from(position_x);
    let y = i64::from(position_y);
    monitors.iter().position(|monitor| {
        let left = i64::from(monitor.position_x);
        let top = i64::from(monitor.position_y);
        let right = left + i64::from(monitor.width);
        let bottom = top + i64::from(monitor.height);
        (left..right).contains(&x) && (top..bottom).contains(&y)
    })
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
    use super::{tip_window_focused, MonitorInfo, Result, TipWindowSpec, WindowPort};
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
                    .focused(tip_window_focused())
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
                    .focused(tip_window_focused())
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

        fn window_outer_position(&self, label: &str) -> Result<Option<(i32, i32)>> {
            let Some(window) = self.app.get_webview_window(label) else {
                return Ok(None);
            };
            window
                .outer_position()
                .map(|position| Some((position.x, position.y)))
                .map_err(|err| crate::error::AppError::IoError {
                    message: err.to_string(),
                })
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
        WindowOuterPosition(String),
        CreateTipWindow(TipWindowSpec),
        CloseWindow(String),
        ListWindowLabels,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ExistingWindow {
        label: String,
        position_x: i32,
        position_y: i32,
    }

    struct FakeWindowPort {
        monitors: Vec<MonitorInfo>,
        primary: Option<MonitorInfo>,
        existing_windows: Mutex<Vec<ExistingWindow>>,
        calls: Mutex<Vec<Call>>,
        successful_specs: Mutex<Vec<TipWindowSpec>>,
        list_monitors_err: bool,
        fail_all_creates: bool,
        interactive_failures_at: Vec<i32>,
        minimal_failures_at: Vec<i32>,
        close_err_for: Option<String>,
    }

    impl FakeWindowPort {
        fn new(monitors: Vec<MonitorInfo>, primary: Option<MonitorInfo>) -> Self {
            Self {
                monitors,
                primary,
                existing_windows: Mutex::new(Vec::new()),
                calls: Mutex::new(Vec::new()),
                successful_specs: Mutex::new(Vec::new()),
                list_monitors_err: false,
                fail_all_creates: false,
                interactive_failures_at: Vec::new(),
                minimal_failures_at: Vec::new(),
                close_err_for: None,
            }
        }

        fn with_existing(mut self, label: &str) -> Self {
            self.existing_windows
                .get_mut()
                .unwrap()
                .push(ExistingWindow {
                    label: label.to_string(),
                    position_x: 0,
                    position_y: 0,
                });
            self
        }

        fn with_existing_at(mut self, label: &str, position_x: i32, position_y: i32) -> Self {
            self.existing_windows
                .get_mut()
                .unwrap()
                .push(ExistingWindow {
                    label: label.to_string(),
                    position_x,
                    position_y,
                });
            self
        }

        fn with_list_monitors_err(mut self) -> Self {
            self.list_monitors_err = true;
            self
        }

        fn with_all_create_failures(mut self) -> Self {
            self.fail_all_creates = true;
            self
        }

        fn with_interactive_failure_at(mut self, position_x: i32) -> Self {
            self.interactive_failures_at.push(position_x);
            self
        }

        fn with_minimal_failure_at(mut self, position_x: i32) -> Self {
            self.minimal_failures_at.push(position_x);
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
            self.successful_specs
                .lock()
                .unwrap()
                .iter()
                .map(|spec| spec.label.clone())
                .collect()
        }

        fn attempted_specs(&self) -> Vec<TipWindowSpec> {
            self.calls()
                .into_iter()
                .filter_map(|call| match call {
                    Call::CreateTipWindow(spec) => Some(spec),
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
            self.existing_windows
                .lock()
                .unwrap()
                .iter()
                .any(|window| window.label == label)
        }

        fn window_outer_position(&self, label: &str) -> Result<Option<(i32, i32)>> {
            self.calls
                .lock()
                .unwrap()
                .push(Call::WindowOuterPosition(label.to_string()));
            Ok(self
                .existing_windows
                .lock()
                .unwrap()
                .iter()
                .find(|window| window.label == label)
                .map(|window| (window.position_x, window.position_y)))
        }

        fn create_tip_window(&self, spec: TipWindowSpec) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(Call::CreateTipWindow(spec.clone()));
            let is_interactive = spec.url == "tip.html";
            let configured_failure = if is_interactive {
                self.interactive_failures_at.contains(&spec.position_x)
            } else {
                self.minimal_failures_at.contains(&spec.position_x)
            };
            if self.fail_all_creates || configured_failure {
                return Err(crate::error::AppError::IoError {
                    message: format!("fake: create_tip_window failure for {}", spec.label),
                });
            }
            self.existing_windows.lock().unwrap().push(ExistingWindow {
                label: spec.label.clone(),
                position_x: spec.position_x,
                position_y: spec.position_y,
            });
            self.successful_specs.lock().unwrap().push(spec);
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
            self.existing_windows
                .lock()
                .unwrap()
                .retain(|window| window.label != label);
            Ok(())
        }

        fn list_window_labels(&self) -> Vec<String> {
            self.calls.lock().unwrap().push(Call::ListWindowLabels);
            self.existing_windows
                .lock()
                .unwrap()
                .iter()
                .map(|window| window.label.clone())
                .collect()
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
    fn show_tip_windows_all_success_returns_full_receipt() {
        let primary = monitor("DP-1", 0, 0);
        let secondary = monitor("HDMI-1", 1920, 0);
        let port = Arc::new(FakeWindowPort::new(
            vec![primary.clone(), secondary.clone()],
            Some(primary.clone()),
        ));
        let service = WindowService::new(port.clone());

        let receipt = service.show_tip_windows();

        let labels = port.created_labels();
        assert_eq!(
            labels,
            vec![
                "tip-window-0".to_string(),
                "tip-window-minimal-1".to_string()
            ]
        );
        assert_eq!(
            receipt,
            PresentationReceipt {
                windows_shown: 2,
                interactive_shown: true,
            }
        );
    }

    #[test]
    fn show_tip_windows_never_requests_focus() {
        assert!(!tip_window_focused(), "tip windows must not steal focus");
    }

    #[test]
    fn show_tip_windows_monitor_enumeration_failure_returns_empty_receipt() {
        let port = Arc::new(FakeWindowPort::new(vec![], None).with_list_monitors_err());
        let service = WindowService::new(port.clone());

        let receipt = service.show_tip_windows();

        assert_eq!(receipt, PresentationReceipt::default());
        assert!(
            port.created_labels().is_empty(),
            "no windows should be created when monitor listing fails"
        );
    }

    #[test]
    fn show_tip_windows_no_monitors_returns_empty_receipt() {
        let port = Arc::new(FakeWindowPort::new(vec![], None));
        let service = WindowService::new(port.clone());

        let receipt = service.show_tip_windows();

        assert_eq!(receipt, PresentationReceipt::default());
        assert!(port.created_labels().is_empty());
    }

    #[test]
    fn show_tip_windows_existing_interactive_counts_as_success() {
        let primary = monitor("DP-1", 0, 0);
        let secondary = monitor("HDMI-1", 1920, 0);
        let port = Arc::new(
            FakeWindowPort::new(vec![primary.clone(), secondary], Some(primary))
                .with_existing("tip-window-0"),
        );
        let service = WindowService::new(port.clone());

        let receipt = service.show_tip_windows();

        assert_eq!(
            receipt,
            PresentationReceipt {
                windows_shown: 2,
                interactive_shown: true,
            }
        );
        assert_eq!(port.created_labels(), vec!["tip-window-minimal-1"]);
    }

    #[test]
    fn show_tip_windows_only_existing_minimal_still_creates_interactive() {
        let primary = monitor("DP-1", 0, 0);
        let port = Arc::new(
            FakeWindowPort::new(vec![primary.clone()], Some(primary))
                .with_existing("tip-window-minimal-0"),
        );
        let service = WindowService::new(port.clone());

        let receipt = service.show_tip_windows();

        assert_eq!(
            receipt,
            PresentationReceipt {
                windows_shown: 2,
                interactive_shown: true,
            }
        );
        assert_eq!(port.created_labels(), vec!["tip-window-0"]);
    }

    #[test]
    fn show_tip_windows_partial_success_keeps_interactive_receipt() {
        let primary = monitor("DP-1", 0, 0);
        let secondary = monitor("HDMI-1", 1920, 0);
        let port = Arc::new(
            FakeWindowPort::new(vec![primary.clone(), secondary.clone()], Some(primary))
                .with_minimal_failure_at(secondary.position_x),
        );
        let service = WindowService::new(port.clone());

        let receipt = service.show_tip_windows();

        assert_eq!(
            receipt,
            PresentationReceipt {
                windows_shown: 1,
                interactive_shown: true,
            }
        );
        assert_eq!(port.created_labels(), vec!["tip-window-0"]);
    }

    #[test]
    fn show_tip_windows_all_create_failures_returns_empty_receipt() {
        let primary = monitor("DP-1", 0, 0);
        let secondary = monitor("HDMI-1", 1920, 0);
        let port = Arc::new(
            FakeWindowPort::new(vec![primary.clone(), secondary], Some(primary))
                .with_all_create_failures(),
        );
        let service = WindowService::new(port.clone());

        let receipt = service.show_tip_windows();

        assert_eq!(receipt, PresentationReceipt::default());
        assert!(port.created_labels().is_empty());
    }

    #[test]
    fn show_tip_windows_minimal_only_receipt_is_not_interactive() {
        let primary = monitor("DP-1", 0, 0);
        let secondary = monitor("HDMI-1", 1920, 0);
        let port = Arc::new(
            FakeWindowPort::new(
                vec![primary.clone(), secondary.clone()],
                Some(primary.clone()),
            )
            .with_interactive_failure_at(primary.position_x)
            .with_interactive_failure_at(secondary.position_x),
        );
        let service = WindowService::new(port.clone());

        let receipt = service.show_tip_windows();

        assert_eq!(
            receipt,
            PresentationReceipt {
                windows_shown: 2,
                interactive_shown: false,
            }
        );
        assert_eq!(
            port.created_labels(),
            vec!["tip-window-minimal-0", "tip-window-minimal-1"]
        );
    }

    #[test]
    fn show_tip_windows_promotes_secondary_after_primary_failure() {
        let primary = monitor("DP-1", 0, 0);
        let secondary = monitor("HDMI-1", 1920, 0);
        let port = Arc::new(
            FakeWindowPort::new(
                vec![primary.clone(), secondary.clone()],
                Some(primary.clone()),
            )
            .with_interactive_failure_at(primary.position_x),
        );
        let service = WindowService::new(port.clone());

        let receipt = service.show_tip_windows();

        assert_eq!(
            receipt,
            PresentationReceipt {
                windows_shown: 2,
                interactive_shown: true,
            }
        );
        let attempts = port.attempted_specs();
        assert_eq!(attempts[0].url, "tip.html");
        assert_eq!(attempts[0].position_x, primary.position_x);
        assert_eq!(attempts[1].url, "tip.html");
        assert_eq!(attempts[1].position_x, secondary.position_x);
        assert_eq!(attempts[2].url, "tip-minimal.html");
        assert_eq!(attempts[2].position_x, primary.position_x);
    }

    #[test]
    fn existing_interactive_on_secondary_keeps_primary_minimal_without_secondary_overlap() {
        let primary = monitor("DP-1", 0, 0);
        let secondary = monitor("HDMI-1", 1920, 0);
        let port = Arc::new(
            FakeWindowPort::new(
                vec![primary.clone(), secondary.clone()],
                Some(primary.clone()),
            )
            .with_existing_at(
                "tip-window-0",
                secondary.position_x,
                secondary.position_y,
            ),
        );
        let service = WindowService::new(port.clone());

        let receipt = service.show_tip_windows();

        assert_eq!(
            receipt,
            PresentationReceipt {
                windows_shown: 2,
                interactive_shown: true,
            }
        );
        let created = port.attempted_specs();
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].label, "tip-window-minimal-0");
        assert_eq!(created[0].position_x, primary.position_x);
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
        let alt = monitor("DP-1", 0, 0);
        let primary = monitor("HDMI-2", 1920, 0);
        let port = Arc::new(FakeWindowPort::new(
            vec![alt.clone(), primary.clone()],
            Some(primary.clone()),
        ));
        let service = WindowService::new(port.clone());

        let receipt = service.show_tip_windows();

        assert!(receipt.interactive_shown);
        let specs = port.attempted_specs();
        assert_eq!(specs[0].label, "tip-window-0");
        assert_eq!(specs[0].position_x, primary.position_x);
        assert_eq!(specs[1].label, "tip-window-minimal-0");
        assert_eq!(specs[1].position_x, alt.position_x);
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

        let receipt = service.show_tip_windows();

        assert!(receipt.interactive_shown);
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
