#![allow(clippy::module_name_repetitions)]

use serde::{Deserialize, Serialize};

use super::hotkeys::HotkeysConfig;

/// Root application configuration persisted as TOML.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(default)]
pub struct Config {
    pub timer: TimerConfig,
    pub behavior: BehaviorConfig,
    pub display: DisplayConfig,
    pub schedule: ScheduleConfig,
    pub hotkeys: HotkeysConfig,
    pub pomodoro: PomodoroConfig,
}

/// Which rhythm the timer uses.
///
/// `TwentyTwentyTwenty` keeps the existing eye-care behavior (`work_minutes`
/// ⇒ `rest_seconds`). `Pomodoro` follows classic 25/5/15 pacing using
/// `PomodoroConfig`; cycle counter is held in the timer runtime, not in TOML.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "snake_case")]
pub enum TimerMode {
    #[default]
    TwentyTwentyTwenty,
    Pomodoro,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct TimerConfig {
    #[serde(default = "default_work_minutes")]
    pub work_minutes: u32,
    #[serde(default = "default_rest_seconds")]
    pub rest_seconds: u32,
    #[serde(default = "default_pre_alert_seconds")]
    pub pre_alert_seconds: u32,
    #[serde(default = "default_alert_timeout_seconds")]
    pub alert_timeout_seconds: u32,
    #[serde(default)]
    pub mode: TimerMode,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            work_minutes: default_work_minutes(),
            rest_seconds: default_rest_seconds(),
            pre_alert_seconds: default_pre_alert_seconds(),
            alert_timeout_seconds: default_alert_timeout_seconds(),
            mode: TimerMode::default(),
        }
    }
}

/// Pomodoro pacing knobs. Only consulted when `TimerConfig::mode ==
/// TimerMode::Pomodoro`; the cycle counter itself is runtime-only and reset on
/// every app start (matches the "session = single run" Pomodoro tradition).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct PomodoroConfig {
    #[serde(default = "default_focus_minutes")]
    pub focus_minutes: u32,
    #[serde(default = "default_short_break_minutes")]
    pub short_break_minutes: u32,
    #[serde(default = "default_long_break_minutes")]
    pub long_break_minutes: u32,
    #[serde(default = "default_cycles_per_long")]
    pub cycles_per_long: u32,
}

impl Default for PomodoroConfig {
    fn default() -> Self {
        Self {
            focus_minutes: default_focus_minutes(),
            short_break_minutes: default_short_break_minutes(),
            long_break_minutes: default_long_break_minutes(),
            cycles_per_long: default_cycles_per_long(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
#[allow(clippy::struct_excessive_bools)]
pub struct BehaviorConfig {
    #[serde(default = "default_true")]
    pub sound_enabled: bool,
    #[serde(default = "default_true")]
    pub fullscreen_skip: bool,
    #[serde(default = "default_true")]
    pub afk_skip_enabled: bool,
    #[serde(default = "default_afk_threshold_minutes")]
    pub afk_threshold_minutes: u32,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub process_whitelist_enabled: bool,
    #[serde(default)]
    pub process_whitelist: Vec<String>,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            sound_enabled: true,
            fullscreen_skip: true,
            afk_skip_enabled: true,
            afk_threshold_minutes: default_afk_threshold_minutes(),
            auto_start: false,
            process_whitelist_enabled: false,
            process_whitelist: Vec::new(),
        }
    }
}

/// Maximum entries allowed in `BehaviorConfig::process_whitelist`. Entries
/// beyond this cap are silently dropped at load time and rejected by the UI.
pub const PROCESS_WHITELIST_MAX_LEN: usize = 32;

/// Reserved process names that may never be added to the whitelist.
/// Comparison is case-insensitive against the trimmed candidate.
pub const PROCESS_WHITELIST_RESERVED: &[&str] = &["eyezen", "eyezen.exe"];

/// Normalise a raw whitelist vector into canonical storage form:
///   1. trim each entry
///   2. lowercase each entry
///   3. drop empty entries
///   4. drop reserved names (eyezen / eyezen.exe)
///   5. de-duplicate preserving first occurrence
///   6. truncate to `PROCESS_WHITELIST_MAX_LEN`
#[must_use]
pub fn sanitize_process_whitelist(raw: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(raw.len().min(PROCESS_WHITELIST_MAX_LEN));
    for entry in raw {
        let trimmed = entry.trim().to_lowercase();
        if trimmed.is_empty() {
            continue;
        }
        if PROCESS_WHITELIST_RESERVED.iter().any(|r| *r == trimmed) {
            continue;
        }
        if out.iter().any(|existing| existing == &trimmed) {
            continue;
        }
        out.push(trimmed);
        if out.len() >= PROCESS_WHITELIST_MAX_LEN {
            break;
        }
    }
    out
}

/// Weekly schedule controlling when rest reminders are allowed to surface.
///
/// `active_days` is indexed by `chrono::Weekday::num_days_from_monday`:
/// `0 = Mon`, `1 = Tue`, ..., `6 = Sun`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct ScheduleConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_active_days")]
    pub active_days: [bool; 7],
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            active_days: default_active_days(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct DisplayConfig {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            language: default_language(),
            theme: default_theme(),
        }
    }
}

const fn default_work_minutes() -> u32 {
    20
}

const fn default_rest_seconds() -> u32 {
    20
}

const fn default_pre_alert_seconds() -> u32 {
    15
}

const fn default_alert_timeout_seconds() -> u32 {
    60
}

const fn default_true() -> bool {
    true
}

const fn default_afk_threshold_minutes() -> u32 {
    5
}

const fn default_focus_minutes() -> u32 {
    25
}

const fn default_short_break_minutes() -> u32 {
    5
}

const fn default_long_break_minutes() -> u32 {
    15
}

const fn default_cycles_per_long() -> u32 {
    4
}

fn default_language() -> String {
    "zh-CN".to_string()
}

fn default_theme() -> String {
    "light".to_string()
}

const fn default_active_days() -> [bool; 7] {
    [true, true, true, true, true, false, false]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let config = Config::default();
        assert_eq!(config.timer.work_minutes, 20);
        assert_eq!(config.timer.rest_seconds, 20);
        assert_eq!(config.timer.pre_alert_seconds, 15);
        assert_eq!(config.timer.alert_timeout_seconds, 60);
        assert_eq!(config.timer.mode, TimerMode::TwentyTwentyTwenty);
        assert!(config.behavior.sound_enabled);
        assert!(config.behavior.fullscreen_skip);
        assert!(config.behavior.afk_skip_enabled);
        assert_eq!(config.behavior.afk_threshold_minutes, 5);
        assert!(!config.behavior.auto_start);
        assert!(!config.behavior.process_whitelist_enabled);
        assert!(config.behavior.process_whitelist.is_empty());
        assert_eq!(config.display.language, "zh-CN");
        assert_eq!(config.display.theme, "light");
        assert!(!config.schedule.enabled);
        assert_eq!(
            config.schedule.active_days,
            [true, true, true, true, true, false, false]
        );
        assert_eq!(config.hotkeys, HotkeysConfig::default());
        assert_eq!(config.pomodoro.focus_minutes, 25);
        assert_eq!(config.pomodoro.short_break_minutes, 5);
        assert_eq!(config.pomodoro.long_break_minutes, 15);
        assert_eq!(config.pomodoro.cycles_per_long, 4);
    }

    #[test]
    fn toml_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).expect("config should serialize");
        let parsed: Config = toml::from_str(&toml_str).expect("config should deserialize");
        assert_eq!(config, parsed);
    }

    #[test]
    fn partial_toml_uses_defaults() {
        let toml_str = r"
[timer]
work_minutes = 25
";
        let config: Config = toml::from_str(toml_str).expect("partial config should parse");
        assert_eq!(config.timer.work_minutes, 25);
        assert_eq!(config.timer.rest_seconds, 20);
        assert_eq!(config.timer.mode, TimerMode::TwentyTwentyTwenty);
        assert!(config.behavior.sound_enabled);
        assert!(config.behavior.afk_skip_enabled);
        assert_eq!(config.behavior.afk_threshold_minutes, 5);
        assert_eq!(config.display.theme, "light");
        assert_eq!(config.hotkeys, HotkeysConfig::default());
        assert_eq!(config.pomodoro, PomodoroConfig::default());
    }

    #[test]
    fn pomodoro_mode_toml_parses() {
        let toml_str = r#"
[timer]
mode = "pomodoro"

[pomodoro]
focus_minutes = 30
short_break_minutes = 7
long_break_minutes = 20
cycles_per_long = 3
"#;
        let config: Config = toml::from_str(toml_str).expect("pomodoro config should parse");
        assert_eq!(config.timer.mode, TimerMode::Pomodoro);
        assert_eq!(config.pomodoro.focus_minutes, 30);
        assert_eq!(config.pomodoro.short_break_minutes, 7);
        assert_eq!(config.pomodoro.long_break_minutes, 20);
        assert_eq!(config.pomodoro.cycles_per_long, 3);
    }

    #[test]
    fn timer_mode_serializes_as_snake_case() {
        let mode = TimerMode::Pomodoro;
        let serialized = serde_json::to_string(&mode).expect("mode should serialize");
        assert_eq!(serialized, "\"pomodoro\"");

        let mode = TimerMode::TwentyTwentyTwenty;
        let serialized = serde_json::to_string(&mode).expect("mode should serialize");
        assert_eq!(serialized, "\"twenty_twenty_twenty\"");
    }

    #[test]
    fn empty_toml_uses_all_defaults() {
        let config: Config = toml::from_str("").expect("empty config should use defaults");
        assert_eq!(config, Config::default());
    }

    #[test]
    fn sanitize_drops_empty_and_reserved() {
        let raw = vec![
            "  ".to_string(),
            "eyezen".to_string(),
            "Eyezen.EXE".to_string(),
            "code.exe".to_string(),
        ];
        let out = sanitize_process_whitelist(raw);
        assert_eq!(out, vec!["code.exe".to_string()]);
    }

    #[test]
    fn sanitize_dedupes_case_insensitively() {
        let raw = vec![
            "Code".to_string(),
            " code ".to_string(),
            "CODE".to_string(),
            "chrome".to_string(),
        ];
        let out = sanitize_process_whitelist(raw);
        assert_eq!(out, vec!["code".to_string(), "chrome".to_string()]);
    }

    #[test]
    fn sanitize_truncates_to_max_len() {
        let raw: Vec<String> = (0..PROCESS_WHITELIST_MAX_LEN + 5)
            .map(|i| format!("proc{i}"))
            .collect();
        let out = sanitize_process_whitelist(raw);
        assert_eq!(out.len(), PROCESS_WHITELIST_MAX_LEN);
    }
}
