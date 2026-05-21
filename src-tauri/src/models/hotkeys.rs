use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "snake_case")]
pub enum HotkeyAction {
    StartRest,
    SkipRest,
    TogglePause,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct HotkeysConfig {
    #[serde(default = "default_start_rest_shortcut")]
    pub start_rest: String,
    #[serde(default = "default_skip_rest_shortcut")]
    pub skip_rest: String,
    #[serde(default = "default_toggle_pause_shortcut")]
    pub toggle_pause: String,
}

impl HotkeysConfig {
    #[must_use]
    pub fn entries(&self) -> [(HotkeyAction, &str); 3] {
        [
            (HotkeyAction::StartRest, self.start_rest.as_str()),
            (HotkeyAction::SkipRest, self.skip_rest.as_str()),
            (HotkeyAction::TogglePause, self.toggle_pause.as_str()),
        ]
    }
}

impl Default for HotkeysConfig {
    fn default() -> Self {
        Self {
            start_rest: default_start_rest_shortcut(),
            skip_rest: default_skip_rest_shortcut(),
            toggle_pause: default_toggle_pause_shortcut(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "snake_case")]
pub enum HotkeyBindingState {
    Registered,
    Conflict,
    PermissionMissing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
#[serde(rename_all = "snake_case")]
pub enum MacosAccessibilityStatus {
    NotRequired,
    Granted,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct HotkeyBindingStatus {
    pub action: HotkeyAction,
    pub shortcut: String,
    pub state: HotkeyBindingState,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct HotkeyStatus {
    pub bindings: Vec<HotkeyBindingStatus>,
    pub macos_accessibility: MacosAccessibilityStatus,
    pub last_error: Option<String>,
}

fn default_start_rest_shortcut() -> String {
    "CommandOrControl+Alt+B".to_string()
}

fn default_skip_rest_shortcut() -> String {
    "CommandOrControl+Alt+S".to_string()
}

fn default_toggle_pause_shortcut() -> String {
    "CommandOrControl+Alt+P".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_enum_serializes_as_snake_case() {
        let json =
            serde_json::to_string(&HotkeyAction::TogglePause).expect("action should serialize");
        assert_eq!(json, "\"toggle_pause\"");

        let decoded: HotkeyAction =
            serde_json::from_str("\"start_rest\"").expect("action should deserialize");
        assert_eq!(decoded, HotkeyAction::StartRest);
    }

    #[test]
    fn default_shortcuts_are_present() {
        let config = HotkeysConfig::default();
        assert_eq!(config.start_rest, "CommandOrControl+Alt+B");
        assert_eq!(config.skip_rest, "CommandOrControl+Alt+S");
        assert_eq!(config.toggle_pause, "CommandOrControl+Alt+P");
    }
}
