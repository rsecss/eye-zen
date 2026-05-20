#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![cfg_attr(test, allow(unused_imports, dead_code, clippy::unused_self))]

#[cfg(not(test))]
pub(crate) mod commands;
pub(crate) mod error;
#[cfg(not(test))]
pub(crate) mod events;
pub(crate) mod logging;
pub(crate) mod models;
pub(crate) mod platform;
pub(crate) mod services;

#[cfg(not(test))]
use std::future::Future;
#[cfg(not(test))]
use std::sync::Arc;
#[cfg(not(test))]
use std::time::Duration;

#[cfg(not(test))]
use tauri::{Manager, RunEvent, WindowEvent};
#[cfg(not(test))]
use tracing::{error, info, warn};

#[cfg(not(test))]
use crate::services::Service;

#[cfg(not(test))]
#[allow(clippy::missing_errors_doc, clippy::too_many_lines)]
pub fn run() -> Result<(), tauri::Error> {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            commands::get_state_snapshot,
            commands::start_rest,
            commands::skip_rest,
            commands::pause_timer,
            commands::resume_timer,
            commands::get_config,
            commands::update_timer_config,
            commands::update_behavior_config,
            commands::update_display_config,
            commands::update_schedule_config
        ])
        .setup(|app| {
            let log_dir = app
                .path()
                .app_data_dir()
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?
                .join("logs");
            logging::init_tracing(&log_dir);
            info!("Eyezen starting up");

            let config_path = app
                .path()
                .app_config_dir()
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?
                .join("config.toml");

            let config_service =
                services::config::ConfigService::new(config_path).map_err(|err| {
                    error!("failed to initialize ConfigService: {err}");
                    Box::new(err) as Box<dyn std::error::Error>
                })?;
            let sound_service = services::sound::SoundService::new().map_err(|err| {
                error!("failed to initialize SoundService: {err}");
                Box::new(err) as Box<dyn std::error::Error>
            })?;
            let detector_service =
                services::detector::DetectorService::new(platform::create_platform());
            let timer_service = services::timer::TimerService::new(config_service.subscribe());
            let window_service = services::window::WindowService::new();
            let initial_locale = config_service.current().display.language.clone();
            let i18n_service = Arc::new(services::i18n::I18nService::new(&initial_locale));
            let tray_service = services::tray::TrayService::new(
                config_service.subscribe(),
                Arc::clone(&i18n_service),
            );

            let handle = services::ServiceContext::from(app.handle().clone());
            tauri::async_runtime::block_on(async {
                config_service.init(&handle).await?;
                i18n_service.init(&handle).await?;
                detector_service.init(&handle).await?;
                sound_service.init(&handle).await?;
                timer_service.init(&handle).await?;
                window_service.init(&handle).await?;
                tray_service.init(&handle).await?;
                Ok::<(), crate::error::AppError>(())
            })
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?;

            let services = Arc::new(services::AppServices {
                config: config_service,
                timer: timer_service,
                detector: detector_service,
                window: window_service,
                sound: sound_service,
                tray: tray_service,
                i18n: i18n_service,
            });

            app.manage(Arc::clone(&services));

            tauri::async_runtime::block_on(async {
                services.config.start(&handle).await?;
                services.i18n.start(&handle).await?;
                services.detector.start(&handle).await?;
                services.sound.start(&handle).await?;
                services.window.start(&handle).await?;
                services.tray.start(&handle).await?;
                services.timer.start(&handle).await?;
                Ok::<(), crate::error::AppError>(())
            })
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?;

            info!("all services initialized");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main-window" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())?;

    app.run(|app_handle, event| {
        if let RunEvent::ExitRequested { .. } = event {
            if let Some(services) = app_handle.try_state::<services::SharedAppServices>() {
                tauri::async_runtime::block_on(async {
                    info!("shutting down services");
                    // Keep reverse-dependency shutdown aligned with CLAUDE.md:
                    // event sources -> effect executors -> infrastructure.
                    // StatService is not present in the current MVP wiring, so ConfigService is last.
                    shutdown_service("tray", services.tray.shutdown()).await;
                    shutdown_service("timer", services.timer.shutdown()).await;
                    shutdown_service("detector", services.detector.shutdown()).await;
                    shutdown_service("window", services.window.shutdown()).await;
                    shutdown_service("sound", services.sound.shutdown()).await;
                    shutdown_service("i18n", services.i18n.shutdown()).await;
                    shutdown_service("config", services.config.shutdown()).await;
                    info!("all services shut down");
                });
            }
        }
    });

    Ok(())
}

#[cfg(not(test))]
async fn shutdown_service<F>(name: &str, future: F)
where
    F: Future<Output = crate::error::Result<()>>,
{
    match tokio::time::timeout(Duration::from_secs(3), future).await {
        Ok(Ok(())) => {}
        Ok(Err(err)) => warn!("failed to shut down {name} service: {err}"),
        Err(_) => warn!("timed out shutting down {name} service"),
    }
}
