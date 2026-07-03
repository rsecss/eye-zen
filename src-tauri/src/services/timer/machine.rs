use std::time::Instant;

use chrono::Utc;

use crate::models::config::TimerMode;
use crate::models::statistics::{CycleEventDraft, CycleOutcome, RestSessionDraft};
use crate::models::timer::{PomodoroStatePayload, StatePayload};

use super::effect::{Effect, PomodoroProgress, SoundType, TrayTooltip, TrayUpdate};
use super::state::{Inner, SkipFlags, TimerState, Transition, UserEvent};

/// Resolve a user action into a valid state transition.
#[allow(clippy::trivially_copy_pass_by_ref)]
#[must_use]
pub(crate) fn resolve_user_event(
    state: &TimerState,
    event: UserEvent,
    paused_from: Option<TimerState>,
) -> Option<Transition> {
    use TimerState::{Alerting, Paused, PreAlert, Resting, Working};
    use UserEvent::{Pause, Resume, Skip, StartRest};

    let to = match (*state, event) {
        (Working | PreAlert | Alerting, StartRest) => Resting,
        (Alerting, Skip) => Working,
        (Paused, Resume) => paused_from.unwrap_or(Working),
        (Working | PreAlert | Alerting | Resting, Pause) => Paused,
        _ => return None,
    };

    Some(Transition { from: *state, to })
}

/// Resolve timeout-driven state changes.
#[must_use]
pub(crate) fn step_time(inner: &Inner, now: Instant, skip_flags: &SkipFlags) -> Option<Transition> {
    use TimerState::{Alerting, Paused, PreAlert, Resting, Working};

    let elapsed = inner.elapsed(now);

    match inner.state {
        Working => {
            let work_threshold = inner
                .work_duration
                .checked_sub(inner.pre_alert_duration)
                .unwrap_or(inner.work_duration);

            if elapsed >= work_threshold {
                if skip_flags.any_active() {
                    Some(Transition {
                        from: Working,
                        to: Working,
                    })
                } else {
                    Some(Transition {
                        from: Working,
                        to: PreAlert,
                    })
                }
            } else {
                None
            }
        }
        PreAlert => (elapsed >= inner.pre_alert_duration).then_some(Transition {
            from: PreAlert,
            to: Alerting,
        }),
        Alerting => (elapsed >= inner.alert_timeout).then_some(Transition {
            from: Alerting,
            to: Resting,
        }),
        Resting => (elapsed >= inner.rest_duration).then_some(Transition {
            from: Resting,
            to: Working,
        }),
        Paused => None,
    }
}

/// Collect effects for a completed transition.
#[must_use]
pub(crate) fn collect_effects(transition: Transition, inner: &Inner, now: Instant) -> Vec<Effect> {
    use TimerState::{Alerting, Paused, PreAlert, Resting, Working};

    let mut effects = vec![Effect::EmitStateChanged(state_payload(inner, now))];

    match (transition.from, transition.to) {
        (Working, PreAlert) => {
            effects.push(Effect::PlaySound(SoundType::PreAlert));
        }
        (PreAlert, Alerting) => {
            effects.push(Effect::ShowTipWindows);
        }
        (Working | PreAlert, Resting) => {
            effects.push(Effect::ShowTipWindows);
            effects.push(Effect::UpdateTray(TrayUpdate::StateIcon(Resting)));
        }
        (Alerting, Resting) => {
            effects.push(Effect::UpdateTray(TrayUpdate::StateIcon(Resting)));
        }
        (Alerting, Working) => {
            effects.push(Effect::HideTipWindows);
            effects.push(Effect::ResetWorkTimer(inner.work_duration));
            effects.push(Effect::UpdateTray(TrayUpdate::StateIcon(Working)));
        }
        (Resting, Working) => {
            effects.push(Effect::HideTipWindows);
            effects.push(Effect::PlaySound(SoundType::RestComplete));
            effects.push(Effect::ResetWorkTimer(inner.work_duration));
            effects.push(Effect::UpdateTray(TrayUpdate::StateIcon(Working)));
        }
        (_, Paused) => {
            if matches!(transition.from, Alerting | Resting) {
                effects.push(Effect::HideTipWindows);
            }
            effects.push(Effect::UpdateTray(TrayUpdate::StateIcon(Paused)));
        }
        (Paused, Working) => {
            effects.push(Effect::ResetWorkTimer(inner.work_duration));
            effects.push(Effect::UpdateTray(TrayUpdate::StateIcon(Working)));
        }
        (Working, Working) => {
            effects.push(Effect::ResetWorkTimer(inner.work_duration));
        }
        _ => {}
    }

    effects.push(Effect::UpdateTray(TrayUpdate::Tooltip(tray_tooltip(
        inner, now,
    ))));

    effects
}

/// Apply a transition and collect the side effects that result from it.
#[must_use]
pub(crate) fn apply_transition_and_collect_effects(
    inner: &mut Inner,
    transition: Transition,
    now: Instant,
) -> Vec<Effect> {
    let rest_session = rest_session_from_transition(transition, inner, now);
    let cycle_event = cycle_event_from_transition(transition, inner, now);
    inner.apply_transition_at(transition, now);
    let mut effects = collect_effects(transition, inner, now);
    if let Some(session) = rest_session {
        effects.push(Effect::RecordRestSession(session));
    }
    if let Some(event) = cycle_event {
        effects.push(Effect::RecordCycleEvent(event));
    }
    effects
}

/// Collect periodic effects for the current state without a transition.
#[must_use]
pub(crate) fn collect_tick_effects(inner: &Inner, now: Instant) -> Vec<Effect> {
    vec![
        Effect::EmitStateChanged(state_payload(inner, now)),
        Effect::UpdateTray(TrayUpdate::Tooltip(tray_tooltip(inner, now))),
    ]
}

#[must_use]
fn state_payload(inner: &Inner, now: Instant) -> StatePayload {
    let remaining_secs = if inner.state == TimerState::Paused {
        duration_to_secs(inner.paused_remaining)
    } else {
        duration_to_secs(inner.remaining(now))
    };
    StatePayload {
        state: inner.state.as_str().to_string(),
        remaining_secs,
        work_minutes: duration_minutes_to_u32(inner.work_duration),
        rest_seconds: duration_to_secs(Some(inner.rest_duration)),
        mode: inner.mode,
        pomodoro: pomodoro_payload(inner),
    }
}

#[must_use]
fn pomodoro_payload(inner: &Inner) -> Option<PomodoroStatePayload> {
    if inner.mode != TimerMode::Pomodoro {
        return None;
    }
    Some(PomodoroStatePayload {
        cycle_index: inner.cycle_index,
        cycles_per_long: inner.cycles_per_long,
        is_long_break: inner.state == TimerState::Resting && inner.is_long_break,
    })
}

#[must_use]
fn tray_tooltip(inner: &Inner, now: Instant) -> TrayTooltip {
    let remaining_secs = if inner.state == TimerState::Paused {
        None
    } else {
        Some(duration_to_secs(inner.remaining(now)))
    };

    TrayTooltip {
        state: inner.state,
        remaining_secs,
        pomodoro_progress: if inner.mode == TimerMode::Pomodoro {
            Some(PomodoroProgress {
                current: inner.cycle_index,
                total: inner.cycles_per_long,
            })
        } else {
            None
        },
    }
}

fn duration_to_secs(duration: Option<std::time::Duration>) -> u32 {
    duration.map_or(0, |value| {
        u32::try_from(value.as_secs()).unwrap_or(u32::MAX)
    })
}

fn duration_minutes_to_u32(duration: std::time::Duration) -> u32 {
    u32::try_from(duration.as_secs() / 60).unwrap_or(u32::MAX)
}

fn rest_session_from_transition(
    transition: Transition,
    inner: &Inner,
    now: Instant,
) -> Option<RestSessionDraft> {
    if transition.from != TimerState::Resting || transition.to != TimerState::Working {
        return None;
    }

    let duration_secs = duration_to_secs(Some(inner.elapsed(now)));
    let ended_at_utc = Utc::now();
    let started_at_utc = ended_at_utc - chrono::Duration::seconds(i64::from(duration_secs));

    Some(RestSessionDraft {
        started_at_utc,
        ended_at_utc,
        duration_secs,
    })
}

/// Build a `taken` or `skipped` cycle event from a transition. Suppressed
/// events (Working -> Working with skip flags) are emitted from the timer
/// loop, not here, because the loop owns the flag-priority decision.
///
/// MUST be called before `inner.apply_transition_at` so the Pomodoro
/// `is_long_break` snapshot reflects the rest cycle that just finished
/// rather than the next one.
fn cycle_event_from_transition(
    transition: Transition,
    inner: &Inner,
    now: Instant,
) -> Option<CycleEventDraft> {
    let (outcome, duration_secs) = match (transition.from, transition.to) {
        (TimerState::Resting, TimerState::Working) => (
            CycleOutcome::Taken,
            Some(duration_to_secs(Some(inner.elapsed(now)))),
        ),
        (TimerState::Alerting, TimerState::Working) => (CycleOutcome::Skipped, None),
        _ => return None,
    };

    Some(CycleEventDraft {
        occurred_at_utc: Utc::now(),
        outcome,
        reason: None,
        process_hint: None,
        duration_secs,
        mode: inner.mode,
        is_long_break: inner.mode == TimerMode::Pomodoro && inner.is_long_break,
    })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::models::statistics::CycleOutcome;
    use crate::services::timer::state::TimerState::{Alerting, Paused, PreAlert, Resting, Working};

    fn make_inner(state: TimerState) -> Inner {
        let mut inner = Inner::new(20, 20, 15, 60);
        inner.state = state;
        inner
    }

    fn future_instant(seconds: u64) -> Instant {
        Instant::now() + Duration::from_secs(seconds)
    }

    #[test]
    fn alerting_start_rest() {
        let transition = resolve_user_event(&Alerting, UserEvent::StartRest, None);
        assert_eq!(
            transition,
            Some(Transition {
                from: Alerting,
                to: Resting,
            })
        );
    }

    #[test]
    fn alerting_skip() {
        let transition = resolve_user_event(&Alerting, UserEvent::Skip, None);
        assert_eq!(
            transition,
            Some(Transition {
                from: Alerting,
                to: Working,
            })
        );
    }

    #[test]
    fn working_pause() {
        let transition = resolve_user_event(&Working, UserEvent::Pause, None);
        assert_eq!(
            transition,
            Some(Transition {
                from: Working,
                to: Paused,
            })
        );
    }

    #[test]
    fn paused_resume_defaults_to_working() {
        let transition = resolve_user_event(&Paused, UserEvent::Resume, None);
        assert_eq!(
            transition,
            Some(Transition {
                from: Paused,
                to: Working,
            })
        );
    }

    #[test]
    fn paused_resume_from_resting() {
        let transition = resolve_user_event(&Paused, UserEvent::Resume, Some(Resting));
        assert_eq!(
            transition,
            Some(Transition {
                from: Paused,
                to: Resting,
            })
        );
    }

    #[test]
    fn working_start_rest_enters_resting() {
        let transition = resolve_user_event(&Working, UserEvent::StartRest, None);
        assert_eq!(
            transition,
            Some(Transition {
                from: Working,
                to: Resting,
            })
        );
    }

    #[test]
    fn pre_alert_start_rest_enters_resting() {
        let transition = resolve_user_event(&PreAlert, UserEvent::StartRest, None);
        assert_eq!(
            transition,
            Some(Transition {
                from: PreAlert,
                to: Resting,
            })
        );
    }

    #[test]
    fn paused_pause_invalid() {
        assert_eq!(resolve_user_event(&Paused, UserEvent::Pause, None), None);
    }

    #[test]
    fn resting_pause() {
        let transition = resolve_user_event(&Resting, UserEvent::Pause, None);
        assert_eq!(
            transition,
            Some(Transition {
                from: Resting,
                to: Paused,
            })
        );
    }

    #[test]
    fn pre_alert_pause() {
        let transition = resolve_user_event(&PreAlert, UserEvent::Pause, None);
        assert_eq!(
            transition,
            Some(Transition {
                from: PreAlert,
                to: Paused,
            })
        );
    }

    #[test]
    fn alerting_pause() {
        let transition = resolve_user_event(&Alerting, UserEvent::Pause, None);
        assert_eq!(
            transition,
            Some(Transition {
                from: Alerting,
                to: Paused,
            })
        );
    }

    #[test]
    fn working_timeout_enters_pre_alert() {
        let inner = make_inner(Working);

        let transition = step_time(&inner, future_instant(20 * 60), &SkipFlags::default());
        assert_eq!(
            transition,
            Some(Transition {
                from: Working,
                to: PreAlert,
            })
        );
    }

    #[test]
    fn working_not_yet_timeout() {
        let inner = make_inner(Working);
        assert_eq!(
            step_time(&inner, Instant::now(), &SkipFlags::default()),
            None
        );
    }

    #[test]
    fn working_timeout_with_fullscreen_skip_resets() {
        let inner = make_inner(Working);
        let flags = SkipFlags {
            fullscreen_active: true,
            ..SkipFlags::default()
        };

        let transition = step_time(&inner, future_instant(20 * 60), &flags);
        assert_eq!(
            transition,
            Some(Transition {
                from: Working,
                to: Working,
            })
        );
    }

    #[test]
    fn working_timeout_with_schedule_inactive_resets() {
        let inner = make_inner(Working);
        let flags = SkipFlags {
            schedule_inactive: true,
            ..SkipFlags::default()
        };

        let transition = step_time(&inner, future_instant(20 * 60), &flags);
        assert_eq!(
            transition,
            Some(Transition {
                from: Working,
                to: Working,
            })
        );
    }

    #[test]
    fn working_timeout_with_afk_skip_resets() {
        let inner = make_inner(Working);
        let flags = SkipFlags {
            afk_active: true,
            ..SkipFlags::default()
        };

        let transition = step_time(&inner, future_instant(20 * 60), &flags);
        assert_eq!(
            transition,
            Some(Transition {
                from: Working,
                to: Working,
            })
        );
    }

    #[test]
    fn skip_flags_any_active_includes_afk() {
        let flags = SkipFlags {
            afk_active: true,
            ..SkipFlags::default()
        };

        assert!(flags.any_active());
    }

    #[test]
    fn pre_alert_timeout_enters_alerting() {
        let inner = make_inner(PreAlert);

        let transition = step_time(&inner, future_instant(16), &SkipFlags::default());
        assert_eq!(
            transition,
            Some(Transition {
                from: PreAlert,
                to: Alerting,
            })
        );
    }

    #[test]
    fn alerting_timeout_auto_rest() {
        let inner = make_inner(Alerting);

        let transition = step_time(&inner, future_instant(61), &SkipFlags::default());
        assert_eq!(
            transition,
            Some(Transition {
                from: Alerting,
                to: Resting,
            })
        );
    }

    #[test]
    fn resting_timeout_returns_to_working() {
        let inner = make_inner(Resting);

        let transition = step_time(&inner, future_instant(21), &SkipFlags::default());
        assert_eq!(
            transition,
            Some(Transition {
                from: Resting,
                to: Working,
            })
        );
    }

    #[test]
    fn paused_never_times_out() {
        let inner = make_inner(Paused);
        assert_eq!(
            step_time(&inner, future_instant(3_600), &SkipFlags::default()),
            None
        );
    }

    #[test]
    fn effects_working_to_pre_alert() {
        let inner = make_inner(PreAlert);
        let effects = collect_effects(
            Transition {
                from: Working,
                to: PreAlert,
            },
            &inner,
            Instant::now(),
        );

        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::EmitStateChanged(_))));
        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::PlaySound(SoundType::PreAlert))));
        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::UpdateTray(_))));
    }

    #[test]
    fn tick_effects_emit_state_and_countdown_tooltip() {
        let inner = make_inner(Resting);

        let effects = collect_tick_effects(&inner, future_instant(5));

        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::EmitStateChanged(StatePayload {
                state,
                remaining_secs: 14..=15,
                ..
            }) if state == "resting"
        )));
        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::UpdateTray(TrayUpdate::Tooltip(TrayTooltip {
                state: Resting,
                remaining_secs: Some(14..=15),
                ..
            }))
        )));
    }

    #[test]
    fn effects_pre_alert_to_alerting() {
        let inner = make_inner(Alerting);
        let effects = collect_effects(
            Transition {
                from: PreAlert,
                to: Alerting,
            },
            &inner,
            Instant::now(),
        );

        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::ShowTipWindows)));
        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::EmitStateChanged(_))));
    }

    #[test]
    fn effects_alerting_to_resting() {
        let inner = make_inner(Resting);
        let effects = collect_effects(
            Transition {
                from: Alerting,
                to: Resting,
            },
            &inner,
            Instant::now(),
        );

        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::EmitStateChanged(_))));
        assert!(!effects
            .iter()
            .any(|effect| matches!(effect, Effect::HideTipWindows)));
    }

    #[test]
    fn effects_working_to_resting_show_tip_windows() {
        let inner = make_inner(Resting);
        let effects = collect_effects(
            Transition {
                from: Working,
                to: Resting,
            },
            &inner,
            Instant::now(),
        );

        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::ShowTipWindows)));
        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::UpdateTray(TrayUpdate::StateIcon(Resting)))));
    }

    #[test]
    fn effects_resting_to_working() {
        let inner = make_inner(Working);
        let effects = collect_effects(
            Transition {
                from: Resting,
                to: Working,
            },
            &inner,
            Instant::now(),
        );

        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::HideTipWindows)));
        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::PlaySound(SoundType::RestComplete))));
        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::ResetWorkTimer(_))));
    }

    #[test]
    fn transition_resting_to_working_records_rest_session() {
        let mut inner = make_inner(Resting);
        let now = future_instant(21);
        let effects = apply_transition_and_collect_effects(
            &mut inner,
            Transition {
                from: Resting,
                to: Working,
            },
            now,
        );

        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::RecordRestSession(RestSessionDraft {
                duration_secs: 20..=21,
                ..
            })
        )));
    }

    #[test]
    fn transition_resting_to_working_records_taken_cycle_event() {
        let mut inner = make_inner(Resting);
        let now = future_instant(21);
        let effects = apply_transition_and_collect_effects(
            &mut inner,
            Transition {
                from: Resting,
                to: Working,
            },
            now,
        );

        let event = effects
            .iter()
            .find_map(|effect| match effect {
                Effect::RecordCycleEvent(draft) => Some(draft.clone()),
                _ => None,
            })
            .expect("cycle event should be emitted");
        assert_eq!(event.outcome, CycleOutcome::Taken);
        assert!(event.reason.is_none());
        assert!(event.duration_secs.is_some());
        assert!(!event.is_long_break);
    }

    #[test]
    fn transition_alerting_to_working_records_skipped_cycle_event() {
        let mut inner = make_inner(Alerting);
        let effects = apply_transition_and_collect_effects(
            &mut inner,
            Transition {
                from: Alerting,
                to: Working,
            },
            Instant::now(),
        );

        let event = effects
            .iter()
            .find_map(|effect| match effect {
                Effect::RecordCycleEvent(draft) => Some(draft.clone()),
                _ => None,
            })
            .expect("skipped cycle event should be emitted");
        assert_eq!(event.outcome, CycleOutcome::Skipped);
        assert!(event.reason.is_none());
        assert!(event.duration_secs.is_none());
    }

    #[test]
    fn working_to_working_skip_does_not_record_cycle_event_from_machine() {
        let mut inner = make_inner(Working);
        let effects = apply_transition_and_collect_effects(
            &mut inner,
            Transition {
                from: Working,
                to: Working,
            },
            Instant::now(),
        );

        // Suppression events are emitted from the timer loop, not from the
        // pure transition pipeline (the loop owns the flag-priority decision).
        assert!(!effects
            .iter()
            .any(|effect| matches!(effect, Effect::RecordCycleEvent(_))));
    }

    #[test]
    fn effects_alerting_to_working_skip() {
        let inner = make_inner(Working);
        let effects = collect_effects(
            Transition {
                from: Alerting,
                to: Working,
            },
            &inner,
            Instant::now(),
        );

        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::HideTipWindows)));
        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::ResetWorkTimer(_))));
    }

    #[test]
    fn effects_any_to_paused() {
        let inner = make_inner(Paused);
        let effects = collect_effects(
            Transition {
                from: Working,
                to: Paused,
            },
            &inner,
            Instant::now(),
        );

        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::EmitStateChanged(_))));
        assert!(effects
            .iter()
            .any(|effect| matches!(effect, Effect::UpdateTray(_))));
    }

    fn make_pomodoro_inner(cycle_index: u32, cycles_per_long: u32) -> Inner {
        use crate::models::config::{Config, PomodoroConfig, TimerMode};
        let mut config = Config::default();
        config.timer.mode = TimerMode::Pomodoro;
        config.pomodoro = PomodoroConfig {
            focus_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            cycles_per_long,
        };
        let mut inner = Inner::from_config(&config);
        inner.cycle_index = cycle_index;
        inner
    }

    #[test]
    fn pomodoro_entering_resting_picks_short_break_for_non_long_cycle() {
        let mut inner = make_pomodoro_inner(1, 4);
        let now = Instant::now();
        let transition = Transition {
            from: Working,
            to: Resting,
        };
        inner.apply_transition_at(transition, now);

        assert_eq!(inner.rest_duration, std::time::Duration::from_mins(5));
        assert!(!inner.is_long_break);
    }

    #[test]
    fn pomodoro_entering_resting_picks_long_break_at_cycle_boundary() {
        let mut inner = make_pomodoro_inner(4, 4);
        let now = Instant::now();
        let transition = Transition {
            from: Alerting,
            to: Resting,
        };
        inner.apply_transition_at(transition, now);

        assert_eq!(inner.rest_duration, std::time::Duration::from_mins(15));
        assert!(inner.is_long_break);
    }

    #[test]
    fn pomodoro_resting_to_working_advances_cycle_on_short_break() {
        let mut inner = make_pomodoro_inner(2, 4);
        inner.is_long_break = false;
        inner.state = Resting;
        let now = Instant::now();
        inner.apply_transition_at(
            Transition {
                from: Resting,
                to: Working,
            },
            now,
        );

        assert_eq!(inner.cycle_index, 3);
        assert!(!inner.is_long_break);
    }

    #[test]
    fn pomodoro_resting_to_working_resets_cycle_after_long_break() {
        let mut inner = make_pomodoro_inner(4, 4);
        inner.is_long_break = true;
        inner.state = Resting;
        let now = Instant::now();
        inner.apply_transition_at(
            Transition {
                from: Resting,
                to: Working,
            },
            now,
        );

        assert_eq!(inner.cycle_index, 1);
        assert!(!inner.is_long_break);
    }

    #[test]
    fn pomodoro_state_payload_includes_progress() {
        let inner = make_pomodoro_inner(2, 4);
        let effects = collect_tick_effects(&inner, Instant::now());
        let payload = effects.iter().find_map(|effect| match effect {
            Effect::EmitStateChanged(payload) => Some(payload.clone()),
            _ => None,
        });

        let payload = payload.expect("state payload should be emitted");
        assert_eq!(payload.mode, crate::models::config::TimerMode::Pomodoro);
        let pomodoro = payload
            .pomodoro
            .expect("pomodoro payload should be present");
        assert_eq!(pomodoro.cycle_index, 2);
        assert_eq!(pomodoro.cycles_per_long, 4);
        assert!(!pomodoro.is_long_break);
    }

    #[test]
    fn pomodoro_is_long_break_only_during_long_resting() {
        let mut inner = make_pomodoro_inner(4, 4);
        inner.apply_transition_at(
            Transition {
                from: Working,
                to: Resting,
            },
            Instant::now(),
        );

        let payload = state_payload(&inner, Instant::now());
        let pomodoro = payload
            .pomodoro
            .expect("pomodoro payload should be present");
        assert!(pomodoro.is_long_break);

        // After long break completes, is_long_break clears and cycle resets.
        inner.apply_transition_at(
            Transition {
                from: Resting,
                to: Working,
            },
            Instant::now(),
        );
        let payload = state_payload(&inner, Instant::now());
        let pomodoro = payload
            .pomodoro
            .expect("pomodoro payload should be present");
        assert!(!pomodoro.is_long_break);
        assert_eq!(pomodoro.cycle_index, 1);
    }

    #[test]
    fn twenty_twenty_twenty_payload_has_no_pomodoro() {
        let inner = make_inner(Working);
        let payload = state_payload(&inner, Instant::now());
        assert_eq!(
            payload.mode,
            crate::models::config::TimerMode::TwentyTwentyTwenty
        );
        assert!(payload.pomodoro.is_none());
    }

    #[test]
    fn pomodoro_tray_tooltip_carries_progress() {
        let inner = make_pomodoro_inner(3, 4);
        let tooltip = tray_tooltip(&inner, Instant::now());
        let progress = tooltip
            .pomodoro_progress
            .expect("pomodoro progress should be set");
        assert_eq!(progress.current, 3);
        assert_eq!(progress.total, 4);
    }

    #[test]
    fn pomodoro_pause_resume_preserves_cycle() {
        let mut inner = make_pomodoro_inner(3, 4);
        inner.state = Working;
        let now = Instant::now();
        inner.apply_transition_at(
            Transition {
                from: Working,
                to: Paused,
            },
            now,
        );
        assert_eq!(inner.cycle_index, 3);

        inner.apply_transition_at(
            Transition {
                from: Paused,
                to: Working,
            },
            now,
        );
        assert_eq!(inner.cycle_index, 3);
    }
}
