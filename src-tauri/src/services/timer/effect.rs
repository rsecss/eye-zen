#![allow(clippy::module_name_repetitions)]

use std::time::Duration;

use crate::models::statistics::RestSessionDraft;
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
}
