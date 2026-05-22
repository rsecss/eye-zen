#![allow(clippy::module_name_repetitions)]

use std::time::Duration;

use crate::models::statistics::{CycleEventDraft, RestSessionDraft};
use crate::models::types::StatePayload;

use super::state::TimerState;

/// Side effects collected by the pure timer core.
#[derive(Debug, Clone)]
pub(crate) enum Effect {
    EmitStateChanged(StatePayload),
    ShowTipWindows,
    HideTipWindows,
    PlaySound(SoundType),
    UpdateTray(TrayUpdate),
    ResetWorkTimer(Duration),
    RecordRestSession(RestSessionDraft),
    /// Persist a rest-cycle outcome (taken / skipped / suppressed) with its
    /// reason snapshot for v0.6 Health Analysis.
    RecordCycleEvent(CycleEventDraft),
}

/// Sound variants consumed by the sound service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SoundType {
    PreAlert,
    RestComplete,
}

/// Tray updates consumed by the tray service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrayUpdate {
    Tooltip(TrayTooltip),
    StateIcon(TimerState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TrayTooltip {
    pub(crate) state: TimerState,
    pub(crate) remaining_secs: Option<u32>,
    /// Pomodoro progress shown as `(Pomo current/total)` suffix when present.
    pub(crate) pomodoro_progress: Option<PomodoroProgress>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PomodoroProgress {
    pub(crate) current: u32,
    pub(crate) total: u32,
}
