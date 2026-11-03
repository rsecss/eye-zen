#![allow(clippy::module_name_repetitions)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;
use tokio::sync::watch;
use tracing::{info, warn};

use crate::error::{AppError, Result};
use crate::models::config::{
    canonicalize_language, sanitize_process_whitelist, BehaviorConfig, Config, ConfigSnapshot,
    DisplayConfig, PomodoroConfig, ScheduleConfig, TimerConfig, TimerMode,
};
use crate::models::hotkeys::HotkeysConfig;
use crate::services::{Service, ServiceContext};

pub(crate) struct ConfigService {
    config: Arc<ArcSwap<VersionedConfig>>,
    tx: watch::Sender<Arc<Config>>,
    path: PathBuf,
    write_lock: Mutex<()>,
    app: Mutex<Option<ServiceContext>>,
}

struct VersionedConfig {
    revision: u32,
    config: Arc<Config>,
}

impl ConfigService {
    /// Create a service from a TOML file path.
    #[allow(clippy::missing_errors_doc)]
    pub(crate) fn new(path: PathBuf) -> Result<Self> {
        let config = Self::load_or_default(&path)?;
        let config = Arc::new(config);
        let (tx, _rx) = watch::channel(Arc::clone(&config));
        let versioned = Arc::new(VersionedConfig {
            revision: 0,
            config,
        });

        Ok(Self {
            config: Arc::new(ArcSwap::from(versioned)),
            tx,
            path,
            write_lock: Mutex::new(()),
            app: Mutex::new(None),
        })
    }

    /// Return the current lock-free config snapshot.
    #[must_use]
    pub(crate) fn current(&self) -> Arc<Config> {
        Arc::clone(&self.config.load_full().config)
    }

    /// Return the current IPC snapshot.
    #[must_use]
    pub(crate) fn snapshot(&self) -> ConfigSnapshot {
        let versioned = self.config.load_full();
        ConfigSnapshot {
            revision: versioned.revision,
            config: (*versioned.config).clone(),
        }
    }

    /// Subscribe to config updates.
    #[must_use]
    pub(crate) fn subscribe(&self) -> watch::Receiver<Arc<Config>> {
        self.tx.subscribe()
    }

    /// Replace the timer section and persist the new config.
    #[allow(clippy::missing_errors_doc)]
    pub(crate) fn update_timer(
        &self,
        timer: TimerConfig,
        expected_revision: u32,
    ) -> Result<ConfigSnapshot> {
        self.update_with(expected_revision, move |config| {
            config.timer = timer;
        })
    }

    /// Replace the behavior section and persist the new config.
    ///
    /// The `process_whitelist` is sanitised (trim, lowercase, dedupe, drop
    /// reserved names, truncate to cap) before storage so callers do not need
    /// to do it themselves.
    #[allow(clippy::missing_errors_doc)]
    pub(crate) fn update_behavior(
        &self,
        behavior: BehaviorConfig,
        expected_revision: u32,
    ) -> Result<ConfigSnapshot> {
        let mut sanitised = behavior;
        sanitised.process_whitelist = sanitize_process_whitelist(sanitised.process_whitelist);
        self.update_with(expected_revision, move |config| {
            config.behavior = sanitised;
        })
    }

    /// Replace the display section and persist the new config.
    #[allow(clippy::missing_errors_doc)]
    pub(crate) fn update_display(
        &self,
        display: DisplayConfig,
        expected_revision: u32,
    ) -> Result<ConfigSnapshot> {
        self.update_with(expected_revision, move |config| {
            config.display = display;
        })
    }

    /// Replace the schedule section and persist the new config.
    #[allow(clippy::missing_errors_doc)]
    pub(crate) fn update_schedule(
        &self,
        schedule: ScheduleConfig,
        expected_revision: u32,
    ) -> Result<ConfigSnapshot> {
        self.update_with(expected_revision, move |config| {
            config.schedule = schedule;
        })
    }

    /// Apply and persist hotkeys under the same revision-checked write lock.
    #[allow(clippy::missing_errors_doc)]
    pub(crate) fn update_hotkeys<F>(
        &self,
        hotkeys: HotkeysConfig,
        expected_revision: u32,
        mut apply_hotkeys: F,
    ) -> Result<ConfigSnapshot>
    where
        F: FnMut(&HotkeysConfig) -> Result<()>,
    {
        let snapshot = {
            let _write_guard = self.write_lock.lock().map_err(|_| AppError::IoError {
                message: "config write lock poisoned".to_string(),
            })?;
            let current = self.config.load_full();
            if current.revision != expected_revision {
                return Err(AppError::ConfigConflict {
                    expected: expected_revision,
                    actual: current.revision,
                });
            }

            let previous = current.config.hotkeys.clone();
            let mut config = (*current.config).clone();
            config.hotkeys = hotkeys;
            Self::validate_timer_contract(&config)?;
            let revision =
                current
                    .revision
                    .checked_add(1)
                    .ok_or_else(|| AppError::InvalidOperation {
                        operation: "config revision increment".to_string(),
                        reason: "revision counter exhausted".to_string(),
                    })?;

            if let Err(err) = apply_hotkeys(&config.hotkeys) {
                if let Err(rollback_err) = apply_hotkeys(&previous) {
                    warn!("failed to roll back hotkeys after apply failure: {rollback_err}");
                }
                return Err(err);
            }
            if let Err(err) = self.save(&config) {
                if let Err(rollback_err) = apply_hotkeys(&previous) {
                    warn!("failed to roll back hotkeys after config save failure: {rollback_err}");
                }
                return Err(err);
            }

            let config = Arc::new(config);
            self.config.store(Arc::new(VersionedConfig {
                revision,
                config: Arc::clone(&config),
            }));
            let _ = self.tx.send(Arc::clone(&config));
            ConfigSnapshot {
                revision,
                config: (*config).clone(),
            }
        };

        self.emit_config_changed(&snapshot);
        Ok(snapshot)
    }

    /// Replace the pomodoro section and persist the new config.
    #[allow(clippy::missing_errors_doc)]
    pub(crate) fn update_pomodoro(
        &self,
        pomodoro: PomodoroConfig,
        expected_revision: u32,
    ) -> Result<ConfigSnapshot> {
        self.update_with(expected_revision, move |config| {
            config.pomodoro = pomodoro;
        })
    }

    #[allow(clippy::missing_errors_doc)]
    fn load_or_default(path: &Path) -> Result<Config> {
        if !path.exists() {
            let config = Config::default();
            Self::write_config_file(path, &config)?;
            info!("config not found, created defaults at {}", path.display());
            return Ok(config);
        }

        let content = std::fs::read_to_string(path)?;
        let candidate = toml::from_str::<Config>(&content)
            .map_err(AppError::from)
            .and_then(|mut config| {
                config.behavior.process_whitelist =
                    sanitize_process_whitelist(config.behavior.process_whitelist);
                canonicalize_language(&mut config.display.language);
                Self::validate_timer_contract(&config)?;
                Ok(config)
            });

        match candidate {
            Ok(config) => {
                info!("config loaded from {}", path.display());
                Ok(config)
            }
            Err(err) => {
                warn!("invalid config at {}: {err}", path.display());
                let backup_path = path.with_extension("toml.bak");

                if let Err(copy_err) = std::fs::copy(path, &backup_path) {
                    warn!(
                        "failed to back up invalid config from {} to {}: {copy_err}",
                        path.display(),
                        backup_path.display()
                    );
                }

                let config = Config::default();
                Ok(config)
            }
        }
    }

    #[allow(clippy::missing_errors_doc)]
    fn save(&self, config: &Config) -> Result<()> {
        Self::write_config_file(&self.path, config)?;
        info!("config saved to {}", self.path.display());
        Ok(())
    }

    #[allow(clippy::missing_errors_doc)]
    fn update_with<F>(&self, expected_revision: u32, update: F) -> Result<ConfigSnapshot>
    where
        F: FnOnce(&mut Config),
    {
        let snapshot = {
            let _write_guard = self.write_lock.lock().map_err(|_| AppError::IoError {
                message: "config write lock poisoned".to_string(),
            })?;
            let current = self.config.load_full();
            if current.revision != expected_revision {
                return Err(AppError::ConfigConflict {
                    expected: expected_revision,
                    actual: current.revision,
                });
            }

            let mut config = (*current.config).clone();
            update(&mut config);
            Self::validate_timer_contract(&config)?;
            let revision =
                current
                    .revision
                    .checked_add(1)
                    .ok_or_else(|| AppError::InvalidOperation {
                        operation: "config revision increment".to_string(),
                        reason: "revision counter exhausted".to_string(),
                    })?;
            self.save(&config)?;
            let config = Arc::new(config);
            self.config.store(Arc::new(VersionedConfig {
                revision,
                config: Arc::clone(&config),
            }));
            // watch::Sender::send only errors when every Receiver has been
            // dropped. At runtime that means every consumer service has shut
            // down, so a config update has no audience anyway — safe to drop.
            let _ = self.tx.send(Arc::clone(&config));
            ConfigSnapshot {
                revision,
                config: (*config).clone(),
            }
        };

        self.emit_config_changed(&snapshot);

        Ok(snapshot)
    }

    fn validate_timer_contract(config: &Config) -> Result<()> {
        Self::validate_timer_ranges(&config.timer)?;
        Self::validate_pomodoro_ranges(&config.pomodoro)?;

        let active_work_secs = match config.timer.mode {
            TimerMode::TwentyTwentyTwenty => u64::from(config.timer.work_minutes) * 60,
            TimerMode::Pomodoro => u64::from(config.pomodoro.focus_minutes) * 60,
        };
        let pre_alert_secs = u64::from(config.timer.pre_alert_seconds);
        if active_work_secs <= pre_alert_secs {
            return Err(AppError::ConfigInvalid {
                field: "timer.active_work_duration".to_string(),
                reason: format!(
                    "active work duration ({active_work_secs}s) must exceed pre-alert duration ({pre_alert_secs}s)"
                ),
            });
        }
        Ok(())
    }

    fn validate_timer_ranges(config: &TimerConfig) -> Result<()> {
        if !(1..=120).contains(&config.work_minutes) {
            return Err(AppError::ConfigInvalid {
                field: "work_minutes".to_string(),
                reason: format!("must be 1-120, got {}", config.work_minutes),
            });
        }
        if !(5..=300).contains(&config.rest_seconds) {
            return Err(AppError::ConfigInvalid {
                field: "rest_seconds".to_string(),
                reason: format!("must be 5-300, got {}", config.rest_seconds),
            });
        }
        if !(5..=60).contains(&config.pre_alert_seconds) {
            return Err(AppError::ConfigInvalid {
                field: "pre_alert_seconds".to_string(),
                reason: format!("must be 5-60, got {}", config.pre_alert_seconds),
            });
        }
        if !(10..=300).contains(&config.alert_timeout_seconds) {
            return Err(AppError::ConfigInvalid {
                field: "alert_timeout_seconds".to_string(),
                reason: format!("must be 10-300, got {}", config.alert_timeout_seconds),
            });
        }
        Ok(())
    }

    fn validate_pomodoro_ranges(config: &PomodoroConfig) -> Result<()> {
        if !(1..=180).contains(&config.focus_minutes) {
            return Err(AppError::ConfigInvalid {
                field: "pomodoro.focus_minutes".to_string(),
                reason: format!("must be 1-180, got {}", config.focus_minutes),
            });
        }
        if !(1..=60).contains(&config.short_break_minutes) {
            return Err(AppError::ConfigInvalid {
                field: "pomodoro.short_break_minutes".to_string(),
                reason: format!("must be 1-60, got {}", config.short_break_minutes),
            });
        }
        if !(1..=180).contains(&config.long_break_minutes) {
            return Err(AppError::ConfigInvalid {
                field: "pomodoro.long_break_minutes".to_string(),
                reason: format!("must be 1-180, got {}", config.long_break_minutes),
            });
        }
        if !(1..=12).contains(&config.cycles_per_long) {
            return Err(AppError::ConfigInvalid {
                field: "pomodoro.cycles_per_long".to_string(),
                reason: format!("must be 1-12, got {}", config.cycles_per_long),
            });
        }
        Ok(())
    }

    #[allow(clippy::missing_errors_doc)]
    fn write_config_file(path: &Path, config: &Config) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(config).map_err(|err| AppError::ConfigInvalid {
            field: "serialize".to_string(),
            reason: err.to_string(),
        })?;

        let tmp_path = Self::tmp_path(path);
        std::fs::write(&tmp_path, content)?;
        std::fs::rename(&tmp_path, path)?;
        Ok(())
    }

    #[must_use]
    fn tmp_path(path: &Path) -> PathBuf {
        match path.extension().and_then(std::ffi::OsStr::to_str) {
            Some(ext) if !ext.is_empty() => path.with_extension(format!("{ext}.tmp")),
            _ => path.with_extension("tmp"),
        }
    }

    fn emit_config_changed(&self, snapshot: &ConfigSnapshot) {
        let app = {
            match self.app.lock() {
                Ok(guard) => guard.clone(),
                Err(e) => {
                    warn!("app mutex poisoned in emit_config_changed: {e}");
                    return;
                }
            }
        };
        if let Some(app) = app {
            app.emit_config_changed(snapshot);
        }
    }
}

impl Service for ConfigService {
    async fn init(&self, app: &ServiceContext) -> Result<()> {
        match self.app.lock() {
            Ok(mut guard) => {
                *guard = Some(app.clone());
            }
            Err(e) => {
                warn!("app mutex poisoned in init: {e}");
            }
        }
        Ok(())
    }

    async fn start(&self, _app: &ServiceContext) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        info!("config service shutdown complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::{mpsc, Arc, Barrier, Mutex as TestMutex};
    use std::thread;

    use tempfile::TempDir;

    use super::*;

    fn test_path(dir: &TempDir) -> PathBuf {
        dir.path().join("config.toml")
    }

    #[test]
    fn load_missing_file_creates_default() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = test_path(&dir);
        assert!(!path.exists());

        let service = ConfigService::new(path.clone()).expect("service should load defaults");
        assert_eq!(*service.current(), Config::default());
        assert!(path.exists());
        assert!(!ConfigService::tmp_path(&path).exists());
    }

    #[test]
    fn load_valid_toml() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = test_path(&dir);
        std::fs::write(
            &path,
            r"
[timer]
work_minutes = 25
rest_seconds = 30
",
        )
        .expect("config file should be written");

        let service = ConfigService::new(path).expect("service should load valid config");
        assert_eq!(service.current().timer.work_minutes, 25);
        assert_eq!(service.current().timer.rest_seconds, 30);
        assert_eq!(service.current().timer.pre_alert_seconds, 15);
    }

    #[test]
    fn load_canonicalizes_legacy_en_us_language() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = test_path(&dir);
        std::fs::write(
            &path,
            r#"
[display]
language = "en-US"
theme = "dark"
"#,
        )
        .expect("legacy config file should be written");

        let service = ConfigService::new(path).expect("service should load legacy config");
        assert_eq!(service.current().display.language, "en");
        assert_eq!(service.current().display.theme, "dark");
    }

    #[test]
    fn load_invalid_toml_backs_up_and_uses_defaults() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = test_path(&dir);
        std::fs::write(&path, "invalid [[[ toml content")
            .expect("invalid config should be written");

        let service =
            ConfigService::new(path.clone()).expect("service should recover from invalid config");
        assert_eq!(*service.current(), Config::default());
        assert!(path.with_extension("toml.bak").exists());
        let original = std::fs::read_to_string(&path).expect("invalid config should be preserved");
        assert_eq!(original, "invalid [[[ toml content");
    }

    #[test]
    fn load_creates_parent_directories() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = dir.path().join("nested").join("deep").join("config.toml");

        let service = ConfigService::new(path.clone()).expect("service should create parent dirs");
        assert_eq!(*service.current(), Config::default());
        assert!(path.exists());
        assert!(!ConfigService::tmp_path(&path).exists());
    }

    #[test]
    fn save_and_reload_roundtrip() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = test_path(&dir);
        let service = ConfigService::new(path.clone()).expect("service should initialize");

        let mut config = (*service.current()).clone();
        config.timer.work_minutes = 30;
        service.save(&config).expect("save should succeed");

        let content = std::fs::read_to_string(&path).expect("config should exist");
        let reloaded: Config = toml::from_str(&content).expect("saved config should parse");
        assert_eq!(reloaded.timer.work_minutes, 30);
    }

    #[test]
    fn save_does_not_leave_tmp_file() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = test_path(&dir);
        let service = ConfigService::new(path.clone()).expect("service should initialize");

        service
            .save(&Config::default())
            .expect("save should not leave tmp file behind");

        assert!(!path.with_extension("toml.tmp").exists());
    }

    #[test]
    fn subscribe_receives_updates() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = test_path(&dir);
        let service = ConfigService::new(path).expect("service should initialize");

        let mut rx = service.subscribe();
        assert_eq!(rx.borrow().timer.work_minutes, 20);

        let new_timer = TimerConfig {
            work_minutes: 30,
            ..TimerConfig::default()
        };
        service
            .update_timer(new_timer, 0)
            .expect("timer config update should succeed");

        assert!(rx.has_changed().expect("receiver should observe change"));
        let updated = rx.borrow_and_update();
        assert_eq!(updated.timer.work_minutes, 30);
    }

    #[test]
    fn update_behavior_persists() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = test_path(&dir);
        let service = ConfigService::new(path.clone()).expect("service should initialize");

        let new_behavior = BehaviorConfig {
            sound_enabled: false,
            ..BehaviorConfig::default()
        };
        service
            .update_behavior(new_behavior, 0)
            .expect("behavior update should succeed");

        assert!(!service.current().behavior.sound_enabled);
        let reloaded: Config =
            toml::from_str(&std::fs::read_to_string(&path).expect("config file should exist"))
                .expect("persisted config should parse");
        assert!(!reloaded.behavior.sound_enabled);
    }

    #[test]
    fn update_display_persists() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = test_path(&dir);
        let service = ConfigService::new(path).expect("service should initialize");

        let new_display = DisplayConfig {
            theme: "dark".to_string(),
            ..DisplayConfig::default()
        };
        service
            .update_display(new_display, 0)
            .expect("display update should succeed");

        assert_eq!(service.current().display.theme, "dark");
    }

    #[test]
    fn update_hotkeys_persists() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = test_path(&dir);
        let service = ConfigService::new(path.clone()).expect("service should initialize");

        let new_hotkeys = HotkeysConfig {
            start_rest: "ctrl+shift+b".to_string(),
            ..HotkeysConfig::default()
        };
        service
            .update_hotkeys(new_hotkeys, 0, |_| Ok(()))
            .expect("hotkeys update should succeed");

        assert_eq!(service.current().hotkeys.start_rest, "ctrl+shift+b");
        let reloaded: Config =
            toml::from_str(&std::fs::read_to_string(&path).expect("config file should exist"))
                .expect("persisted config should parse");
        assert_eq!(reloaded.hotkeys.start_rest, "ctrl+shift+b");
    }

    #[test]
    fn current_is_lock_free_snapshot() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = test_path(&dir);
        let service = ConfigService::new(path).expect("service should initialize");

        let snap1 = service.current();
        let snap2 = service.current();
        assert_eq!(*snap1, *snap2);

        let new_timer = TimerConfig {
            work_minutes: 99,
            ..TimerConfig::default()
        };
        service
            .update_timer(new_timer, 0)
            .expect("timer update should succeed");

        let snap3 = service.current();
        assert_eq!(snap3.timer.work_minutes, 99);
        assert_eq!(snap1.timer.work_minutes, 20);
        assert_eq!(service.snapshot().revision, 1);
    }

    #[test]
    fn concurrent_stale_section_update_is_rejected_without_overwrite() {
        for _ in 0..64 {
            let dir = TempDir::new().expect("temp dir should exist");
            let service = Arc::new(
                ConfigService::new(test_path(&dir))
                    .expect("service should initialize for concurrency"),
            );
            let barrier = Arc::new(Barrier::new(3));

            let timer_service = Arc::clone(&service);
            let timer_barrier = Arc::clone(&barrier);
            let timer_thread = thread::spawn(move || {
                let timer = TimerConfig {
                    work_minutes: 30,
                    ..TimerConfig::default()
                };
                timer_barrier.wait();
                timer_service.update_timer(timer, 0)
            });

            let behavior_service = Arc::clone(&service);
            let behavior_barrier = Arc::clone(&barrier);
            let behavior_thread = thread::spawn(move || {
                let behavior = BehaviorConfig {
                    sound_enabled: false,
                    ..BehaviorConfig::default()
                };
                behavior_barrier.wait();
                behavior_service.update_behavior(behavior, 0)
            });

            barrier.wait();
            let timer_outcome = timer_thread.join().expect("timer thread should join");
            let behavior_outcome = behavior_thread.join().expect("behavior thread should join");

            assert_ne!(timer_outcome.is_ok(), behavior_outcome.is_ok());
            let conflict = if let Err(err) = &timer_outcome {
                err
            } else {
                behavior_outcome
                    .as_ref()
                    .expect_err("one update must conflict")
            };
            assert!(matches!(
                conflict,
                AppError::ConfigConflict {
                    expected: 0,
                    actual: 1
                }
            ));

            let config = service.current();
            if timer_outcome.is_ok() {
                assert_eq!(config.timer.work_minutes, 30);
                assert!(config.behavior.sound_enabled);
            } else {
                assert_eq!(config.timer.work_minutes, 20);
                assert!(!config.behavior.sound_enabled);
            }
            assert_eq!(service.snapshot().revision, 1);
        }
    }

    #[test]
    fn stale_hotkey_update_cannot_apply_side_effect_while_first_update_is_slow() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = test_path(&dir);
        let service = Arc::new(
            ConfigService::new(path.clone()).expect("service should initialize for concurrency"),
        );
        let active_hotkeys = Arc::new(TestMutex::new(HotkeysConfig::default()));
        let (first_applied_tx, first_applied_rx) = mpsc::channel();
        let (release_first_tx, release_first_rx) = mpsc::channel();

        let first_hotkeys = HotkeysConfig {
            start_rest: "CommandOrControl+Alt+1".to_string(),
            ..HotkeysConfig::default()
        };
        let first_service = Arc::clone(&service);
        let first_active = Arc::clone(&active_hotkeys);
        let first_candidate = first_hotkeys.clone();
        let first_thread = thread::spawn(move || {
            first_service.update_hotkeys(first_candidate, 0, move |candidate| {
                *first_active
                    .lock()
                    .expect("active hotkeys lock should succeed") = candidate.clone();
                first_applied_tx
                    .send(())
                    .expect("first apply signal should send");
                release_first_rx
                    .recv()
                    .expect("first apply release should arrive");
                Ok(())
            })
        });

        first_applied_rx
            .recv()
            .expect("first update should reach slow side effect");

        let stale_hotkeys = HotkeysConfig {
            start_rest: "CommandOrControl+Alt+2".to_string(),
            ..HotkeysConfig::default()
        };
        let stale_service = Arc::clone(&service);
        let stale_active = Arc::clone(&active_hotkeys);
        let (stale_started_tx, stale_started_rx) = mpsc::channel();
        let stale_thread = thread::spawn(move || {
            stale_started_tx
                .send(())
                .expect("stale update start signal should send");
            stale_service.update_hotkeys(stale_hotkeys, 0, move |candidate| {
                *stale_active
                    .lock()
                    .expect("active hotkeys lock should succeed") = candidate.clone();
                Ok(())
            })
        });
        stale_started_rx
            .recv()
            .expect("stale update should be queued behind first update");

        release_first_tx
            .send(())
            .expect("first update should be released");
        first_thread
            .join()
            .expect("first update thread should join")
            .expect("first update should commit");
        let stale_error = stale_thread
            .join()
            .expect("stale update thread should join")
            .expect_err("stale update must conflict before applying");

        assert!(matches!(
            stale_error,
            AppError::ConfigConflict {
                expected: 0,
                actual: 1
            }
        ));
        assert_eq!(
            *active_hotkeys
                .lock()
                .expect("active hotkeys lock should succeed"),
            first_hotkeys
        );
        assert_eq!(service.current().hotkeys, first_hotkeys);
        let persisted: Config =
            toml::from_str(&std::fs::read_to_string(path).expect("config should persist"))
                .expect("persisted config should parse");
        assert_eq!(persisted.hotkeys, first_hotkeys);
    }

    #[test]
    fn load_semantically_invalid_config_backs_up_and_uses_defaults() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = test_path(&dir);
        let invalid = r#"
[timer]
pre_alert_seconds = 60
mode = "pomodoro"

[pomodoro]
focus_minutes = 1
"#;
        std::fs::write(&path, invalid).expect("invalid config should be written");

        let service = ConfigService::new(path.clone())
            .expect("service should recover from semantically invalid config");

        assert_eq!(*service.current(), Config::default());
        assert!(path.with_extension("toml.bak").exists());
        assert_eq!(
            std::fs::read_to_string(path).expect("invalid config should be preserved"),
            invalid
        );
    }

    #[test]
    fn pomodoro_update_rejects_active_duration_equal_to_pre_alert() {
        let dir = TempDir::new().expect("temp dir should exist");
        let service = ConfigService::new(test_path(&dir)).expect("service should initialize");

        let snapshot = service
            .update_timer(
                TimerConfig {
                    pre_alert_seconds: 60,
                    mode: crate::models::config::TimerMode::Pomodoro,
                    ..TimerConfig::default()
                },
                0,
            )
            .expect("safe pomodoro mode should be accepted");

        let err = service
            .update_pomodoro(
                PomodoroConfig {
                    focus_minutes: 1,
                    ..PomodoroConfig::default()
                },
                snapshot.revision,
            )
            .expect_err("equal active duration must be rejected");

        assert!(matches!(
            err,
            AppError::ConfigInvalid { ref field, .. }
                if field == "timer.active_work_duration"
        ));
        assert_eq!(service.current().pomodoro, PomodoroConfig::default());
    }

    #[test]
    fn timer_update_rejects_pre_alert_that_invalidates_active_pomodoro() {
        let dir = TempDir::new().expect("temp dir should exist");
        let service = ConfigService::new(test_path(&dir)).expect("service should initialize");

        let snapshot = service
            .update_pomodoro(
                PomodoroConfig {
                    focus_minutes: 1,
                    ..PomodoroConfig::default()
                },
                0,
            )
            .expect("inactive pomodoro section may be prepared");
        let snapshot = service
            .update_timer(
                TimerConfig {
                    pre_alert_seconds: 59,
                    mode: crate::models::config::TimerMode::Pomodoro,
                    ..TimerConfig::default()
                },
                snapshot.revision,
            )
            .expect("one-second active duration should be accepted");

        let err = service
            .update_timer(
                TimerConfig {
                    pre_alert_seconds: 60,
                    mode: crate::models::config::TimerMode::Pomodoro,
                    ..TimerConfig::default()
                },
                snapshot.revision,
            )
            .expect_err("equal active duration must be rejected");

        assert!(matches!(
            err,
            AppError::ConfigInvalid { ref field, .. }
                if field == "timer.active_work_duration"
        ));
        assert_eq!(service.current().timer.pre_alert_seconds, 59);
    }

    #[test]
    fn mode_switch_rejects_inactive_invalid_pomodoro_combination() {
        let dir = TempDir::new().expect("temp dir should exist");
        let service = ConfigService::new(test_path(&dir)).expect("service should initialize");

        let snapshot = service
            .update_timer(
                TimerConfig {
                    pre_alert_seconds: 60,
                    ..TimerConfig::default()
                },
                0,
            )
            .expect("20-20-20 duration remains valid");
        let snapshot = service
            .update_pomodoro(
                PomodoroConfig {
                    focus_minutes: 1,
                    ..PomodoroConfig::default()
                },
                snapshot.revision,
            )
            .expect("inactive pomodoro section may be prepared");

        let err = service
            .update_timer(
                TimerConfig {
                    pre_alert_seconds: 60,
                    mode: crate::models::config::TimerMode::Pomodoro,
                    ..TimerConfig::default()
                },
                snapshot.revision,
            )
            .expect_err("mode switch must validate the complete config");

        assert!(matches!(
            err,
            AppError::ConfigInvalid { ref field, .. }
                if field == "timer.active_work_duration"
        ));
        assert_eq!(
            service.current().timer.mode,
            crate::models::config::TimerMode::TwentyTwentyTwenty
        );
    }

    #[test]
    fn load_zero_pomodoro_cycles_backs_up_and_uses_defaults() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = test_path(&dir);
        let invalid = r"
[pomodoro]
cycles_per_long = 0
";
        std::fs::write(&path, invalid).expect("invalid config should be written");

        let service = ConfigService::new(path.clone())
            .expect("service should recover from invalid pomodoro range");

        assert_eq!(*service.current(), Config::default());
        assert!(path.with_extension("toml.bak").exists());
        assert_eq!(
            std::fs::read_to_string(path).expect("invalid config should be preserved"),
            invalid
        );
    }

    #[test]
    fn direct_pomodoro_update_rejects_zero_cycles() {
        let dir = TempDir::new().expect("temp dir should exist");
        let service = ConfigService::new(test_path(&dir)).expect("service should initialize");

        let err = service
            .update_pomodoro(
                PomodoroConfig {
                    cycles_per_long: 0,
                    ..PomodoroConfig::default()
                },
                0,
            )
            .expect_err("zero cycles must be rejected at ConfigService boundary");

        assert!(matches!(
            err,
            AppError::ConfigInvalid { ref field, .. }
                if field == "pomodoro.cycles_per_long"
        ));
        assert_eq!(service.snapshot().revision, 0);
    }
}
