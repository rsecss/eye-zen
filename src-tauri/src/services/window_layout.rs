//! Pure helpers for `WindowService`'s multi-monitor tip-window layout.
//!
//! Extracted from `window.rs` so the primary-monitor selection rule, the
//! window-label scheme, and the tip-URL choice can be unit-tested without
//! depending on Tauri's runtime APIs (which are `#[cfg(not(test))]`).
//!
//! Production `WindowService::show_tip_windows` calls into the live monitor
//! APIs to populate the inputs and then delegates the deterministic part to
//! the functions here.

/// Identity stand-in for a monitor — just its name (or `None` for unnamed).
///
/// Tauri's `Monitor::name()` returns `Option<&String>`. The selector treats
/// two monitors as the "same" when their names match (or both `None`); this
/// mirrors the production primary-monitor check which would otherwise need a
/// full `tauri::Monitor` to test.
pub(crate) type MonitorName<'a> = Option<&'a str>;

/// Decide whether the monitor at `index` is the "primary" tip-window target.
///
/// Rule (matches production behavior):
/// - When a primary monitor is reported by the OS, compare names. The monitor
///   whose name matches `primary` is primary regardless of `index`.
/// - When the OS reports no primary monitor, fall back to "the first one
///   wins" (`index == 0`).
///
/// The primary monitor gets the full `tip.html` page with focus; secondaries
/// get the `tip-minimal.html` overlay.
#[must_use]
pub(crate) fn is_primary_monitor(
    index: usize,
    monitor: MonitorName<'_>,
    primary: Option<MonitorName<'_>>,
) -> bool {
    match primary {
        Some(primary_name) => monitor == primary_name,
        None => index == 0,
    }
}

/// Build the `WebviewWindow` label for the tip window at `index`.
///
/// Convention (preserved from the pre-refactor `window.rs`):
/// - Primary monitor: `tip-window-0` (full tip)
/// - Other monitors: `tip-window-minimal-<index>` (minimal overlay)
///
/// Labels matter for downstream lookups: `WindowService::hide_tip_windows`
/// closes every window whose label starts with `tip-window`.
#[must_use]
pub(crate) fn tip_window_label(index: usize, is_primary: bool) -> String {
    if is_primary {
        "tip-window-0".to_string()
    } else {
        format!("tip-window-minimal-{index}")
    }
}

/// Choose the HTML page for the tip window. Primary monitor shows the full
/// tip UI; secondaries show a minimal overlay so the user is gently nudged
/// without three duplicate panels competing for attention.
#[must_use]
pub(crate) const fn tip_window_url(is_primary: bool) -> &'static str {
    if is_primary {
        "tip.html"
    } else {
        "tip-minimal.html"
    }
}

/// `true` when `label` belongs to a dynamically created tip window (and is
/// therefore safe to close in `hide_tip_windows`). Used to filter
/// `app.webview_windows()` keys.
#[must_use]
pub(crate) fn is_tip_window_label(label: &str) -> bool {
    label.starts_with("tip-window")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_when_name_matches() {
        assert!(is_primary_monitor(1, Some("HDMI-1"), Some(Some("HDMI-1"))));
    }

    #[test]
    fn not_primary_when_name_differs() {
        assert!(!is_primary_monitor(0, Some("HDMI-1"), Some(Some("HDMI-2"))));
    }

    #[test]
    fn primary_falls_back_to_index_when_os_reports_none() {
        // OS returned `None` for primary monitor (Linux corner case).
        assert!(is_primary_monitor(0, Some("eDP-1"), None));
        assert!(!is_primary_monitor(1, Some("HDMI-1"), None));
    }

    #[test]
    fn primary_matches_when_both_names_are_none() {
        // Unusual case: monitor and primary both have no name. Treated as a
        // match because that's what `String == String` would do; covered here
        // so the rule is locked.
        assert!(is_primary_monitor(0, None, Some(None)));
    }

    #[test]
    fn primary_does_not_match_when_one_name_is_none() {
        assert!(!is_primary_monitor(0, Some("HDMI-1"), Some(None)));
        assert!(!is_primary_monitor(0, None, Some(Some("HDMI-1"))));
    }

    #[test]
    fn label_scheme_for_primary_is_fixed() {
        assert_eq!(tip_window_label(0, true), "tip-window-0");
        // Even when the "primary" monitor has a non-zero index (e.g. user's
        // primary is the secondary GPU output), the label is still the
        // canonical `tip-window-0`.
        assert_eq!(tip_window_label(2, true), "tip-window-0");
    }

    #[test]
    fn label_scheme_for_secondary_includes_index() {
        assert_eq!(tip_window_label(1, false), "tip-window-minimal-1");
        assert_eq!(tip_window_label(3, false), "tip-window-minimal-3");
    }

    #[test]
    fn url_picks_full_or_minimal() {
        assert_eq!(tip_window_url(true), "tip.html");
        assert_eq!(tip_window_url(false), "tip-minimal.html");
    }

    #[test]
    fn label_filter_matches_dynamic_tip_windows_only() {
        assert!(is_tip_window_label("tip-window-0"));
        assert!(is_tip_window_label("tip-window-minimal-2"));
        assert!(!is_tip_window_label("main-window"));
        assert!(!is_tip_window_label("tray-panel"));
        assert!(!is_tip_window_label("tip")); // the static `tip.html` is named differently
        assert!(!is_tip_window_label(""));
    }

    /// Combined contract: for any monitor index/name pair, the four helpers
    /// agree on the same primary judgement.
    #[test]
    fn helpers_agree_on_primary_judgement() {
        let primary_name = "DP-1";
        let monitors: Vec<MonitorName<'_>> = vec![Some("HDMI-1"), Some("DP-1"), Some("HDMI-2")];
        for (i, m) in monitors.iter().copied().enumerate() {
            let is_primary = is_primary_monitor(i, m, Some(Some(primary_name)));
            assert_eq!(
                is_primary,
                i == 1,
                "monitor {i} should be primary iff name matches"
            );
            let label = tip_window_label(i, is_primary);
            let url = tip_window_url(is_primary);
            if is_primary {
                assert_eq!(label, "tip-window-0");
                assert_eq!(url, "tip.html");
            } else {
                assert!(label.starts_with("tip-window-minimal-"));
                assert_eq!(url, "tip-minimal.html");
            }
            assert!(is_tip_window_label(&label));
        }
    }
}
