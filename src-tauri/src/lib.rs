#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![cfg_attr(test, allow(unused_imports, dead_code, clippy::unused_self))]

pub(crate) mod commands;
pub(crate) mod error;
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
#[cfg(all(not(test), feature = "plugin-shortcuts"))]
use tauri_plugin_global_shortcut::ShortcutState;
#[cfg(not(test))]
use tracing::{error, info, warn};

#[cfg(all(not(test), feature = "plugin-shortcuts"))]
use crate::models::hotkeys::HotkeyAction;
#[cfg(not(test))]
use crate::services::Service;

#[cfg(not(test))]
#[allow(clippy::missing_errors_doc, clippy::too_many_lines)]
pub fn run() -> Result<(), tauri::Error> {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init());

    #[cfg(feature = "plugin-shortcuts")]
    let builder = builder.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, _shortcut, event| {
                if !matches!(event.state, ShortcutState::Pressed) {
                    return;
                }

                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let Some(services) = app.try_state::<services::SharedAppServices>() else {
                        warn!("shared services unavailable while handling global hotkey");
                        return;
                    };

                    let Some(action) = services.hotkeys.action_for_id(event.id) else {
                        warn!("unmapped global hotkey id {}", event.id);
                        return;
                    };

                    let result = match action {
                        HotkeyAction::StartRest => {
                            services
                                .timer
                                .handle_user_event(services::timer::UserEvent::StartRest)
                                .await
                        }
                        HotkeyAction::SkipRest => {
                            services
                                .timer
                                .handle_user_event(services::timer::UserEvent::Skip)
                                .await
                        }
                        HotkeyAction::TogglePause => services.timer.toggle_pause().await,
                    };

                    if let Err(err) = result {
                        warn!("global hotkey action {action:?} failed: {err}");
                    }
                });
            })
            .build(),
    );

    let app = builder
        .invoke_handler(tauri::generate_handler![
            commands::get_state_snapshot,
            commands::start_rest,
            commands::skip_rest,
            commands::pause_timer,
            commands::resume_timer,
            commands::get_config,
            commands::get_hotkey_status,
            commands::get_statistics_trends,
            commands::statistics_cycle_outcomes,
            commands::export_statistics,
            commands::get_detector_capabilities,
            commands::update_timer_config,
            commands::update_behavior_config,
            commands::update_display_config,
            commands::update_schedule_config,
            commands::update_hotkeys_config,
            commands::update_pomodoro_config
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
            let data_path = app
                .path()
                .app_data_dir()
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?
                .join("data.db");

            let handle = services::ServiceContext::from(app.handle().clone());

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
            let stat_service = services::stat::StatService::new(data_path);
            let window_port: Arc<dyn services::window::WindowPort> =
                Arc::new(services::window::TauriWindowPort::new(app.handle().clone()));
            let window_service = services::window::WindowService::new(window_port);
            let initial_locale = config_service.current().display.language.clone();
            let i18n_service = Arc::new(services::i18n::I18nService::new(&initial_locale));
            let tray_port: Arc<dyn services::tray::TrayPort> =
                Arc::new(services::tray::TauriTrayPort::new(app.handle().clone()));
            let tray_service = services::tray::TrayService::new(
                tray_port,
                config_service.subscribe(),
                Arc::clone(&i18n_service),
            );
            let hotkey_service = {
                #[cfg(feature = "plugin-shortcuts")]
                {
                    services::hotkeys::HotkeyService::new(
                        config_service.subscribe(),
                        app.handle().clone(),
                    )
                }
                #[cfg(not(feature = "plugin-shortcuts"))]
                {
                    services::hotkeys::HotkeyService::new_disabled(config_service.subscribe())
                }
            };

            tauri::async_runtime::block_on(async {
                config_service.init(&handle).await?;
                i18n_service.init(&handle).await?;
                detector_service.init(&handle).await?;
                sound_service.init(&handle).await?;
                stat_service.init(&handle).await?;
                timer_service.init(&handle).await?;
                window_service.init(&handle).await?;
                tray_service.init(&handle).await?;
                hotkey_service.init(&handle).await?;
                Ok::<(), crate::error::AppError>(())
            })
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)?;

            let services = Arc::new(services::AppServices {
                config: config_service,
                timer: timer_service,
                detector: detector_service,
                window: window_service,
                sound: sound_service,
                stat: stat_service,
                tray: tray_service,
                i18n: i18n_service,
                hotkeys: hotkey_service,
            });

            app.manage(Arc::clone(&services));

            tauri::async_runtime::block_on(async {
                services.config.start(&handle).await?;
                services.i18n.start(&handle).await?;
                services.detector.start(&handle).await?;
                services.sound.start(&handle).await?;
                services.stat.start(&handle).await?;
                services.window.start(&handle).await?;
                services.tray.start(&handle).await?;
                services.timer.start(&handle).await?;
                services.hotkeys.start(&handle).await?;
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
                    if let Err(err) = window.hide() {
                        warn!("failed to hide main window on close request: {err}");
                    }
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
                    shutdown_service("hotkeys", services.hotkeys.shutdown()).await;
                    shutdown_service("tray", services.tray.shutdown()).await;
                    shutdown_service("timer", services.timer.shutdown()).await;
                    shutdown_service("detector", services.detector.shutdown()).await;
                    shutdown_service("window", services.window.shutdown()).await;
                    shutdown_service("sound", services.sound.shutdown()).await;
                    shutdown_service("i18n", services.i18n.shutdown()).await;
                    shutdown_service("stat", services.stat.shutdown()).await;
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
