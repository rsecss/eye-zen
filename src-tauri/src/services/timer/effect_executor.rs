use tracing::info;

use super::effect::{Effect, SoundType, TrayUpdate};
use super::state::TimerState;
use crate::models::statistics::{CycleEventDraft, RestSessionDraft};
use crate::models::types::StatePayload;

/// Sink that executes side effects emitted by the pure timer core.
///
/// Production wires this to `ServiceContext`, which dispatches to the real
/// Tauri-backed services (window / sound / tray / stat). Tests use a recording
/// fake (`RecordingSink` in `#[cfg(test)]`) to assert effect-to-call mapping
/// without needing an `AppHandle`.
///
/// One method per `Effect` variant on purpose: a single dispatch `fn(&Effect)`
/// would push the match into every fake, which CLAUDE.md forbids ("flag-driven
/// branching"). Granular methods also make fakes record structured payloads
/// rather than stringified `Debug`.
pub(crate) trait EffectSink: Send + Sync {
    fn emit_state_changed(&self, payload: &StatePayload);
    fn show_tip_windows(&self);
    fn hide_tip_windows(&self);
    fn play_sound(&self, sound: SoundType);
    fn update_tray_tooltip(&self, tooltip: super::effect::TrayTooltip);
    fn update_tray_state_icon(&self, state: TimerState);
    fn reset_work_timer(&self, duration: std::time::Duration);
    fn record_rest_session(&self, session: &RestSessionDraft);
    fn record_cycle_event(&self, event: &CycleEventDraft);
}

/// Dispatch a single timer `Effect` to the supplied sink.
///
/// `sink: Option<&dyn EffectSink>` lets the timer service run without a
/// runtime context (early boot, tests using `TimerService` directly) — in that
/// case the effect is logged but not executed, matching the pre-refactor
/// `STUB effect:` behavior.
pub(crate) fn execute_effect(sink: Option<&dyn EffectSink>, effect: &Effect) {
    let Some(sink) = sink else {
        info!("STUB effect: {effect:?}");
        return;
    };

    match effect {
        Effect::EmitStateChanged(payload) => sink.emit_state_changed(payload),
        Effect::ShowTipWindows => sink.show_tip_windows(),
        Effect::HideTipWindows => sink.hide_tip_windows(),
        Effect::PlaySound(sound) => sink.play_sound(*sound),
        Effect::UpdateTray(update) => match update {
            TrayUpdate::Tooltip(tooltip) => sink.update_tray_tooltip(*tooltip),
            TrayUpdate::StateIcon(state) => sink.update_tray_state_icon(*state),
        },
        Effect::ResetWorkTimer(duration) => sink.reset_work_timer(*duration),
        Effect::RecordRestSession(session) => sink.record_rest_session(session),
        Effect::RecordCycleEvent(event) => sink.record_cycle_event(event),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use chrono::Utc;

    use super::*;
    use crate::models::config::TimerMode;
    use crate::models::statistics::CycleOutcome;
    use crate::services::timer::effect::{PomodoroProgress, TrayTooltip};

    /// Recorded effect-call for `RecordingSink` assertions. One variant per
    /// `EffectSink` method so test failures point at the right boundary.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Call {
        EmitStateChanged(String, u32),
        ShowTipWindows,
        HideTipWindows,
        PlaySound(SoundType),
        UpdateTrayTooltip(TrayTooltip),
        UpdateTrayStateIcon(TimerState),
        ResetWorkTimer(Duration),
        RecordRestSession(u32),
        RecordCycleEvent(CycleOutcome),
    }

    #[derive(Default)]
    struct RecordingSink {
        calls: Mutex<Vec<Call>>,
    }

    impl RecordingSink {
        fn calls(&self) -> Vec<Call> {
            self.calls.lock().expect("calls mutex").clone()
        }

        fn push(&self, call: Call) {
            self.calls.lock().expect("calls mutex").push(call);
        }
    }

    impl EffectSink for RecordingSink {
        fn emit_state_changed(&self, payload: &StatePayload) {
            self.push(Call::EmitStateChanged(
                payload.state.clone(),
                payload.remaining_secs,
            ));
        }
        fn show_tip_windows(&self) {
            self.push(Call::ShowTipWindows);
        }
        fn hide_tip_windows(&self) {
            self.push(Call::HideTipWindows);
        }
        fn play_sound(&self, sound: SoundType) {
            self.push(Call::PlaySound(sound));
        }
        fn update_tray_tooltip(&self, tooltip: TrayTooltip) {
            self.push(Call::UpdateTrayTooltip(tooltip));
        }
        fn update_tray_state_icon(&self, state: TimerState) {
            self.push(Call::UpdateTrayStateIcon(state));
        }
        fn reset_work_timer(&self, duration: Duration) {
            self.push(Call::ResetWorkTimer(duration));
        }
        fn record_rest_session(&self, session: &RestSessionDraft) {
            self.push(Call::RecordRestSession(session.duration_secs));
        }
        fn record_cycle_event(&self, event: &CycleEventDraft) {
            self.push(Call::RecordCycleEvent(event.outcome));
        }
    }

    fn sample_state_payload() -> StatePayload {
        StatePayload {
            state: "working".to_string(),
            remaining_secs: 1200,
            work_minutes: 20,
            rest_seconds: 20,
            mode: TimerMode::TwentyTwentyTwenty,
            pomodoro: None,
        }
    }

    #[test]
    fn no_sink_logs_stub_and_returns() {
        // Should not panic when sink is None.
        execute_effect(None, &Effect::ShowTipWindows);
        execute_effect(None, &Effect::HideTipWindows);
    }

    #[test]
    fn emit_state_changed_dispatches() {
        let sink = RecordingSink::default();
        execute_effect(
            Some(&sink),
            &Effect::EmitStateChanged(sample_state_payload()),
        );
        assert_eq!(
            sink.calls(),
            vec![Call::EmitStateChanged("working".to_string(), 1200)]
        );
    }

    #[test]
    fn show_and_hide_tip_windows_dispatch() {
        let sink = RecordingSink::default();
        execute_effect(Some(&sink), &Effect::ShowTipWindows);
        execute_effect(Some(&sink), &Effect::HideTipWindows);
        assert_eq!(
            sink.calls(),
            vec![Call::ShowTipWindows, Call::HideTipWindows]
        );
    }

    #[test]
    fn play_sound_routes_variants_distinctly() {
        let sink = RecordingSink::default();
        execute_effect(Some(&sink), &Effect::PlaySound(SoundType::PreAlert));
        execute_effect(Some(&sink), &Effect::PlaySound(SoundType::RestComplete));
        assert_eq!(
            sink.calls(),
            vec![
                Call::PlaySound(SoundType::PreAlert),
                Call::PlaySound(SoundType::RestComplete),
            ]
        );
    }

    #[test]
    fn update_tray_routes_tooltip_and_state_icon_separately() {
        let sink = RecordingSink::default();
        let tooltip = TrayTooltip {
            state: TimerState::Working,
            remaining_secs: Some(60),
            pomodoro_progress: Some(PomodoroProgress {
                current: 2,
                total: 4,
            }),
        };
        execute_effect(
            Some(&sink),
            &Effect::UpdateTray(TrayUpdate::Tooltip(tooltip)),
        );
        execute_effect(
            Some(&sink),
            &Effect::UpdateTray(TrayUpdate::StateIcon(TimerState::Paused)),
        );
        assert_eq!(
            sink.calls(),
            vec![
                Call::UpdateTrayTooltip(tooltip),
                Call::UpdateTrayStateIcon(TimerState::Paused),
            ]
        );
    }

    #[test]
    fn reset_work_timer_passes_duration() {
        let sink = RecordingSink::default();
        execute_effect(
            Some(&sink),
            &Effect::ResetWorkTimer(Duration::from_mins(20)),
        );
        assert_eq!(
            sink.calls(),
            vec![Call::ResetWorkTimer(Duration::from_mins(20))]
        );
    }

    #[test]
    fn record_rest_session_borrows_draft() {
        let sink = RecordingSink::default();
        let session = RestSessionDraft {
            started_at_utc: Utc::now(),
            ended_at_utc: Utc::now(),
            duration_secs: 20,
        };
        execute_effect(Some(&sink), &Effect::RecordRestSession(session));
        assert_eq!(sink.calls(), vec![Call::RecordRestSession(20)]);
    }

    #[test]
    fn record_cycle_event_borrows_draft() {
        let sink = RecordingSink::default();
        let event = CycleEventDraft {
            occurred_at_utc: Utc::now(),
            outcome: CycleOutcome::Suppressed,
            reason: None,
            process_hint: None,
            duration_secs: None,
            mode: TimerMode::TwentyTwentyTwenty,
            is_long_break: false,
        };
        execute_effect(Some(&sink), &Effect::RecordCycleEvent(event));
        assert_eq!(
            sink.calls(),
            vec![Call::RecordCycleEvent(CycleOutcome::Suppressed)]
        );
    }
}
