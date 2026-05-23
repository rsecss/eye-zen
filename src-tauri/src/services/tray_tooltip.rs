//! Pure tooltip rendering for the system tray.
//!
//! Extracted from `tray.rs` so the `i18n` + state → tooltip string mapping can
//! be unit-tested without compiling the Tauri tray-icon code path (which is
//! `#[cfg(not(test))]`). The production `TrayService::render_tooltip` simply
//! delegates here.

use crate::services::i18n::I18nService;
use crate::services::timer::{TimerState, TrayTooltip};

/// Render the system-tray tooltip text for the given timer snapshot.
///
/// Format:
/// - `<App> - <State label> mm:ss [(Pomo current/total)]` when `remaining_secs`
///   is set
/// - `<App> - <State label> [(Pomo current/total)]` when no remaining is
///   relevant (e.g. paused before the timer was ever observed)
///
/// Pomodoro suffix is added only when `tooltip.pomodoro_progress` is `Some`,
/// reflecting the runtime `TimerMode::Pomodoro` cycle index.
#[must_use]
pub(crate) fn render_tooltip(i18n: &I18nService, tooltip: TrayTooltip) -> String {
    let app_name = i18n.get("tray.tooltip.app_name");
    let label = state_label(i18n, tooltip.state);

    let pomodoro_suffix = tooltip.pomodoro_progress.map_or(String::new(), |progress| {
        format!(" (Pomo {}/{})", progress.current, progress.total)
    });

    match tooltip.remaining_secs {
        Some(remaining_secs) => format!(
            "{app_name} - {label} {}{pomodoro_suffix}",
            format_remaining(remaining_secs)
        ),
        None => format!("{app_name} - {label}{pomodoro_suffix}"),
    }
}

/// Localised label for the given timer state.
#[must_use]
pub(crate) fn state_label(i18n: &I18nService, state: TimerState) -> &'static str {
    let key = match state {
        TimerState::Working => "tray.tooltip.working",
        TimerState::PreAlert => "tray.tooltip.pre_alert",
        TimerState::Alerting => "tray.tooltip.alerting",
        TimerState::Resting => "tray.tooltip.resting",
        TimerState::Paused => "tray.tooltip.paused",
    };
    i18n.get(key)
}

/// Format a `u32` seconds count as `MM:SS`. Minutes are unbounded (60-minute
/// rests would render as `60:00`); seconds wrap at 60.
#[must_use]
pub(crate) fn format_remaining(total_secs: u32) -> String {
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    format!("{minutes:02}:{seconds:02}")
}

/// Localised text for the pause/resume menu item, based on whether the
/// timer is currently paused.
#[must_use]
pub(crate) fn pause_menu_text(i18n: &I18nService, is_paused: bool) -> &'static str {
    if is_paused {
        i18n.get("tray.resume")
    } else {
        i18n.get("tray.pause")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::timer::effect::PomodoroProgress;

    #[test]
    fn format_remaining_zero_padding() {
        assert_eq!(format_remaining(0), "00:00");
        assert_eq!(format_remaining(5), "00:05");
        assert_eq!(format_remaining(65), "01:05");
        assert_eq!(format_remaining(20 * 60), "20:00");
    }

    #[test]
    fn format_remaining_handles_long_durations() {
        // 90 minutes -> 90:00 (minutes are not clamped, by design).
        assert_eq!(format_remaining(90 * 60), "90:00");
        // 1 second short of an hour.
        assert_eq!(format_remaining(60 * 60 - 1), "59:59");
    }

    #[test]
    fn state_label_uses_i18n_key_per_state() {
        let i18n = I18nService::new("en");
        // We can't assert exact text without coupling to i18n bundle, but we
        // can assert each state produces a distinct non-empty string.
        let working = state_label(&i18n, TimerState::Working);
        let pre_alert = state_label(&i18n, TimerState::PreAlert);
        let alerting = state_label(&i18n, TimerState::Alerting);
        let resting = state_label(&i18n, TimerState::Resting);
        let paused = state_label(&i18n, TimerState::Paused);

        for label in [&working, &pre_alert, &alerting, &resting, &paused] {
            assert!(
                !label.is_empty(),
                "state_label must not return empty: {label:?}"
            );
        }
    }

    #[test]
    fn pause_menu_text_flips_with_paused_flag() {
        let i18n = I18nService::new("en");
        let pause_text = pause_menu_text(&i18n, false);
        let resume_text = pause_menu_text(&i18n, true);
        assert_ne!(
            pause_text, resume_text,
            "pause and resume strings must differ"
        );
    }

    #[test]
    fn render_tooltip_includes_remaining_and_label_separator() {
        let i18n = I18nService::new("en");
        let text = render_tooltip(
            &i18n,
            TrayTooltip {
                state: TimerState::Working,
                remaining_secs: Some(20 * 60),
                pomodoro_progress: None,
            },
        );
        assert!(
            text.contains(" - "),
            "tooltip must use app/state separator: {text}"
        );
        assert!(text.contains("20:00"), "tooltip must contain MM:SS: {text}");
        assert!(
            !text.contains("Pomo"),
            "no pomodoro suffix expected: {text}"
        );
    }

    #[test]
    fn render_tooltip_omits_time_when_remaining_is_none() {
        let i18n = I18nService::new("en");
        let text = render_tooltip(
            &i18n,
            TrayTooltip {
                state: TimerState::Paused,
                remaining_secs: None,
                pomodoro_progress: None,
            },
        );
        // No digits-colon-digits run.
        let has_clock = text.chars().any(|c| c == ':')
            && text.chars().filter(char::is_ascii_digit).count() >= 2;
        assert!(
            !has_clock,
            "no MM:SS clock expected when remaining is None: {text}"
        );
    }

    #[test]
    fn render_tooltip_appends_pomodoro_progress_when_present() {
        let i18n = I18nService::new("en");
        let text = render_tooltip(
            &i18n,
            TrayTooltip {
                state: TimerState::Working,
                remaining_secs: Some(25 * 60),
                pomodoro_progress: Some(PomodoroProgress {
                    current: 2,
                    total: 4,
                }),
            },
        );
        assert!(
            text.contains("(Pomo 2/4)"),
            "expected pomodoro suffix: {text}"
        );
        assert!(
            text.contains("25:00"),
            "tooltip must still contain remaining: {text}"
        );
    }

    #[test]
    fn render_tooltip_appends_pomodoro_suffix_even_when_no_remaining() {
        let i18n = I18nService::new("en");
        let text = render_tooltip(
            &i18n,
            TrayTooltip {
                state: TimerState::Paused,
                remaining_secs: None,
                pomodoro_progress: Some(PomodoroProgress {
                    current: 1,
                    total: 4,
                }),
            },
        );
        assert!(
            text.contains("(Pomo 1/4)"),
            "pomodoro suffix should append even when no remaining: {text}"
        );
    }
}
