#![allow(clippy::module_name_repetitions)]

use std::collections::BTreeSet;
#[cfg(target_os = "macos")]
use std::os::raw::c_uchar;
use std::sync::{Arc, Mutex};

#[cfg(all(not(test), feature = "plugin-shortcuts"))]
use tauri::AppHandle;
#[cfg(all(not(test), feature = "plugin-shortcuts"))]
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::error::{AppError, Result};
use crate::models::config::Config;
use crate::models::hotkeys::{
    HotkeyAction, HotkeyBindingState, HotkeyBindingStatus, HotkeyStatus, HotkeysConfig,
    MacosAccessibilityStatus,
};
use crate::services::{Service, ServiceContext};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Binding {
    action: HotkeyAction,
    shortcut: String,
    id: u32,
}

struct ActionOutcome {
    active_entry: Option<Binding>,
    status: HotkeyBindingStatus,
    failure_message: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ActiveBindings {
    entries: Vec<Binding>,
}

impl ActiveBindings {
    #[must_use]
    #[cfg_attr(not(feature = "plugin-shortcuts"), allow(dead_code))]
    fn action_for_id(&self, id: u32) -> Option<HotkeyAction> {
        self.entries
            .iter()
            .find_map(|binding| (binding.id == id).then_some(binding.action))
    }
}

trait HotkeyRegistry: Send + Sync {
    fn shortcut_id(&self, shortcut: &str) -> std::result::Result<u32, String>;
    fn register(&self, shortcut: &str) -> std::result::Result<(), String>;
    fn unregister(&self, shortcut: &str) -> std::result::Result<(), String>;
}

#[cfg(all(not(test), not(feature = "plugin-shortcuts")))]
struct DisabledHotkeyRegistry;

#[cfg(all(not(test), not(feature = "plugin-shortcuts")))]
impl HotkeyRegistry for DisabledHotkeyRegistry {
    fn shortcut_id(&self, _shortcut: &str) -> std::result::Result<u32, String> {
        Err("plugin-shortcuts feature is disabled in this build".to_string())
    }

    fn register(&self, _shortcut: &str) -> std::result::Result<(), String> {
        Err("plugin-shortcuts feature is disabled in this build".to_string())
    }

    fn unregister(&self, _shortcut: &str) -> std::result::Result<(), String> {
        Ok(())
    }
}

#[cfg(all(not(test), feature = "plugin-shortcuts"))]
struct TauriHotkeyRegistry {
    app: AppHandle,
}

#[cfg(all(not(test), feature = "plugin-shortcuts"))]
impl TauriHotkeyRegistry {
    fn parse_shortcut(shortcut: &str) -> std::result::Result<Shortcut, String> {
        shortcut
            .parse::<Shortcut>()
            .map_err(|err| format!("invalid shortcut \"{shortcut}\": {err}"))
    }
}

#[cfg(all(not(test), feature = "plugin-shortcuts"))]
impl HotkeyRegistry for TauriHotkeyRegistry {
    fn shortcut_id(&self, shortcut: &str) -> std::result::Result<u32, String> {
        Ok(Self::parse_shortcut(shortcut)?.id())
    }

    fn register(&self, shortcut: &str) -> std::result::Result<(), String> {
        let shortcut = Self::parse_shortcut(shortcut)?;
        self.app
            .global_shortcut()
            .register(shortcut)
            .map_err(|err| err.to_string())
    }

    fn unregister(&self, shortcut: &str) -> std::result::Result<(), String> {
        let shortcut = Self::parse_shortcut(shortcut)?;
        self.app
            .global_shortcut()
            .unregister(shortcut)
            .map_err(|err| err.to_string())
    }
}

struct HotkeyInner {
    registry: Box<dyn HotkeyRegistry>,
    active: Mutex<ActiveBindings>,
    transaction: Mutex<()>,
    status: Mutex<HotkeyStatus>,
    app: Mutex<Option<ServiceContext>>,
}

impl HotkeyInner {
    fn new(registry: Box<dyn HotkeyRegistry>) -> Self {
        Self {
            registry,
            active: Mutex::new(ActiveBindings::default()),
            transaction: Mutex::new(()),
            status: Mutex::new(Self::registered_status(
                &[],
                current_macos_accessibility_status(),
            )),
            app: Mutex::new(None),
        }
    }

    fn apply_config(&self, config: &HotkeysConfig) -> Result<()> {
        let _transaction = self.transaction.lock().map_err(|_| AppError::IoError {
            message: "hotkey transaction lock poisoned".to_string(),
        })?;

        let accessibility = current_macos_accessibility_status();
        if accessibility == MacosAccessibilityStatus::Missing {
            self.unregister_active_best_effort();
            self.set_active(ActiveBindings::default())?;
            self.publish_status(Self::permission_status(config, accessibility));
            return Ok(());
        }

        let desired = match self.bindings_from_config(config) {
            Ok(bindings) => bindings,
            Err(err) => {
                self.publish_status(Self::failed_status(config, accessibility, &err.to_string()));
                return Err(err);
            }
        };

        let current = self.active_bindings()?;
        if current.entries == desired {
            self.publish_status(Self::registered_status(&desired, accessibility));
            return Ok(());
        }

        let mut next_active: Vec<Binding> = Vec::new();
        let mut statuses: Vec<HotkeyBindingStatus> = Vec::new();
        let mut failed_count = 0usize;
        let mut last_failure_message: Option<String> = None;

        for desired_binding in &desired {
            let current_for_action = current
                .entries
                .iter()
                .find(|binding| binding.action == desired_binding.action);
            let outcome = self.reconcile_single_action(desired_binding, current_for_action);
            if let Some(entry) = outcome.active_entry {
                next_active.push(entry);
            }
            if let Some(message) = outcome.failure_message {
                failed_count += 1;
                last_failure_message = Some(message);
            }
            statuses.push(outcome.status);
        }

        self.set_active(ActiveBindings {
            entries: next_active,
        })?;

        let total = desired.len();
        let last_error = if failed_count == total && total > 0 {
            last_failure_message.clone()
        } else {
            None
        };

        // Partial conflicts no longer bubble Err (so Settings doesn't show the
        // global banner), but they still warrant a log trail for diagnosis.
        if failed_count > 0 && failed_count < total {
            warn!(
                "hotkey partial conflict: {}/{} bindings failed; see status for details",
                failed_count, total
            );
        }

        self.publish_status(HotkeyStatus {
            bindings: statuses,
            macos_accessibility: accessibility,
            last_error,
        });

        if failed_count == total && total > 0 {
            Err(AppError::ConfigInvalid {
                field: Self::field_for_action(desired[0].action).to_string(),
                reason: last_failure_message
                    .unwrap_or_else(|| "all hotkey bindings failed".to_string()),
            })
        } else {
            Ok(())
        }
    }

    fn reconcile_single_action(
        &self,
        desired: &Binding,
        current: Option<&Binding>,
    ) -> ActionOutcome {
        if let Some(cur) = current {
            if cur.shortcut == desired.shortcut {
                return ActionOutcome {
                    active_entry: Some(cur.clone()),
                    status: HotkeyBindingStatus {
                        action: desired.action,
                        shortcut: cur.shortcut.clone(),
                        state: HotkeyBindingState::Registered,
                        message: None,
                    },
                    failure_message: None,
                };
            }
            if let Err(err) = self.registry.unregister(&cur.shortcut) {
                warn!(
                    "failed to unregister previous hotkey {}: {err}",
                    cur.shortcut
                );
            }
        }

        match self.registry.register(&desired.shortcut) {
            Ok(()) => ActionOutcome {
                active_entry: Some(desired.clone()),
                status: HotkeyBindingStatus {
                    action: desired.action,
                    shortcut: desired.shortcut.clone(),
                    state: HotkeyBindingState::Registered,
                    message: None,
                },
                failure_message: None,
            },
            Err(err) => {
                let message = format!(
                    "failed to register {} for {:?}: {err}",
                    desired.shortcut, desired.action
                );
                let restored = current.and_then(|cur| {
                    if self.registry.register(&cur.shortcut).is_ok() {
                        Some(cur.clone())
                    } else {
                        warn!(
                            "failed to restore previous hotkey {} after rollback",
                            cur.shortcut
                        );
                        None
                    }
                });

                ActionOutcome {
                    active_entry: restored,
                    status: HotkeyBindingStatus {
                        action: desired.action,
                        shortcut: desired.shortcut.clone(),
                        state: HotkeyBindingState::Conflict,
                        message: Some(message.clone()),
                    },
                    failure_message: Some(message),
                }
            }
        }
    }

    fn status(&self) -> HotkeyStatus {
        self.status.lock().map_or_else(
            |err| HotkeyStatus {
                bindings: Vec::new(),
                macos_accessibility: current_macos_accessibility_status(),
                last_error: Some(format!("hotkey status lock poisoned: {err}")),
            },
            |status| status.clone(),
        )
    }

    #[cfg_attr(not(feature = "plugin-shortcuts"), allow(dead_code))]
    fn action_for_id(&self, id: u32) -> Option<HotkeyAction> {
        self.active
            .lock()
            .ok()
            .and_then(|active| active.action_for_id(id))
    }

    fn set_app(&self, app: ServiceContext) {
        match self.app.lock() {
            Ok(mut guard) => *guard = Some(app),
            Err(err) => warn!("hotkey app lock poisoned in init: {err}"),
        }
    }

    fn clear_bindings(&self) {
        let old = match self.active_bindings() {
            Ok(bindings) => bindings,
            Err(err) => {
                warn!("failed to read active hotkeys during shutdown: {err}");
                return;
            }
        };

        if let Err(err) = self.unregister_bindings(&old.entries) {
            warn!("failed to unregister hotkeys during shutdown: {err}");
        }

        if let Err(err) = self.set_active(ActiveBindings::default()) {
            warn!("failed to clear hotkey bindings during shutdown: {err}");
        }
    }

    fn bindings_from_config(&self, config: &HotkeysConfig) -> Result<Vec<Binding>> {
        let mut seen = BTreeSet::new();
        let mut bindings = Vec::new();

        for (action, shortcut) in config.entries() {
            let shortcut = shortcut.trim();
            if shortcut.is_empty() {
                return Err(AppError::ConfigInvalid {
                    field: Self::field_for_action(action).to_string(),
                    reason: "shortcut must not be empty".to_string(),
                });
            }

            let id =
                self.registry
                    .shortcut_id(shortcut)
                    .map_err(|reason| AppError::ConfigInvalid {
                        field: Self::field_for_action(action).to_string(),
                        reason,
                    })?;

            if !seen.insert(id) {
                return Err(AppError::ConfigInvalid {
                    field: Self::field_for_action(action).to_string(),
                    reason: "shortcut duplicates another hotkey action".to_string(),
                });
            }

            bindings.push(Binding {
                action,
                shortcut: shortcut.to_string(),
                id,
            });
        }

        Ok(bindings)
    }

    fn unregister_active_best_effort(&self) {
        match self.active_bindings() {
            Ok(active) => {
                if let Err(err) = self.unregister_bindings(&active.entries) {
                    warn!("failed to unregister hotkeys after permission downgrade: {err}");
                }
            }
            Err(err) => warn!("failed to read active hotkeys after permission downgrade: {err}"),
        }
    }

    fn active_bindings(&self) -> Result<ActiveBindings> {
        self.active
            .lock()
            .map(|active| active.clone())
            .map_err(|_| AppError::IoError {
                message: "hotkey active bindings lock poisoned".to_string(),
            })
    }

    fn set_active(&self, bindings: ActiveBindings) -> Result<()> {
        self.active
            .lock()
            .map(|mut active| *active = bindings)
            .map_err(|_| AppError::IoError {
                message: "hotkey active bindings lock poisoned".to_string(),
            })
    }

    fn unregister_bindings(&self, bindings: &[Binding]) -> std::result::Result<(), String> {
        for binding in bindings {
            self.registry.unregister(&binding.shortcut)?;
        }
        Ok(())
    }

    fn publish_status(&self, status: HotkeyStatus) {
        let emit_status = status.clone();
        match self.status.lock() {
            Ok(mut guard) => *guard = status,
            Err(err) => {
                warn!("hotkey status lock poisoned: {err}");
                return;
            }
        }

        let app = match self.app.lock() {
            Ok(guard) => guard.clone(),
            Err(err) => {
                warn!("hotkey app lock poisoned while publishing status: {err}");
                return;
            }
        };

        if let Some(app) = app {
            app.emit_hotkey_status_changed(&emit_status);
        }
    }

    fn registered_status(
        bindings: &[Binding],
        accessibility: MacosAccessibilityStatus,
    ) -> HotkeyStatus {
        HotkeyStatus {
            bindings: bindings
                .iter()
                .map(|binding| HotkeyBindingStatus {
                    action: binding.action,
                    shortcut: binding.shortcut.clone(),
                    state: HotkeyBindingState::Registered,
                    message: None,
                })
                .collect(),
            macos_accessibility: accessibility,
            last_error: None,
        }
    }

    fn failed_status(
        config: &HotkeysConfig,
        accessibility: MacosAccessibilityStatus,
        message: &str,
    ) -> HotkeyStatus {
        HotkeyStatus {
            bindings: config
                .entries()
                .into_iter()
                .map(|(action, shortcut)| HotkeyBindingStatus {
                    action,
                    shortcut: shortcut.to_string(),
                    state: HotkeyBindingState::Conflict,
                    message: Some(message.to_string()),
                })
                .collect(),
            macos_accessibility: accessibility,
            last_error: Some(message.to_string()),
        }
    }

    fn permission_status(
        config: &HotkeysConfig,
        accessibility: MacosAccessibilityStatus,
    ) -> HotkeyStatus {
        let message = "macOS Accessibility permission is required for global hotkeys".to_string();
        HotkeyStatus {
            bindings: config
                .entries()
                .into_iter()
                .map(|(action, shortcut)| HotkeyBindingStatus {
                    action,
                    shortcut: shortcut.to_string(),
                    state: HotkeyBindingState::PermissionMissing,
                    message: Some(message.clone()),
                })
                .collect(),
            macos_accessibility: accessibility,
            last_error: Some(message),
        }
    }

    const fn field_for_action(action: HotkeyAction) -> &'static str {
        match action {
            HotkeyAction::StartRest => "hotkeys.start_rest",
            HotkeyAction::SkipRest => "hotkeys.skip_rest",
            HotkeyAction::TogglePause => "hotkeys.toggle_pause",
        }
    }
}

pub(crate) struct HotkeyService {
    inner: Arc<HotkeyInner>,
    config_rx: watch::Receiver<Arc<Config>>,
    sync_handle: tokio::sync::Mutex<Option<JoinHandle<()>>>,
}

impl HotkeyService {
    #[cfg(all(not(test), feature = "plugin-shortcuts"))]
    #[must_use]
    pub(crate) fn new(config_rx: watch::Receiver<Arc<Config>>, app: AppHandle) -> Self {
        Self::with_registry(config_rx, Box::new(TauriHotkeyRegistry { app }))
    }

    #[cfg(all(not(test), not(feature = "plugin-shortcuts")))]
    #[must_use]
    pub(crate) fn new_disabled(config_rx: watch::Receiver<Arc<Config>>) -> Self {
        Self::with_registry(config_rx, Box::new(DisabledHotkeyRegistry))
    }

    fn with_registry(
        config_rx: watch::Receiver<Arc<Config>>,
        registry: Box<dyn HotkeyRegistry>,
    ) -> Self {
        Self {
            inner: Arc::new(HotkeyInner::new(registry)),
            config_rx,
            sync_handle: tokio::sync::Mutex::new(None),
        }
    }

    pub(crate) fn apply_config(&self, config: &HotkeysConfig) -> Result<()> {
        self.inner.apply_config(config)
    }

    pub(crate) fn status(&self) -> HotkeyStatus {
        self.inner.status()
    }

    #[must_use]
    #[cfg_attr(not(feature = "plugin-shortcuts"), allow(dead_code))]
    pub(crate) fn action_for_id(&self, id: u32) -> Option<HotkeyAction> {
        self.inner.action_for_id(id)
    }
}

impl Service for HotkeyService {
    async fn init(&self, app: &ServiceContext) -> Result<()> {
        self.inner.set_app(app.clone());
        Ok(())
    }

    async fn start(&self, _app: &ServiceContext) -> Result<()> {
        let current = Arc::clone(&self.config_rx.borrow());
        if let Err(err) = self.inner.apply_config(&current.hotkeys) {
            warn!("global hotkeys degraded at startup: {err}");
        }

        let inner = Arc::clone(&self.inner);
        let mut config_rx = self.config_rx.clone();
        let handle = tokio::spawn(async move {
            while config_rx.changed().await.is_ok() {
                let config = Arc::clone(&config_rx.borrow_and_update());
                if let Err(err) = inner.apply_config(&config.hotkeys) {
                    warn!("failed to apply hotkey config update: {err}");
                }
            }
        });

        *self.sync_handle.lock().await = Some(handle);
        info!("hotkey service started");
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        if let Some(handle) = self.sync_handle.lock().await.take() {
            handle.abort();
        }
        self.inner.clear_bindings();
        info!("hotkey service shutdown complete");
        Ok(())
    }
}

fn current_macos_accessibility_status() -> MacosAccessibilityStatus {
    #[cfg(target_os = "macos")]
    {
        if is_macos_accessibility_trusted() {
            MacosAccessibilityStatus::Granted
        } else {
            MacosAccessibilityStatus::Missing
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        MacosAccessibilityStatus::NotRequired
    }
}

#[cfg(target_os = "macos")]
fn is_macos_accessibility_trusted() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> c_uchar;
    }

    // SAFETY: AXIsProcessTrusted has no parameters and returns a CoreServices
    // Boolean indicating the current process trust state.
    unsafe { AXIsProcessTrusted() != 0 }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    #[derive(Clone, Default)]
    struct FakeRegistry {
        inner: Arc<Mutex<FakeRegistryState>>,
    }

    #[derive(Default)]
    struct FakeRegistryState {
        ids: BTreeMap<String, u32>,
        next_id: u32,
        registered: BTreeSet<String>,
        fail_register: BTreeSet<String>,
        operations: Vec<String>,
    }

    impl FakeRegistry {
        fn fail_register(&self, shortcut: &str) {
            self.inner
                .lock()
                .expect("fake lock should not be poisoned")
                .fail_register
                .insert(shortcut.to_string());
        }

        fn registered(&self) -> BTreeSet<String> {
            self.inner
                .lock()
                .expect("fake lock should not be poisoned")
                .registered
                .clone()
        }

        fn clear_operations(&self) {
            self.inner
                .lock()
                .expect("fake lock should not be poisoned")
                .operations
                .clear();
        }

        fn operations(&self) -> Vec<String> {
            self.inner
                .lock()
                .expect("fake lock should not be poisoned")
                .operations
                .clone()
        }
    }

    impl HotkeyRegistry for FakeRegistry {
        fn shortcut_id(&self, shortcut: &str) -> std::result::Result<u32, String> {
            let mut inner = self.inner.lock().map_err(|err| err.to_string())?;
            if shortcut.contains("bad") {
                return Err("invalid shortcut".to_string());
            }
            if let Some(id) = inner.ids.get(shortcut).copied() {
                return Ok(id);
            }
            inner.next_id += 1;
            let id = inner.next_id;
            inner.ids.insert(shortcut.to_string(), id);
            Ok(id)
        }

        fn register(&self, shortcut: &str) -> std::result::Result<(), String> {
            let mut inner = self.inner.lock().map_err(|err| err.to_string())?;
            inner.operations.push(format!("register:{shortcut}"));
            if inner.fail_register.contains(shortcut) {
                return Err("shortcut already registered by another app".to_string());
            }
            if !inner.registered.insert(shortcut.to_string()) {
                return Err("shortcut already registered".to_string());
            }
            Ok(())
        }

        fn unregister(&self, shortcut: &str) -> std::result::Result<(), String> {
            let mut inner = self.inner.lock().map_err(|err| err.to_string())?;
            inner.operations.push(format!("unregister:{shortcut}"));
            inner.registered.remove(shortcut);
            Ok(())
        }
    }

    fn make_service(registry: FakeRegistry) -> HotkeyService {
        let config = Arc::new(Config::default());
        let (_tx, rx) = watch::channel(config);
        HotkeyService::with_registry(rx, Box::new(registry))
    }

    #[test]
    fn default_config_registers_all_actions() {
        let registry = FakeRegistry::default();
        let service = make_service(registry.clone());

        service
            .apply_config(&HotkeysConfig::default())
            .expect("default hotkeys should register");

        assert_eq!(
            registry.registered(),
            BTreeSet::from([
                "CommandOrControl+Alt+B".to_string(),
                "CommandOrControl+Alt+P".to_string(),
                "CommandOrControl+Alt+S".to_string(),
            ])
        );
        assert_eq!(service.status().last_error, None);
    }

    #[test]
    fn registration_failure_keeps_other_actions_active() {
        let registry = FakeRegistry::default();
        let service = make_service(registry.clone());
        let old = HotkeysConfig::default();
        service
            .apply_config(&old)
            .expect("old hotkeys should register");

        registry.fail_register("CommandOrControl+Alt+X");
        let new = HotkeysConfig {
            toggle_pause: "CommandOrControl+Alt+X".to_string(),
            ..HotkeysConfig::default()
        };

        let result = service.apply_config(&new);

        assert!(result.is_ok(), "mixed result should not bubble Err");
        assert_eq!(
            registry.registered(),
            BTreeSet::from([
                "CommandOrControl+Alt+B".to_string(),
                "CommandOrControl+Alt+P".to_string(),
                "CommandOrControl+Alt+S".to_string(),
            ]),
            "previous toggle_pause shortcut should be restored after failure"
        );

        let status = service.status();
        assert_eq!(
            status.last_error, None,
            "mixed conflict must not set the global last_error banner"
        );
        let toggle_status = status
            .bindings
            .iter()
            .find(|binding| binding.action == HotkeyAction::TogglePause)
            .expect("toggle_pause status should be present");
        assert_eq!(toggle_status.state, HotkeyBindingState::Conflict);
        assert_eq!(toggle_status.shortcut, "CommandOrControl+Alt+X");
        let start_status = status
            .bindings
            .iter()
            .find(|binding| binding.action == HotkeyAction::StartRest)
            .expect("start_rest status should be present");
        assert_eq!(start_status.state, HotkeyBindingState::Registered);
    }

    #[test]
    fn update_unregisters_old_bindings_before_registering_new_ones() {
        let registry = FakeRegistry::default();
        let service = make_service(registry.clone());
        service
            .apply_config(&HotkeysConfig::default())
            .expect("old hotkeys should register");
        registry.clear_operations();

        let new = HotkeysConfig {
            start_rest: "CommandOrControl+Alt+N".to_string(),
            ..HotkeysConfig::default()
        };

        service
            .apply_config(&new)
            .expect("new hotkeys should register");

        assert_eq!(
            registry.operations(),
            vec![
                "unregister:CommandOrControl+Alt+B".to_string(),
                "register:CommandOrControl+Alt+N".to_string(),
            ],
            "only the changed action should be touched in per-action mode"
        );
    }

    #[test]
    fn partial_conflict_keeps_other_actions_registered_with_no_global_error() {
        let registry = FakeRegistry::default();
        registry.fail_register("CommandOrControl+Alt+S");
        let service = make_service(registry.clone());

        service
            .apply_config(&HotkeysConfig::default())
            .expect("partial conflict on startup should not bubble Err");

        assert_eq!(
            registry.registered(),
            BTreeSet::from([
                "CommandOrControl+Alt+B".to_string(),
                "CommandOrControl+Alt+P".to_string(),
            ]),
            "non-conflicting shortcuts should still register"
        );

        let status = service.status();
        assert_eq!(status.last_error, None);
        let skip_status = status
            .bindings
            .iter()
            .find(|binding| binding.action == HotkeyAction::SkipRest)
            .expect("skip_rest status should be present");
        assert_eq!(skip_status.state, HotkeyBindingState::Conflict);
        assert!(
            skip_status.message.is_some(),
            "conflict row should carry diagnostic message"
        );

        for action in [HotkeyAction::StartRest, HotkeyAction::TogglePause] {
            let row = status
                .bindings
                .iter()
                .find(|binding| binding.action == action)
                .unwrap_or_else(|| panic!("status for {action:?} missing"));
            assert_eq!(row.state, HotkeyBindingState::Registered);
        }
    }

    #[test]
    fn all_bindings_failed_sets_last_error_and_returns_err() {
        let registry = FakeRegistry::default();
        registry.fail_register("CommandOrControl+Alt+B");
        registry.fail_register("CommandOrControl+Alt+S");
        registry.fail_register("CommandOrControl+Alt+P");
        let service = make_service(registry.clone());

        let result = service.apply_config(&HotkeysConfig::default());

        assert!(matches!(result, Err(AppError::ConfigInvalid { .. })));
        assert!(registry.registered().is_empty());

        let status = service.status();
        assert!(
            status.last_error.is_some(),
            "global last_error must surface when every binding fails"
        );
        for binding in &status.bindings {
            assert_eq!(binding.state, HotkeyBindingState::Conflict);
        }
    }

    #[test]
    fn editing_conflict_to_free_shortcut_only_changes_target_row() {
        let registry = FakeRegistry::default();
        registry.fail_register("CommandOrControl+Alt+S");
        let service = make_service(registry.clone());
        service
            .apply_config(&HotkeysConfig::default())
            .expect("partial conflict should keep service running");
        registry.clear_operations();

        let new = HotkeysConfig {
            skip_rest: "CommandOrControl+Alt+K".to_string(),
            ..HotkeysConfig::default()
        };
        service
            .apply_config(&new)
            .expect("freeing the conflict should succeed");

        assert_eq!(
            registry.operations(),
            vec!["register:CommandOrControl+Alt+K".to_string()],
            "only the formerly-conflicting action should be touched"
        );
        assert_eq!(
            registry.registered(),
            BTreeSet::from([
                "CommandOrControl+Alt+B".to_string(),
                "CommandOrControl+Alt+K".to_string(),
                "CommandOrControl+Alt+P".to_string(),
            ])
        );

        let status = service.status();
        assert_eq!(status.last_error, None);
        for binding in &status.bindings {
            assert_eq!(binding.state, HotkeyBindingState::Registered);
        }
    }

    #[test]
    fn duplicate_shortcuts_are_rejected() {
        let registry = FakeRegistry::default();
        let service = make_service(registry);
        let config = HotkeysConfig {
            skip_rest: "CommandOrControl+Alt+B".to_string(),
            ..HotkeysConfig::default()
        };

        let result = service.apply_config(&config);

        assert!(matches!(
            result,
            Err(AppError::ConfigInvalid { field, .. }) if field == "hotkeys.skip_rest"
        ));
        assert_eq!(
            service.status().bindings[0].state,
            HotkeyBindingState::Conflict
        );
    }

    #[test]
    fn action_lookup_uses_registered_shortcut_id() {
        let registry = FakeRegistry::default();
        let service = make_service(registry.clone());
        service
            .apply_config(&HotkeysConfig::default())
            .expect("default hotkeys should register");
        let id = registry
            .shortcut_id("CommandOrControl+Alt+P")
            .expect("fake shortcut id should exist");

        assert_eq!(service.action_for_id(id), Some(HotkeyAction::TogglePause));
    }
}
