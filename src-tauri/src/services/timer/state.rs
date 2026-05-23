#![allow(clippy::module_name_repetitions)]

use std::time::{Duration, Instant};

use serde::Serialize;

#[cfg(test)]
use crate::models::config::PomodoroConfig;
use crate::models::config::{Config, TimerMode};

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
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct SkipFlags {
    pub(crate) fullscreen_active: bool,
    pub(crate) schedule_inactive: bool,
    pub(crate) afk_active: bool,
    pub(crate) process_whitelisted: bool,
}

impl SkipFlags {
    /// Return whether any skip condition is active.
    #[must_use]
    pub(crate) const fn any_active(&self) -> bool {
        self.fullscreen_active
            || self.schedule_inactive
            || self.afk_active
            || self.process_whitelisted
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
    pub(crate) mode: TimerMode,
    /// Pomodoro runtime. Defaults are kept even in 20-20-20 mode so we can
    /// swap modes without re-allocating; only consulted when `mode ==
    /// TimerMode::Pomodoro`.
    pub(crate) cycle_index: u32,
    pub(crate) cycles_per_long: u32,
    pub(crate) short_break_duration: Duration,
    pub(crate) long_break_duration: Duration,
    /// True while the current Resting is the long break (cycle %
    /// `cycles_per_long` == 0 at entry). Cleared when entering Working.
    pub(crate) is_long_break: bool,
}

impl Inner {
    /// Construct a timer state using config-derived durations. Test-only
    /// helper; production code uses [`Inner::from_config`] so Pomodoro pacing
    /// is wired in too.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn new(
        work_minutes: u32,
        rest_seconds: u32,
        pre_alert_seconds: u32,
        alert_timeout_seconds: u32,
    ) -> Self {
        let pomodoro = PomodoroConfig::default();
        Self {
            state: TimerState::Working,
            state_entered_at: Instant::now(),
            work_duration: Duration::from_secs(u64::from(work_minutes) * 60),
            rest_duration: Duration::from_secs(u64::from(rest_seconds)),
            pre_alert_duration: Duration::from_secs(u64::from(pre_alert_seconds)),
            alert_timeout: Duration::from_secs(u64::from(alert_timeout_seconds)),
            paused_from: None,
            paused_remaining: None,
            mode: TimerMode::TwentyTwentyTwenty,
            cycle_index: 1,
            cycles_per_long: pomodoro.cycles_per_long,
            short_break_duration: Duration::from_secs(u64::from(pomodoro.short_break_minutes) * 60),
            long_break_duration: Duration::from_secs(u64::from(pomodoro.long_break_minutes) * 60),
            is_long_break: false,
        }
    }

    /// Construct from full app config. Selects the working duration based on
    /// `config.timer.mode`.
    #[must_use]
    pub(crate) fn from_config(config: &Config) -> Self {
        let work_secs = match config.timer.mode {
            TimerMode::TwentyTwentyTwenty => u64::from(config.timer.work_minutes) * 60,
            TimerMode::Pomodoro => u64::from(config.pomodoro.focus_minutes) * 60,
        };
        let rest_secs = match config.timer.mode {
            TimerMode::TwentyTwentyTwenty => u64::from(config.timer.rest_seconds),
            TimerMode::Pomodoro => u64::from(config.pomodoro.short_break_minutes) * 60,
        };
        Self {
            state: TimerState::Working,
            state_entered_at: Instant::now(),
            work_duration: Duration::from_secs(work_secs),
            rest_duration: Duration::from_secs(rest_secs),
            pre_alert_duration: Duration::from_secs(u64::from(config.timer.pre_alert_seconds)),
            alert_timeout: Duration::from_secs(u64::from(config.timer.alert_timeout_seconds)),
            paused_from: None,
            paused_remaining: None,
            mode: config.timer.mode,
            cycle_index: 1,
            cycles_per_long: config.pomodoro.cycles_per_long.max(1),
            short_break_duration: Duration::from_secs(
                u64::from(config.pomodoro.short_break_minutes) * 60,
            ),
            long_break_duration: Duration::from_secs(
                u64::from(config.pomodoro.long_break_minutes) * 60,
            ),
            is_long_break: false,
        }
    }

    /// Whether the upcoming Resting should be a long break, based on the
    /// current `cycle_index` having just finished the `cycles_per_long`-th
    /// focus session. Only meaningful in Pomodoro mode.
    #[must_use]
    pub(crate) fn next_rest_is_long(&self) -> bool {
        self.cycles_per_long > 0 && self.cycle_index.is_multiple_of(self.cycles_per_long)
    }

    pub(crate) fn apply_transition_at(&mut self, transition: Transition, now: Instant) {
        if transition.to == TimerState::Paused {
            // Transitioning TO Paused: save origin state and remaining time
            self.paused_from = Some(transition.from);
            self.paused_remaining = self.remaining(now);
            self.state = transition.to;
            self.state_entered_at = now;
            return;
        }

        if transition.from == TimerState::Paused {
            // Transitioning FROM Paused: restore saved state with remaining time
            // Invariant: validate_timer_config rejects configs where
            // work_duration <= pre_alert_duration, so this subtraction is
            // always strictly positive.
            let target_duration = match transition.to {
                TimerState::Working => self
                    .work_duration
                    .checked_sub(self.pre_alert_duration)
                    .expect("invariant: work_duration > pre_alert_duration"),
                TimerState::PreAlert => self.pre_alert_duration,
                TimerState::Alerting => self.alert_timeout,
                TimerState::Resting => self.rest_duration,
                TimerState::Paused => unreachable!(),
            };

            if let Some(remaining) = self.paused_remaining {
                let elapsed = target_duration.saturating_sub(remaining);
                self.state_entered_at = now.checked_sub(elapsed).unwrap_or(now);
            } else {
                self.state_entered_at = now;
            }

            self.state = transition.to;
            self.paused_from = None;
            self.paused_remaining = None;
            return;
        }

        // Normal transition: clear pause state
        self.paused_from = None;
        self.paused_remaining = None;

        // Pomodoro: pick the right break duration when entering Resting and
        // advance the cycle counter when leaving it.
        if self.mode == TimerMode::Pomodoro {
            if transition.to == TimerState::Resting && transition.from != TimerState::Resting {
                self.is_long_break = self.next_rest_is_long();
                self.rest_duration = if self.is_long_break {
                    self.long_break_duration
                } else {
                    self.short_break_duration
                };
            } else if transition.from == TimerState::Resting && transition.to == TimerState::Working
            {
                if self.is_long_break {
                    self.cycle_index = 1;
                } else {
                    self.cycle_index = self.cycle_index.saturating_add(1);
                    if self.cycles_per_long > 0 && self.cycle_index > self.cycles_per_long {
                        // Defensive: keep within [1, cycles_per_long]. Should
                        // not happen if `is_long_break` was set correctly, but
                        // guards against config changes mid-cycle.
                        self.cycle_index = 1;
                    }
                }
                self.is_long_break = false;
            } else if transition.to == TimerState::Working && transition.from != TimerState::Resting
            {
                // Entering Working through any non-Resting path (e.g. Alerting
                // -> Working skip, Working -> Working reset) clears the
                // pending long-break flag without touching cycle_index.
                self.is_long_break = false;
            }
        }

        self.state = transition.to;
        self.state_entered_at = now;
    }

    /// Return elapsed time in the current state.
    #[must_use]
    pub(crate) fn elapsed(&self, now: Instant) -> Duration {
        now.duration_since(self.state_entered_at)
    }

    /// Return the remaining time for timed states.
    ///
    /// Invariant: `validate_timer_config` rejects configs where
    /// `work_duration` <= `pre_alert_duration`, so the Working subtraction is
    /// always strictly positive and never returns None.
    #[must_use]
    pub(crate) fn remaining(&self, now: Instant) -> Option<Duration> {
        let target = match self.state {
            TimerState::Working => self
                .work_duration
                .checked_sub(self.pre_alert_duration)
                .expect("invariant: work_duration > pre_alert_duration"),
            TimerState::PreAlert => self.pre_alert_duration,
            TimerState::Alerting => self.alert_timeout,
            TimerState::Resting => self.rest_duration,
            TimerState::Paused => return None,
        };

        target.checked_sub(self.elapsed(now))
    }
}
