#![allow(clippy::module_name_repetitions)]

use std::time::{Duration, Instant};

use serde::Serialize;

/// Timer states in the MVP state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimerState {
    Working,
    PreAlert,
    Alerting,
    Resting,
    Paused,
}

impl TimerState {
    /// Return the stable `snake_case` state name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
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
pub enum UserEvent {
    StartRest,
    Skip,
    Pause,
    Resume,
}

/// A state transition resolved by the timer core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    pub from: TimerState,
    pub to: TimerState,
}

/// Conditions that suppress the rest prompt and reset the work cycle.
#[derive(Debug, Clone, Default)]
pub struct SkipFlags {
    pub fullscreen_active: bool,
}

impl SkipFlags {
    /// Return whether any skip condition is active.
    #[must_use]
    pub const fn any_active(&self) -> bool {
        self.fullscreen_active
    }
}

/// Mutable timer runtime state used by higher-level services.
#[derive(Debug)]
pub struct Inner {
    pub state: TimerState,
    pub state_entered_at: Instant,
    pub work_duration: Duration,
    pub rest_duration: Duration,
    pub pre_alert_duration: Duration,
    pub alert_timeout: Duration,
}

impl Inner {
    /// Construct a timer state using config-derived durations.
    #[must_use]
    pub fn new(
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
        }
    }

    /// Apply a transition and reset the state timestamp.
    pub fn apply_transition(&mut self, transition: Transition) {
        self.state = transition.to;
        self.state_entered_at = Instant::now();
    }

    /// Return elapsed time in the current state.
    #[must_use]
    pub fn elapsed(&self, now: Instant) -> Duration {
        now.duration_since(self.state_entered_at)
    }

    /// Return the remaining time for timed states.
    #[must_use]
    pub fn remaining(&self, now: Instant) -> Option<Duration> {
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
