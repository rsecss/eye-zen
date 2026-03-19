#![allow(clippy::module_name_repetitions)]

use std::time::{Duration, Instant};

use serde::Serialize;

/// Timer states in the MVP state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TimerState {
    Working,
    PreAlert,
    Alerting,
    Resting,
    Paused,
}

impl TimerState {
    /// Return the stable `snake_case` state name.
    #[must_use]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::PreAlert => "pre_alert",
            Self::Alerting => "alerting",
            Self::Resting => "resting",
            Self::Paused => "paused",
        }
    }
}

/// User-driven events accepted by the timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UserEvent {
    StartRest,
    Skip,
    Pause,
    Resume,
}

/// A state transition resolved by the timer core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Transition {
    pub(crate) from: TimerState,
    pub(crate) to: TimerState,
}

/// Conditions that suppress the rest prompt and reset the work cycle.
#[derive(Debug, Clone, Default)]
pub(crate) struct SkipFlags {
    pub(crate) fullscreen_active: bool,
}

impl SkipFlags {
    /// Return whether any skip condition is active.
    #[must_use]
    pub(crate) const fn any_active(&self) -> bool {
        self.fullscreen_active
    }
}

/// Mutable timer runtime state used by higher-level services.
#[derive(Debug)]
pub(crate) struct Inner {
    pub(crate) state: TimerState,
    pub(crate) state_entered_at: Instant,
    pub(crate) work_duration: Duration,
    pub(crate) rest_duration: Duration,
    pub(crate) pre_alert_duration: Duration,
    pub(crate) alert_timeout: Duration,
    pub(crate) paused_from: Option<TimerState>,
    pub(crate) paused_remaining: Option<Duration>,
}

impl Inner {
    /// Construct a timer state using config-derived durations.
    #[must_use]
    pub(crate) fn new(
        work_minutes: u32,
        rest_seconds: u32,
        pre_alert_seconds: u32,
        alert_timeout_seconds: u32,
    ) -> Self {
        Self {
            state: TimerState::Working,
            state_entered_at: Instant::now(),
            work_duration: Duration::from_secs(u64::from(work_minutes) * 60),
            rest_duration: Duration::from_secs(u64::from(rest_seconds)),
            pre_alert_duration: Duration::from_secs(u64::from(pre_alert_seconds)),
            alert_timeout: Duration::from_secs(u64::from(alert_timeout_seconds)),
            paused_from: None,
            paused_remaining: None,
        }
    }

    /// Apply a transition and reset the state timestamp.
    pub(crate) fn apply_transition(&mut self, transition: Transition) {
        let now = Instant::now();

        if transition.to == TimerState::Paused {
            // Transitioning TO Paused: save origin state and remaining time
            self.paused_from = Some(transition.from);
            self.paused_remaining = self.remaining(now);
            self.state = transition.to;
            self.state_entered_at = now;
        } else if transition.from == TimerState::Paused {
            // Transitioning FROM Paused: restore saved state with remaining time
            let target_duration = match transition.to {
                TimerState::Working => {
                    self.work_duration.checked_sub(self.pre_alert_duration).unwrap_or(self.work_duration)
                }
                TimerState::PreAlert => self.pre_alert_duration,
                TimerState::Alerting => self.alert_timeout,
                TimerState::Resting => self.rest_duration,
                TimerState::Paused => unreachable!(),
            };

            if let Some(remaining) = self.paused_remaining {
                let elapsed = target_duration.saturating_sub(remaining);
                self.state_entered_at = now - elapsed;
            } else {
                self.state_entered_at = now;
            }

            self.state = transition.to;
            self.paused_from = None;
            self.paused_remaining = None;
        } else {
            // Normal transition: clear pause state
            self.paused_from = None;
            self.paused_remaining = None;
            self.state = transition.to;
            self.state_entered_at = now;
        }
    }

    /// Return elapsed time in the current state.
    #[must_use]
    pub(crate) fn elapsed(&self, now: Instant) -> Duration {
        now.duration_since(self.state_entered_at)
    }

    /// Return the remaining time for timed states.
    #[must_use]
    pub(crate) fn remaining(&self, now: Instant) -> Option<Duration> {
        let target = match self.state {
            TimerState::Working => self.work_duration.checked_sub(self.pre_alert_duration)?,
            TimerState::PreAlert => self.pre_alert_duration,
            TimerState::Alerting => self.alert_timeout,
            TimerState::Resting => self.rest_duration,
            TimerState::Paused => return None,
        };

        target.checked_sub(self.elapsed(now))
    }
}
