#![allow(clippy::module_name_repetitions)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;
use tokio::sync::watch;
use tracing::{info, warn};

use crate::error::{AppError, Result};
use crate::models::config::{
    sanitize_process_whitelist, BehaviorConfig, Config, DisplayConfig, PomodoroConfig,
    ScheduleConfig, TimerConfig,
};
use crate::models::hotkeys::HotkeysConfig;
use crate::services::{Service, ServiceContext};

pub(crate) struct ConfigService {
    config: Arc<ArcSwap<Config>>,
    tx: watch::Sender<Arc<Config>>,
    path: PathBuf,
    write_lock: Mutex<()>,
    app: Mutex<Option<ServiceContext>>,
}

impl ConfigService {
    /// Create a service from a TOML file path.
    #[allow(clippy::missing_errors_doc)]
    pub(crate) fn new(path: PathBuf) -> Result<Self> {
        let config = Self::load_or_default(&path)?;
        let config = Arc::new(config);
        let (tx, _rx) = watch::channel(Arc::clone(&config));

        Ok(Self {
            config: Arc::new(ArcSwap::from(config)),
            tx,
            path,
            write_lock: Mutex::new(()),
            app: Mutex::new(None),
        })
    }

    /// Return the current lock-free config snapshot.
    #[must_use]
    pub(crate) fn current(&self) -> Arc<Config> {
        self.config.load_full()
    }

    /// Subscribe to config updates.
    #[must_use]
    pub(crate) fn subscribe(&self) -> watch::Receiver<Arc<Config>> {
        self.tx.subscribe()
    }

    /// Replace the timer section and persist the new config.
    #[allow(clippy::missing_errors_doc)]
    pub(crate) fn update_timer(&self, timer: TimerConfig) -> Result<()> {
        self.update_with(move |config| {
            config.timer = timer;
        })
    }

    /// Replace the behavior section and persist the new config.
    ///
    /// The `process_whitelist` is sanitised (trim, lowercase, dedupe, drop
    /// reserved names, truncate to cap) before storage so callers do not need
    /// to do it themselves.
    #[allow(clippy::missing_errors_doc)]
    pub(crate) fn update_behavior(&self, behavior: BehaviorConfig) -> Result<()> {
        let mut sanitised = behavior;
        sanitised.process_whitelist = sanitize_process_whitelist(sanitised.process_whitelist);
        self.update_with(move |config| {
            config.behavior = sanitised;
        })
    }

    /// Replace the display section and persist the new config.
    #[allow(clippy::missing_errors_doc)]
    pub(crate) fn update_display(&self, display: DisplayConfig) -> Result<()> {
        self.update_with(move |config| {
            config.display = display;
        })
    }

    /// Replace the schedule section and persist the new config.
    #[allow(clippy::missing_errors_doc)]
    pub(crate) fn update_schedule(&self, schedule: ScheduleConfig) -> Result<()> {
        self.update_with(move |config| {
            config.schedule = schedule;
        })
    }

    /// Replace the hotkeys section and persist the new config.
    #[allow(clippy::missing_errors_doc)]
    pub(crate) fn update_hotkeys(&self, hotkeys: HotkeysConfig) -> Result<()> {
        self.update_with(move |config| {
            config.hotkeys = hotkeys;
        })
    }

    /// Replace the pomodoro section and persist the new config.
    #[allow(clippy::missing_errors_doc)]
    pub(crate) fn update_pomodoro(&self, pomodoro: PomodoroConfig) -> Result<()> {
        self.update_with(move |config| {
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
        match toml::from_str::<Config>(&content) {
            Ok(mut config) => {
                config.behavior.process_whitelist =
                    sanitize_process_whitelist(config.behavior.process_whitelist);
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
    fn update_with<F>(&self, update: F) -> Result<()>
    where
        F: FnOnce(&mut Config),
    {
        let updated = {
            let _write_guard = self.write_lock.lock().map_err(|_| AppError::IoError {
                message: "config write lock poisoned".to_string(),
            })?;
            let mut config = (*self.current()).clone();
            update(&mut config);
            self.save(&config)?;
            let config = Arc::new(config);
            self.config.store(Arc::clone(&config));
            let _ = self.tx.send(Arc::clone(&config));
            config
        };

        self.emit_config_changed(updated.as_ref());

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

    fn emit_config_changed(&self, config: &Config) {
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
            app.emit_config_changed(config);
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
    use std::sync::{Arc, Barrier};
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
            .update_timer(new_timer)
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
            .update_behavior(new_behavior)
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
            .update_display(new_display)
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
            .update_hotkeys(new_hotkeys)
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
            .update_timer(new_timer)
            .expect("timer update should succeed");

        let snap3 = service.current();
        assert_eq!(snap3.timer.work_minutes, 99);
        assert_eq!(snap1.timer.work_minutes, 20);
    }

    #[test]
    fn concurrent_section_updates_preserve_both_changes() {
        let dir = TempDir::new().expect("temp dir should exist");
        let path = test_path(&dir);

        for _ in 0..64 {
            let service = Arc::new(
                ConfigService::new(path.clone())
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
                timer_service
                    .update_timer(timer)
                    .expect("timer update should succeed");
            });

            let behavior_service = Arc::clone(&service);
            let behavior_barrier = Arc::clone(&barrier);
            let behavior_thread = thread::spawn(move || {
                let behavior = BehaviorConfig {
                    sound_enabled: false,
                    ..BehaviorConfig::default()
                };
                behavior_barrier.wait();
                behavior_service
                    .update_behavior(behavior)
                    .expect("behavior update should succeed");
            });

            barrier.wait();
            timer_thread.join().expect("timer thread should join");
            behavior_thread.join().expect("behavior thread should join");

            let config = service.current();
            assert_eq!(config.timer.work_minutes, 30);
            assert!(!config.behavior.sound_enabled);
        }
    }
}
