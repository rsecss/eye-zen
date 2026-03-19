#![allow(clippy::module_name_repetitions)]

use std::time::Duration;

use crate::models::types::StatePayload;

/// Side effects collected by the pure timer core.
#[derive(Debug, Clone)]
pub(crate) enum Effect {
    EmitStateChanged(StatePayload),
    ShowTipWindows,
    HideTipWindows,
    PlaySound(SoundType),
    UpdateTray(TrayUpdate),
    ResetWorkTimer(Duration),
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
    Tooltip(String),
    StateIcon(String),
}
