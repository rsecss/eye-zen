#![allow(clippy::missing_errors_doc, clippy::must_use_candidate)]
#![allow(clippy::needless_pass_by_value)]

use tauri::State;

use crate::error::AppError;
use crate::models::config::{BehaviorConfig, Config, DisplayConfig, TimerConfig};
use crate::models::types::StatePayload;
use crate::services::timer::UserEvent;
use crate::services::SharedAppServices;

type Services<'a> = State<'a, SharedAppServices>;
type CmdResult<T> = Result<T, AppError>;

#[tauri::command]
pub async fn get_state_snapshot(services: Services<'_>) -> CmdResult<StatePayload> {
    Ok(services.timer.state_snapshot().await)
}

#[tauri::command]
pub async fn start_rest(services: Services<'_>) -> CmdResult<()> {
    services.timer.handle_user_event(UserEvent::StartRest).await
}

#[tauri::command]
pub async fn skip_rest(services: Services<'_>) -> CmdResult<()> {
    services.timer.handle_user_event(UserEvent::Skip).await
}

#[tauri::command]
pub async fn pause_timer(services: Services<'_>) -> CmdResult<()> {
    services.timer.handle_user_event(UserEvent::Pause).await
}

#[tauri::command]
pub async fn resume_timer(services: Services<'_>) -> CmdResult<()> {
    services.timer.handle_user_event(UserEvent::Resume).await
}

#[tauri::command]
pub fn get_config(services: Services<'_>) -> CmdResult<Config> {
    Ok((*services.config.current()).clone())
}

#[tauri::command]
pub async fn update_timer_config(services: Services<'_>, config: TimerConfig) -> CmdResult<()> {
    let services = services.inner().clone();
    tokio::task::spawn_blocking(move || services.config.update_timer(config))
        .await
        .map_err(|err| AppError::IoError {
            message: format!("update_timer_config task failed: {err}"),
        })?
}

#[tauri::command]
pub async fn update_behavior_config(
    services: Services<'_>,
    config: BehaviorConfig,
) -> CmdResult<()> {
    let services = services.inner().clone();
    tokio::task::spawn_blocking(move || services.config.update_behavior(config))
        .await
        .map_err(|err| AppError::IoError {
            message: format!("update_behavior_config task failed: {err}"),
        })?
}

#[tauri::command]
pub async fn update_display_config(services: Services<'_>, config: DisplayConfig) -> CmdResult<()> {
    let services = services.inner().clone();
    tokio::task::spawn_blocking(move || services.config.update_display(config))
        .await
        .map_err(|err| AppError::IoError {
            message: format!("update_display_config task failed: {err}"),
        })?
}
