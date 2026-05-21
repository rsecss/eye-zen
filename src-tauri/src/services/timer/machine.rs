use std::time::Instant;

use crate::models::types::StatePayload;

use super::effect::{Effect, SoundType, TrayTooltip, TrayUpdate};
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
    }
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

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
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
}
