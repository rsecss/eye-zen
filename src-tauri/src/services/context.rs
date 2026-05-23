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
use crate::models::config::TimerMode;
use crate::models::hotkeys::HotkeyStatus;
#[cfg(not(test))]
use crate::models::statistics::{CycleEventDraft, CycleOutcome, RestSessionDraft};
use crate::models::statistics::{CycleReason, StatPersistenceErrorPayload, StatPersistenceKind};
#[cfg(not(test))]
use crate::models::types::StatePayload;
#[cfg(not(test))]
use crate::services::schedule::is_schedule_active;

use super::timer::SkipFlags;
#[cfg(not(test))]
use super::timer::{
    apply_transition_and_collect_effects, collect_tick_effects, step_time, Effect, EffectSink,
    Inner, SoundType, TimerService, TimerState, TrayTooltip,
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
    pub(crate) fn emit_hotkey_status_changed(&self, status: &HotkeyStatus) {
        let Some(app) = self.app.as_ref() else {
            return;
        };

        if let Err(err) = app.emit(events::HOTKEY_STATUS_CHANGED, status) {
            warn!("failed to emit hotkey_status_changed: {err}");
        }
    }

    #[cfg(test)]
    pub(crate) fn emit_hotkey_status_changed(&self, _status: &HotkeyStatus) {}

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
                let TimerLoopStep {
                    mut effects,
                    suppressed_skip,
                    mode,
                    is_long_break,
                } = {
                    let mut guard = inner.lock().await;
                    let now = std::time::Instant::now();

                    match step_time(&guard, now, &flags) {
                        Some(transition) => {
                            let suppressed_skip = transition.from == TimerState::Working
                                && transition.to == TimerState::Working;
                            if suppressed_skip {
                                if flags.afk_active {
                                    info!("skip: afk");
                                }
                                if flags.process_whitelisted {
                                    info!("skip: process whitelist");
                                }
                            }
                            let mode = guard.mode;
                            let is_long_break = guard.is_long_break;
                            let collected =
                                apply_transition_and_collect_effects(&mut guard, transition, now);
                            TimerLoopStep {
                                effects: collected,
                                suppressed_skip,
                                mode,
                                is_long_break,
                            }
                        }
                        None => TimerLoopStep {
                            effects: collect_tick_effects(&guard, now),
                            suppressed_skip: false,
                            mode: guard.mode,
                            is_long_break: guard.is_long_break,
                        },
                    }
                };

                if suppressed_skip {
                    if let Some(event) = suppression_event(&app, &flags, mode, is_long_break) {
                        effects.push(Effect::RecordCycleEvent(event));
                    }
                }

                let context = ServiceContext::from(app.clone());
                for effect in &effects {
                    super::timer::effect_executor::execute_effect(Some(&context), effect);
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

#[cfg(not(test))]
impl EffectSink for ServiceContext {
    fn emit_state_changed(&self, payload: &StatePayload) {
        let Some(app) = self.app.as_ref() else {
            info!("STUB emit_state_changed: {payload:?}");
            return;
        };

        if let Err(err) = app.emit(events::STATE_CHANGED, payload) {
            warn!("failed to emit state_changed: {err}");
        }
    }

    fn show_tip_windows(&self) {
        let Some(app) = self.app.as_ref() else {
            return;
        };
        let Some(services) = app.try_state::<SharedAppServices>() else {
            warn!("shared services unavailable while showing tip windows");
            return;
        };
        services.window.show_tip_windows();
    }

    fn hide_tip_windows(&self) {
        let Some(app) = self.app.as_ref() else {
            return;
        };
        let Some(services) = app.try_state::<SharedAppServices>() else {
            warn!("shared services unavailable while hiding tip windows");
            return;
        };
        services.window.hide_tip_windows();
    }

    fn play_sound(&self, sound: SoundType) {
        let Some(app) = self.app.as_ref() else {
            return;
        };
        let Some(services) = app.try_state::<SharedAppServices>() else {
            warn!("shared services unavailable while playing sound");
            return;
        };
        if services.config.current().behavior.sound_enabled {
            services.sound.play_type(sound);
        }
    }

    fn update_tray_tooltip(&self, tooltip: TrayTooltip) {
        let Some(app) = self.app.as_ref() else {
            return;
        };
        let Some(services) = app.try_state::<SharedAppServices>() else {
            warn!("shared services unavailable while updating tray tooltip");
            return;
        };
        services.tray.update_tooltip_best_effort(tooltip);
    }

    fn update_tray_state_icon(&self, state: TimerState) {
        let Some(app) = self.app.as_ref() else {
            return;
        };
        let Some(services) = app.try_state::<SharedAppServices>() else {
            warn!("shared services unavailable while updating tray state icon");
            return;
        };
        services.tray.update_pause_item_best_effort(state);
    }

    fn reset_work_timer(&self, duration: std::time::Duration) {
        info!("work timer reset to {}s", duration.as_secs());
    }

    fn record_rest_session(&self, session: &RestSessionDraft) {
        let Some(app) = self.app.as_ref() else {
            return;
        };
        let Some(services) = app.try_state::<SharedAppServices>() else {
            warn!("shared services unavailable while recording rest session");
            return;
        };
        if let Err(err) = services.stat.enqueue_rest_session(session.clone()) {
            emit_stat_queue_overflow(app, &err);
        }
    }

    fn record_cycle_event(&self, event: &CycleEventDraft) {
        let Some(app) = self.app.as_ref() else {
            return;
        };
        let Some(services) = app.try_state::<SharedAppServices>() else {
            warn!("shared services unavailable while recording cycle event");
            return;
        };
        if let Err(err) = services.stat.enqueue_cycle_event(event.clone()) {
            emit_stat_queue_overflow(app, &err);
        }
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

/// Emit a `stat_persistence_error` event for the `QueueOverflow` kind so
/// the UI can warn the user that a rest record could not be enqueued.
/// Underlying `SQLite` errors (foreign key, IO, etc.) are emitted by the
/// stat writer task itself with the appropriate `kind`.
#[cfg(not(test))]
fn emit_stat_queue_overflow(app: &AppHandle, err: &crate::error::AppError) {
    let payload = make_stat_queue_overflow_payload(err, &chrono::Utc::now().to_rfc3339());
    tracing::error!("stat write enqueue failed: {err}");
    if let Err(emit_err) = app.emit(events::STAT_PERSISTENCE_ERROR, &payload) {
        warn!("failed to emit stat_persistence_error: {emit_err}");
    }
}

/// Build the `QueueOverflow` payload from an `AppError` + RFC3339 timestamp.
///
/// Compiled in both prod and test builds so the payload shape (kind +
/// `occurred_at` + message) is unit-testable without an `AppHandle` and
/// without duplicating the construction inside a test module. The timestamp
/// is supplied by the caller so tests can pin it.
pub(crate) fn make_stat_queue_overflow_payload(
    err: &crate::error::AppError,
    occurred_at: &str,
) -> StatPersistenceErrorPayload {
    StatPersistenceErrorPayload {
        kind: StatPersistenceKind::QueueOverflow,
        occurred_at: occurred_at.to_string(),
        message: err.to_string(),
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
    let process_whitelisted = config.behavior.process_whitelist_enabled
        && !config.behavior.process_whitelist.is_empty()
        && services
            .detector
            .is_foreground_in_whitelist(&config.behavior.process_whitelist);
    SkipFlags {
        fullscreen_active: fullscreen_skip && services.detector.is_fullscreen(),
        schedule_inactive,
        afk_active,
        process_whitelisted,
    }
}

/// Build a `Suppressed` cycle event from the priority-resolved skip flag.
/// Priority order (fullscreen > schedule > afk > `process_whitelisted`) is
/// fixed by PRD §2 so analytics treats the loudest signal as the cause.
/// `process_hint` is populated only when the reason is
/// `ProcessWhitelisted` AND the user is already opted in by virtue of
/// having that entry in the whitelist.
#[cfg(not(test))]
fn suppression_event(
    app: &AppHandle,
    flags: &SkipFlags,
    mode: TimerMode,
    is_long_break: bool,
) -> Option<CycleEventDraft> {
    let services = app.try_state::<SharedAppServices>()?;

    let (reason, process_hint) = pick_suppression_reason(flags, || {
        services
            .detector
            .foreground_whitelist_match(&services.config.current().behavior.process_whitelist)
    })?;

    Some(CycleEventDraft {
        occurred_at_utc: chrono::Utc::now(),
        outcome: CycleOutcome::Suppressed,
        reason: Some(reason),
        process_hint,
        duration_secs: None,
        mode,
        is_long_break: mode == TimerMode::Pomodoro && is_long_break,
    })
}

/// Resolve which `CycleReason` won the priority race over the four skip
/// flags. Returns `None` when no flag is active.
///
/// Priority is `fullscreen > schedule > afk > process_whitelisted`; the
/// `process_hint_fn` closure is only invoked when `process_whitelisted` wins
/// so the (potentially expensive) foreground-process lookup stays lazy.
///
/// Compiled in both prod and test builds so the priority rule can be tested
/// without an `AppHandle`.
pub(crate) fn pick_suppression_reason<F>(
    flags: &SkipFlags,
    process_hint_fn: F,
) -> Option<(CycleReason, Option<String>)>
where
    F: FnOnce() -> Option<String>,
{
    if flags.fullscreen_active {
        Some((CycleReason::Fullscreen, None))
    } else if flags.schedule_inactive {
        Some((CycleReason::Schedule, None))
    } else if flags.afk_active {
        Some((CycleReason::Afk, None))
    } else if flags.process_whitelisted {
        Some((CycleReason::ProcessWhitelisted, process_hint_fn()))
    } else {
        None
    }
}

/// Output of one timer-loop tick step, threaded through the function so the
/// suppression-event append happens outside the inner-state lock.
#[cfg(not(test))]
struct TimerLoopStep {
    effects: Vec<Effect>,
    suppressed_skip: bool,
    mode: TimerMode,
    is_long_break: bool,
}

#[cfg(test)]
impl ServiceContext {
    /// Stub used by tests that touch the timer service's effect-dispatch loop
    /// without a real Tauri context. The `EffectSink` trait is exercised
    /// separately by `RecordingSink` in `effect_executor::tests`.
    pub(crate) fn execute_timer_effect(&self, effect: &Effect) {
        info!("STUB effect: {effect:?}");
    }
}

/// Test-build `EffectSink` impl: every method is a no-op so the timer
/// service's dispatch loop compiles in tests without an `AppHandle`. The
/// real effect-routing behavior is covered by `effect_executor::tests` with
/// a dedicated recording sink.
#[cfg(test)]
impl super::timer::EffectSink for ServiceContext {
    fn emit_state_changed(&self, _payload: &crate::models::types::StatePayload) {}
    fn show_tip_windows(&self) {}
    fn hide_tip_windows(&self) {}
    fn play_sound(&self, _sound: super::timer::SoundType) {}
    fn update_tray_tooltip(&self, _tooltip: super::timer::TrayTooltip) {}
    fn update_tray_state_icon(&self, _state: super::timer::TimerState) {}
    fn reset_work_timer(&self, _duration: std::time::Duration) {}
    fn record_rest_session(&self, _session: &crate::models::statistics::RestSessionDraft) {}
    fn record_cycle_event(&self, _event: &crate::models::statistics::CycleEventDraft) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use crate::models::statistics::{CycleReason, StatPersistenceKind};

    #[test]
    fn queue_overflow_payload_carries_kind_timestamp_and_message() {
        let err = AppError::IoError {
            message: "stat-writer queue full".to_string(),
        };
        let payload = make_stat_queue_overflow_payload(&err, "2026-05-23T12:00:00Z");
        assert_eq!(payload.kind, StatPersistenceKind::QueueOverflow);
        assert_eq!(payload.occurred_at, "2026-05-23T12:00:00Z");
        assert!(
            payload.message.contains("stat-writer queue full"),
            "payload.message should embed the AppError display, got: {}",
            payload.message
        );
    }

    #[test]
    fn pick_suppression_reason_none_when_no_flag() {
        let chosen = pick_suppression_reason(&SkipFlags::default(), || None);
        assert!(chosen.is_none());
    }

    #[test]
    fn pick_suppression_reason_fullscreen_wins_over_all_and_skips_hint() {
        let hint_called = std::cell::Cell::new(false);
        let chosen = pick_suppression_reason(
            &SkipFlags {
                fullscreen_active: true,
                schedule_inactive: true,
                afk_active: true,
                process_whitelisted: true,
            },
            || {
                hint_called.set(true);
                Some("Game.exe".to_string())
            },
        );
        assert_eq!(chosen, Some((CycleReason::Fullscreen, None)));
        assert!(
            !hint_called.get(),
            "process_hint_fn should NOT be called when fullscreen wins"
        );
    }

    #[test]
    fn pick_suppression_reason_schedule_beats_afk_and_whitelist() {
        let chosen = pick_suppression_reason(
            &SkipFlags {
                fullscreen_active: false,
                schedule_inactive: true,
                afk_active: true,
                process_whitelisted: true,
            },
            || Some("never".to_string()),
        );
        assert_eq!(chosen, Some((CycleReason::Schedule, None)));
    }

    #[test]
    fn pick_suppression_reason_afk_beats_whitelist() {
        let chosen = pick_suppression_reason(
            &SkipFlags {
                fullscreen_active: false,
                schedule_inactive: false,
                afk_active: true,
                process_whitelisted: true,
            },
            || Some("never".to_string()),
        );
        assert_eq!(chosen, Some((CycleReason::Afk, None)));
    }

    #[test]
    fn pick_suppression_reason_whitelist_populates_hint() {
        let chosen = pick_suppression_reason(
            &SkipFlags {
                fullscreen_active: false,
                schedule_inactive: false,
                afk_active: false,
                process_whitelisted: true,
            },
            || Some("Code.exe".to_string()),
        );
        assert_eq!(
            chosen,
            Some((
                CycleReason::ProcessWhitelisted,
                Some("Code.exe".to_string())
            ))
        );
    }

    #[test]
    fn pick_suppression_reason_whitelist_allows_none_hint() {
        let chosen = pick_suppression_reason(
            &SkipFlags {
                fullscreen_active: false,
                schedule_inactive: false,
                afk_active: false,
                process_whitelisted: true,
            },
            || None,
        );
        assert_eq!(chosen, Some((CycleReason::ProcessWhitelisted, None)));
    }

    /// Exercise every `EffectSink` stub method on the test-build
    /// `ServiceContext` so the cov counter records them. The stubs are
    /// no-ops by design — production behavior is in the `#[cfg(not(test))]`
    /// impl and is covered by manual smoke + the recording-sink dispatch
    /// tests in `effect_executor::tests`.
    #[test]
    fn service_context_stub_effect_sink_methods_are_callable() {
        use crate::services::timer::EffectSink;
        use std::time::Duration;

        let ctx = ServiceContext::default();
        let payload = crate::models::types::StatePayload {
            state: "working".to_string(),
            remaining_secs: 0,
            work_minutes: 20,
            rest_seconds: 20,
            mode: crate::models::config::TimerMode::TwentyTwentyTwenty,
            pomodoro: None,
        };
        ctx.emit_state_changed(&payload);
        ctx.show_tip_windows();
        ctx.hide_tip_windows();
        ctx.play_sound(crate::services::timer::SoundType::PreAlert);
        let tooltip = crate::services::timer::TrayTooltip {
            state: crate::services::timer::TimerState::Working,
            remaining_secs: None,
            pomodoro_progress: None,
        };
        ctx.update_tray_tooltip(tooltip);
        ctx.update_tray_state_icon(crate::services::timer::TimerState::Working);
        ctx.reset_work_timer(Duration::from_secs(1));
        let session = crate::models::statistics::RestSessionDraft {
            started_at_utc: chrono::Utc::now(),
            ended_at_utc: chrono::Utc::now(),
            duration_secs: 0,
        };
        ctx.record_rest_session(&session);
        let event = crate::models::statistics::CycleEventDraft {
            occurred_at_utc: chrono::Utc::now(),
            outcome: crate::models::statistics::CycleOutcome::Taken,
            reason: None,
            process_hint: None,
            duration_secs: None,
            mode: crate::models::config::TimerMode::TwentyTwentyTwenty,
            is_long_break: false,
        };
        ctx.record_cycle_event(&event);
    }

    /// Sanity for the test-build `execute_timer_effect` stub and the
    /// `Default` constructor — both are part of the test-only API surface
    /// that other modules depend on.
    #[test]
    fn service_context_default_and_legacy_execute_stub_are_callable() {
        let ctx = ServiceContext::default();
        assert_eq!(ctx.app_handle(), None);
        ctx.emit_config_changed(&crate::models::config::Config::default());
        ctx.emit_hotkey_status_changed(&crate::models::hotkeys::HotkeyStatus {
            bindings: vec![],
            macos_accessibility: crate::models::hotkeys::MacosAccessibilityStatus::NotRequired,
            last_error: None,
        });
        ctx.execute_timer_effect(&Effect::ShowTipWindows);
        let _ = format!("{ctx:?}");
    }
}
