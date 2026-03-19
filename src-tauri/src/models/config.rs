#![allow(clippy::module_name_repetitions)]

use serde::{Deserialize, Serialize};

/// Root application configuration persisted as TOML.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(default)]
pub struct Config {
    pub timer: TimerConfig,
    pub behavior: BehaviorConfig,
    pub display: DisplayConfig,
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
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            work_minutes: default_work_minutes(),
            rest_seconds: default_rest_seconds(),
            pre_alert_seconds: default_pre_alert_seconds(),
            alert_timeout_seconds: default_alert_timeout_seconds(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct BehaviorConfig {
    #[serde(default = "default_true")]
    pub sound_enabled: bool,
    #[serde(default = "default_true")]
    pub fullscreen_skip: bool,
    #[serde(default)]
    pub auto_start: bool,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            sound_enabled: true,
            fullscreen_skip: true,
            auto_start: false,
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

fn default_language() -> String {
    "zh-CN".to_string()
}

fn default_theme() -> String {
    "light".to_string()
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
        assert!(config.behavior.sound_enabled);
        assert!(config.behavior.fullscreen_skip);
        assert!(!config.behavior.auto_start);
        assert_eq!(config.display.language, "zh-CN");
        assert_eq!(config.display.theme, "light");
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
        assert!(config.behavior.sound_enabled);
        assert_eq!(config.display.theme, "light");
    }

    #[test]
    fn empty_toml_uses_all_defaults() {
        let config: Config = toml::from_str("").expect("empty config should use defaults");
        assert_eq!(config, Config::default());
    }
}
