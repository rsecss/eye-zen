#![allow(clippy::module_name_repetitions)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::error::{AppError, Result};
use crate::models::config::{Config, TimerMode};
use crate::models::types::{PomodoroStatePayload, StatePayload};
use crate::services::{Service, ServiceContext};

use super::effect::Effect;
use super::effect_executor;
use super::machine::{
    apply_transition_and_collect_effects, collect_tick_effects, resolve_user_event, step_time,
};
use super::state::{Inner, SkipFlags, UserEvent};

pub(crate) struct TimerService {
    inner: Arc<Mutex<Inner>>,
    #[cfg_attr(test, allow(dead_code))]
    config_rx: watch::Receiver<Arc<Config>>,
    tick_handle: Mutex<Option<JoinHandle<()>>>,
    app: Mutex<Option<ServiceContext>>,
}

impl TimerService {
    /// Create a timer service from the current config snapshot.
    #[must_use]
    pub(crate) fn new(config_rx: watch::Receiver<Arc<Config>>) -> Self {
        let config = Arc::clone(&config_rx.borrow());
        let inner = Inner::from_config(&config);

        Self {
            inner: Arc::new(Mutex::new(inner)),
            config_rx,
            tick_handle: Mutex::new(None),
            app: Mutex::new(None),
        }
    }

    #[allow(dead_code)]
    pub(crate) async fn on_tick(&self, skip_flags: &SkipFlags) -> Vec<Effect> {
        let effects = {
            let mut inner = self.inner.lock().await;
            let now = Instant::now();
            match step_time(&inner, now, skip_flags) {
                Some(transition) => {
                    apply_transition_and_collect_effects(&mut inner, transition, now)
                }
                None => collect_tick_effects(&inner, now),
            }
        };

        let app = self.app.lock().await.clone();
        for effect in &effects {
            effect_executor::execute_effect(
                app.as_ref().map(|c| c as &dyn super::EffectSink),
                effect,
            );
        }

        effects
    }

    /// Apply a user event and execute resulting effects.
    ///
    /// # Errors
    ///
    /// Returns an error when the event is invalid for the current timer state.
    pub(crate) async fn handle_user_event(&self, event: UserEvent) -> Result<()> {
        let effects = {
            let mut inner = self.inner.lock().await;
            if let Some(transition) = resolve_user_event(&inner.state, event, inner.paused_from) {
                let now = Instant::now();
                apply_transition_and_collect_effects(&mut inner, transition, now)
            } else {
                warn!("invalid timer event {event:?} in state {:?}", inner.state);
                return Err(AppError::InvalidOperation {
                    operation: format!("timer event {event:?}"),
                    reason: format!("invalid in {:?}", inner.state),
                });
            }
        };

        let app = self.app.lock().await.clone();
        for effect in &effects {
            effect_executor::execute_effect(
                app.as_ref().map(|c| c as &dyn super::EffectSink),
                effect,
            );
        }

        Ok(())
    }

    #[cfg_attr(not(feature = "plugin-shortcuts"), allow(dead_code))]
    pub(crate) async fn toggle_pause(&self) -> Result<()> {
        let event = {
            let inner = self.inner.lock().await;
            if inner.state == super::state::TimerState::Paused {
                UserEvent::Resume
            } else {
                UserEvent::Pause
            }
        };

        self.handle_user_event(event).await
    }

    #[allow(dead_code)]
    pub(crate) async fn apply_config(&self, config: &Config) {
        let mut inner = self.inner.lock().await;
        Self::sync_runtime_config(&mut inner, config);

        info!(
            "timer config updated: mode={:?} work={}m rest={}s pre_alert={}s timeout={}s focus={}m short={}m long={}m cycles={}",
            config.timer.mode,
            config.timer.work_minutes,
            config.timer.rest_seconds,
            config.timer.pre_alert_seconds,
            config.timer.alert_timeout_seconds,
            config.pomodoro.focus_minutes,
            config.pomodoro.short_break_minutes,
            config.pomodoro.long_break_minutes,
            config.pomodoro.cycles_per_long,
        );
    }

    /// Build a frontend-friendly state snapshot.
    pub(crate) async fn state_snapshot(&self) -> StatePayload {
        let inner = self.inner.lock().await;
        let now = Instant::now();

        let remaining_secs = if inner.state == super::state::TimerState::Paused {
            inner
                .paused_remaining
                .map_or(0, |d| u32::try_from(d.as_secs()).unwrap_or(u32::MAX))
        } else {
            inner
                .remaining(now)
                .map_or(0, |d| u32::try_from(d.as_secs()).unwrap_or(u32::MAX))
        };

        let pomodoro = if inner.mode == TimerMode::Pomodoro {
            Some(PomodoroStatePayload {
                cycle_index: inner.cycle_index,
                cycles_per_long: inner.cycles_per_long,
                is_long_break: inner.state == super::state::TimerState::Resting
                    && inner.is_long_break,
            })
        } else {
            None
        };

        StatePayload {
            state: inner.state.as_str().to_string(),
            remaining_secs,
            work_minutes: u32::try_from(inner.work_duration.as_secs() / 60).unwrap_or(u32::MAX),
            rest_seconds: u32::try_from(inner.rest_duration.as_secs()).unwrap_or(u32::MAX),
            mode: inner.mode,
            pomodoro,
        }
    }

    pub(crate) fn sync_runtime_config(inner: &mut Inner, config: &Config) {
        let new_mode = config.timer.mode;
        let mode_changed = inner.mode != new_mode;

        // Pomodoro parameters always tracked so a later mode swap is cheap.
        inner.cycles_per_long = config.pomodoro.cycles_per_long.max(1);
        inner.short_break_duration =
            Duration::from_secs(u64::from(config.pomodoro.short_break_minutes) * 60);
        inner.long_break_duration =
            Duration::from_secs(u64::from(config.pomodoro.long_break_minutes) * 60);
        inner.pre_alert_duration = Duration::from_secs(u64::from(config.timer.pre_alert_seconds));
        inner.alert_timeout = Duration::from_secs(u64::from(config.timer.alert_timeout_seconds));

        let work_secs = match new_mode {
            TimerMode::TwentyTwentyTwenty => u64::from(config.timer.work_minutes) * 60,
            TimerMode::Pomodoro => u64::from(config.pomodoro.focus_minutes) * 60,
        };
        let rest_secs = match new_mode {
            TimerMode::TwentyTwentyTwenty => u64::from(config.timer.rest_seconds),
            TimerMode::Pomodoro => u64::from(config.pomodoro.short_break_minutes) * 60,
        };
        inner.work_duration = Duration::from_secs(work_secs);
        inner.rest_duration = Duration::from_secs(rest_secs);

        if mode_changed {
            inner.mode = new_mode;
            inner.cycle_index = 1;
            inner.is_long_break = false;
            inner.paused_from = None;
            inner.paused_remaining = None;
            inner.state = super::state::TimerState::Working;
            inner.state_entered_at = Instant::now();
            info!("timer mode switched to {:?}, runtime reset", new_mode);
        }
    }
}

impl Service for TimerService {
    async fn init(&self, app: &ServiceContext) -> Result<()> {
        *self.app.lock().await = Some(app.clone());
        Ok(())
    }

    async fn start(&self, app: &ServiceContext) -> Result<()> {
        if let Some(handle) = app.spawn_timer_loop(Arc::clone(&self.inner), self.config_rx.clone())
        {
            *self.tick_handle.lock().await = Some(handle);
            info!("timer service tick loop started");
        }
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        if let Some(handle) = self.tick_handle.lock().await.take() {
            handle.abort();
            info!("timer service tick loop stopped");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::sync::watch;

    use super::*;
    use crate::services::timer::TimerState;

    fn make_test_service() -> (TimerService, watch::Sender<Arc<Config>>) {
        let config = Arc::new(Config::default());
        let (tx, rx) = watch::channel(config);
        (TimerService::new(rx), tx)
    }

    fn past_instant(seconds: u64) -> Instant {
        let dur = Duration::from_secs(seconds);
        Instant::now().checked_sub(dur).unwrap_or_else(Instant::now)
    }

    #[tokio::test]
    async fn initial_state_is_working() {
        let (service, _tx) = make_test_service();
        let inner = service.inner.lock().await;
        assert_eq!(inner.state, TimerState::Working);
    }

    #[tokio::test]
    async fn on_tick_no_transition_when_fresh() {
        let (service, _tx) = make_test_service();
        let effects = service.on_tick(&SkipFlags::default()).await;
        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::EmitStateChanged(_))));
    }

    #[tokio::test]
    async fn on_tick_resting_emits_decreasing_remaining_secs() {
        use crate::services::timer::machine::collect_tick_effects;

        let (service, _tx) = make_test_service();
        let future_now = Instant::now() + Duration::from_secs(5);
        {
            let mut inner = service.inner.lock().await;
            inner.state = TimerState::Resting;
            let effects = collect_tick_effects(&inner, future_now);
            let payload = effects.iter().find_map(|effect| match effect {
                Effect::EmitStateChanged(payload) => Some(payload),
                _ => None,
            });

            assert!(matches!(
                payload,
                Some(StatePayload {
                    state,
                    remaining_secs: 14..=15,
                    ..
                }) if state == "resting"
            ));
        }
    }

    #[tokio::test]
    async fn on_tick_transitions_after_timeout() {
        use crate::services::timer::machine::step_time;

        let (service, _tx) = make_test_service();
        let future_now = Instant::now() + Duration::from_mins(20);

        let mut inner = service.inner.lock().await;
        if let Some(transition) = step_time(&inner, future_now, &SkipFlags::default()) {
            inner.apply_transition_at(transition, future_now);
        }
        assert_eq!(inner.state, TimerState::PreAlert);
    }

    #[tokio::test]
    async fn start_rest_from_alerting() {
        let (service, _tx) = make_test_service();
        {
            let mut inner = service.inner.lock().await;
            inner.state = TimerState::Alerting;
        }

        let result = service.handle_user_event(UserEvent::StartRest).await;
        assert!(result.is_ok());

        let inner = service.inner.lock().await;
        assert_eq!(inner.state, TimerState::Resting);
    }

    #[tokio::test]
    async fn skip_from_alerting() {
        let (service, _tx) = make_test_service();
        {
            let mut inner = service.inner.lock().await;
            inner.state = TimerState::Alerting;
        }

        let result = service.handle_user_event(UserEvent::Skip).await;
        assert!(result.is_ok());

        let inner = service.inner.lock().await;
        assert_eq!(inner.state, TimerState::Working);
    }

    #[tokio::test]
    async fn pause_and_resume() {
        let (service, _tx) = make_test_service();
        service
            .handle_user_event(UserEvent::Pause)
            .await
            .expect("pause should succeed");

        {
            let inner = service.inner.lock().await;
            assert_eq!(inner.state, TimerState::Paused);
        }

        service
            .handle_user_event(UserEvent::Resume)
            .await
            .expect("resume should succeed");

        let inner = service.inner.lock().await;
        assert_eq!(inner.state, TimerState::Working);
    }

    #[tokio::test]
    async fn toggle_pause_pauses_then_resumes() {
        let (service, _tx) = make_test_service();

        service.toggle_pause().await.expect("toggle should pause");
        {
            let inner = service.inner.lock().await;
            assert_eq!(inner.state, TimerState::Paused);
        }

        service.toggle_pause().await.expect("toggle should resume");
        let inner = service.inner.lock().await;
        assert_eq!(inner.state, TimerState::Working);
    }

    #[tokio::test]
    async fn invalid_event_returns_error() {
        let (service, _tx) = make_test_service();
        let result = service.handle_user_event(UserEvent::Resume).await;
        assert!(matches!(
            result,
            Err(AppError::InvalidOperation { operation, reason })
                if operation == "timer event Resume" && reason == "invalid in Working"
        ));
    }

    #[tokio::test]
    async fn apply_config_updates_durations() {
        let (service, _tx) = make_test_service();
        let config = Config {
            timer: crate::models::config::TimerConfig {
                work_minutes: 30,
                rest_seconds: 40,
                pre_alert_seconds: 10,
                alert_timeout_seconds: 20,
                mode: crate::models::config::TimerMode::default(),
            },
            ..Config::default()
        };

        service.apply_config(&config).await;

        let inner = service.inner.lock().await;
        assert_eq!(inner.work_duration, Duration::from_mins(30));
        assert_eq!(inner.rest_duration, Duration::from_secs(40));
        assert_eq!(inner.pre_alert_duration, Duration::from_secs(10));
        assert_eq!(inner.alert_timeout, Duration::from_secs(20));
    }

    #[tokio::test]
    async fn state_snapshot_matches_inner() {
        let (service, _tx) = make_test_service();
        let snapshot = service.state_snapshot().await;

        assert_eq!(snapshot.state, "working");
        assert_eq!(snapshot.work_minutes, 20);
        assert_eq!(snapshot.rest_seconds, 20);
    }

    #[tokio::test]
    async fn state_snapshot_paused_returns_remaining() {
        let (service, _tx) = make_test_service();
        // Enter Working for some time, then Pause
        {
            let mut inner = service.inner.lock().await;
            inner.state_entered_at = past_instant(300); // 5 min worked
        }
        service
            .handle_user_event(UserEvent::Pause)
            .await
            .expect("pause should succeed");

        let snapshot = service.state_snapshot().await;
        assert_eq!(snapshot.state, "paused");
        // Should show remaining work time, not 0
        assert!(
            snapshot.remaining_secs > 0,
            "paused snapshot should preserve remaining time"
        );
    }
}
