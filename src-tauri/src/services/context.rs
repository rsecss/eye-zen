use std::sync::Arc;

#[cfg(not(test))]
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;
use tracing::info;
#[cfg(not(test))]
use tracing::warn;

#[cfg(not(test))]
use crate::events;
use crate::models::config::Config;
#[cfg(not(test))]
use crate::services::schedule::is_schedule_active;

#[cfg(not(test))]
use super::timer::{
    apply_transition_and_collect_effects, collect_tick_effects, step_time, Effect, Inner,
    SkipFlags, TimerService, TimerState, TrayUpdate,
};
#[cfg(test)]
use super::timer::{Effect, Inner};
#[cfg(not(test))]
use super::SharedAppServices;

#[derive(Clone, Default)]
pub(crate) struct ServiceContext {
    #[cfg(not(test))]
    app: Option<AppHandle>,
}

impl ServiceContext {
    #[must_use]
    #[cfg(not(test))]
    pub(crate) const fn new(app: Option<AppHandle>) -> Self {
        Self { app }
    }

    #[must_use]
    #[cfg(not(test))]
    pub(crate) fn app_handle(&self) -> Option<AppHandle> {
        self.app.clone()
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn app_handle(&self) -> Option<()> {
        None
    }

    #[cfg(not(test))]
    pub(crate) fn emit_config_changed(&self, config: &Config) {
        let Some(app) = self.app.as_ref() else {
            return;
        };

        if let Err(err) = app.emit(events::CONFIG_CHANGED, config) {
            warn!("failed to emit config_changed: {err}");
        }
    }

    #[cfg(test)]
    pub(crate) fn emit_config_changed(&self, _config: &Config) {}

    #[cfg(not(test))]
    pub(crate) fn execute_timer_effect(&self, effect: &Effect) {
        let Some(app) = self.app.as_ref() else {
            info!("STUB effect: {effect:?}");
            return;
        };

        let Some(services) = app.try_state::<SharedAppServices>() else {
            warn!("shared services unavailable while executing effect: {effect:?}");
            return;
        };

        match effect {
            Effect::EmitStateChanged(payload) => {
                if let Err(err) = app.emit(events::STATE_CHANGED, payload) {
                    warn!("failed to emit state_changed: {err}");
                }
            }
            Effect::ShowTipWindows => services.window.show_tip_windows(app),
            Effect::HideTipWindows => services.window.hide_tip_windows(app),
            Effect::PlaySound(sound) => {
                if services.config.current().behavior.sound_enabled {
                    services.sound.play_type(*sound);
                }
            }
            Effect::UpdateTray(update) => match update {
                TrayUpdate::Tooltip(tooltip) => services.tray.update_tooltip(app, *tooltip),
                TrayUpdate::StateIcon(state) => {
                    services.tray.update_pause_item(*state);
                }
            },
            Effect::ResetWorkTimer(duration) => {
                info!("work timer reset to {}s", duration.as_secs());
            }
            Effect::RecordRestSession(session) => {
                let stat = services.stat.clone();
                let session = session.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = stat.record_rest_session(session).await {
                        warn!("failed to record rest session: {err}");
                    }
                });
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn execute_timer_effect(&self, effect: &Effect) {
        info!("STUB effect: {effect:?}");
    }

    #[must_use]
    #[cfg(not(test))]
    pub(crate) fn spawn_timer_loop(
        &self,
        inner: Arc<Mutex<Inner>>,
        mut config_rx: watch::Receiver<Arc<Config>>,
    ) -> Option<JoinHandle<()>> {
        let app = self.app.clone()?;

        Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));

            loop {
                interval.tick().await;

                if config_rx.has_changed().unwrap_or(false) {
                    let config = Arc::clone(&config_rx.borrow_and_update());
                    let mut guard = inner.lock().await;
                    TimerService::sync_runtime_config(&mut guard, &config);
                }

                let flags = current_skip_flags(&app);
                let effects = {
                    let mut guard = inner.lock().await;
                    let now = std::time::Instant::now();

                    match step_time(&guard, now, &flags) {
                        Some(transition) => {
                            if transition.from == TimerState::Working
                                && transition.to == TimerState::Working
                                && flags.afk_active
                            {
                                info!("skip: afk");
                            }
                            apply_transition_and_collect_effects(&mut guard, transition, now)
                        }
                        None => collect_tick_effects(&guard, now),
                    }
                };

                let context = ServiceContext::from(app.clone());
                for effect in &effects {
                    context.execute_timer_effect(effect);
                }
            }
        }))
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) fn spawn_timer_loop(
        &self,
        _inner: Arc<Mutex<Inner>>,
        _config_rx: watch::Receiver<Arc<Config>>,
    ) -> Option<JoinHandle<()>> {
        None
    }
}

impl std::fmt::Debug for ServiceContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("ServiceContext");
        #[cfg(not(test))]
        debug.field("has_app", &self.app.is_some());
        debug.finish()
    }
}

#[cfg(not(test))]
impl From<AppHandle> for ServiceContext {
    fn from(app: AppHandle) -> Self {
        Self::new(Some(app))
    }
}

#[cfg(not(test))]
fn current_skip_flags(app: &AppHandle) -> SkipFlags {
    let Some(services) = app.try_state::<SharedAppServices>() else {
        warn!("shared services unavailable while resolving skip flags");
        return SkipFlags::default();
    };

    let config = services.config.current();
    let fullscreen_skip = config.behavior.fullscreen_skip;
    let schedule_inactive = !is_schedule_active(chrono::Local::now(), &config.schedule);
    let afk_active = config.behavior.afk_skip_enabled
        && services
            .detector
            .is_afk_for_threshold(config.behavior.afk_threshold_minutes);
    SkipFlags {
        fullscreen_active: fullscreen_skip && services.detector.is_fullscreen(),
        schedule_inactive,
        afk_active,
    }
}
