#![allow(clippy::missing_errors_doc, clippy::must_use_candidate)]
#![allow(clippy::needless_pass_by_value)]

use tauri::State;

use crate::error::AppError;
use tracing::warn;

use crate::models::config::{BehaviorConfig, Config, DisplayConfig, ScheduleConfig, TimerConfig};
use crate::models::hotkeys::{HotkeyStatus, HotkeysConfig};
use crate::models::types::StatePayload;
use crate::services::timer::UserEvent;
use crate::services::SharedAppServices;

type Services<'a> = State<'a, SharedAppServices>;
type CmdResult<T> = Result<T, AppError>;

#[tauri::command]
pub(crate) async fn get_state_snapshot(services: Services<'_>) -> CmdResult<StatePayload> {
    Ok(services.timer.state_snapshot().await)
}

#[tauri::command]
pub(crate) async fn start_rest(services: Services<'_>) -> CmdResult<()> {
    services.timer.handle_user_event(UserEvent::StartRest).await
}

#[tauri::command]
pub(crate) async fn skip_rest(services: Services<'_>) -> CmdResult<()> {
    services.timer.handle_user_event(UserEvent::Skip).await
}

#[tauri::command]
pub(crate) async fn pause_timer(services: Services<'_>) -> CmdResult<()> {
    services.timer.handle_user_event(UserEvent::Pause).await
}

#[tauri::command]
pub(crate) async fn resume_timer(services: Services<'_>) -> CmdResult<()> {
    services.timer.handle_user_event(UserEvent::Resume).await
}

#[allow(clippy::unnecessary_wraps)]
#[tauri::command]
pub(crate) fn get_config(services: Services<'_>) -> CmdResult<Config> {
    Ok((*services.config.current()).clone())
}

#[allow(clippy::unnecessary_wraps)]
#[tauri::command]
pub(crate) fn get_hotkey_status(services: Services<'_>) -> CmdResult<HotkeyStatus> {
    Ok(services.hotkeys.status())
}

fn validate_timer_config(config: &TimerConfig) -> CmdResult<()> {
    if config.work_minutes == 0 || config.work_minutes > 120 {
        return Err(AppError::ConfigInvalid {
            field: "work_minutes".to_string(),
            reason: format!("must be 1-120, got {}", config.work_minutes),
        });
    }
    if config.rest_seconds < 5 || config.rest_seconds > 300 {
        return Err(AppError::ConfigInvalid {
            field: "rest_seconds".to_string(),
            reason: format!("must be 5-300, got {}", config.rest_seconds),
        });
    }
    if config.pre_alert_seconds < 5 || config.pre_alert_seconds > 60 {
        return Err(AppError::ConfigInvalid {
            field: "pre_alert_seconds".to_string(),
            reason: format!("must be 5-60, got {}", config.pre_alert_seconds),
        });
    }
    if config.alert_timeout_seconds < 10 || config.alert_timeout_seconds > 300 {
        return Err(AppError::ConfigInvalid {
            field: "alert_timeout_seconds".to_string(),
            reason: format!("must be 10-300, got {}", config.alert_timeout_seconds),
        });
    }
    Ok(())
}

fn validate_display_config(config: &DisplayConfig) -> CmdResult<()> {
    let valid_languages = ["zh-CN", "en", "en-US"];
    if !valid_languages.contains(&config.language.as_str()) {
        return Err(AppError::ConfigInvalid {
            field: "language".to_string(),
            reason: format!(
                "must be one of [{}], got \"{}\"",
                valid_languages.join(", "),
                config.language
            ),
        });
    }
    let valid_themes = ["light", "dark"];
    if !valid_themes.contains(&config.theme.as_str()) {
        return Err(AppError::ConfigInvalid {
            field: "theme".to_string(),
            reason: format!(
                "must be one of [{}], got \"{}\"",
                valid_themes.join(", "),
                config.theme
            ),
        });
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn update_timer_config(
    services: Services<'_>,
    config: TimerConfig,
) -> CmdResult<()> {
    validate_timer_config(&config)?;
    let services = services.inner().clone();
    tokio::task::spawn_blocking(move || services.config.update_timer(config))
        .await
        .map_err(|err| AppError::IoError {
            message: format!("update_timer_config task failed: {err}"),
        })?
}

#[tauri::command]
pub(crate) async fn update_behavior_config(
    services: Services<'_>,
    config: BehaviorConfig,
) -> CmdResult<()> {
    // BehaviorConfig contains only boolean fields; no range validation needed.
    let services = services.inner().clone();
    tokio::task::spawn_blocking(move || services.config.update_behavior(config))
        .await
        .map_err(|err| AppError::IoError {
            message: format!("update_behavior_config task failed: {err}"),
        })?
}

#[tauri::command]
pub(crate) async fn update_display_config(
    services: Services<'_>,
    config: DisplayConfig,
) -> CmdResult<()> {
    validate_display_config(&config)?;
    let services = services.inner().clone();
    tokio::task::spawn_blocking(move || services.config.update_display(config))
        .await
        .map_err(|err| AppError::IoError {
            message: format!("update_display_config task failed: {err}"),
        })?
}

#[tauri::command]
pub(crate) async fn update_schedule_config(
    services: Services<'_>,
    config: ScheduleConfig,
) -> CmdResult<()> {
    // ScheduleConfig only contains a bool flag and a fixed-size boolean array;
    // no range validation needed.
    let services = services.inner().clone();
    tokio::task::spawn_blocking(move || services.config.update_schedule(config))
        .await
        .map_err(|err| AppError::IoError {
            message: format!("update_schedule_config task failed: {err}"),
        })?
}

#[tauri::command]
pub(crate) async fn update_hotkeys_config(
    services: Services<'_>,
    config: HotkeysConfig,
) -> CmdResult<()> {
    let previous = services.config.current().hotkeys.clone();
    services.hotkeys.apply_config(&config)?;

    let services_for_save = services.inner().clone();
    let config_for_save = config.clone();
    let save_result = tokio::task::spawn_blocking(move || {
        services_for_save.config.update_hotkeys(config_for_save)
    })
    .await
    .map_err(|err| AppError::IoError {
        message: format!("update_hotkeys_config task failed: {err}"),
    })?;

    if let Err(err) = save_result {
        if let Err(rollback_err) = services.hotkeys.apply_config(&previous) {
            warn!("failed to roll back hotkeys after config save failure: {rollback_err}");
        }
        return Err(err);
    }

    Ok(())
}
