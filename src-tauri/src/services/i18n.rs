use std::sync::Mutex;

use tracing::warn;

use crate::error::Result;
use crate::services::{Service, ServiceContext};

const DEFAULT_LOCALE: &str = "zh-CN";
const ENGLISH_LOCALE: &str = "en";

#[cfg(test)]
const ALL_KEYS: [&str; 11] = [
    "tray.settings",
    "tray.about",
    "tray.pause",
    "tray.resume",
    "tray.quit",
    "tray.tooltip.working",
    "tray.tooltip.resting",
    "tray.tooltip.paused",
    "tray.tooltip.alerting",
    "tray.tooltip.pre_alert",
    "tray.tooltip.app_name",
];

pub(crate) struct I18nService {
    locale: Mutex<String>,
}

impl I18nService {
    #[must_use]
    pub(crate) fn new(initial_locale: &str) -> Self {
        let locale = Self::normalize_locale(initial_locale).unwrap_or(DEFAULT_LOCALE);
        Self {
            locale: Mutex::new(locale.to_string()),
        }
    }

    #[must_use]
    pub(crate) fn get(&self, key: &str) -> &'static str {
        let locale = match self.locale.lock() {
            Ok(guard) => guard.clone(),
            Err(err) => {
                warn!("i18n locale mutex poisoned: {err}");
                err.into_inner().clone()
            }
        };
        let text = Self::translate(&locale, key);
        if text == "???" {
            warn!("missing i18n key: {key}");
        }
        text
    }

    pub(crate) fn set_locale(&self, new_locale: &str) {
        let Some(locale) = Self::normalize_locale(new_locale) else {
            warn!("ignoring unsupported locale: {new_locale}");
            return;
        };

        match self.locale.lock() {
            Ok(mut guard) => {
                *guard = locale.to_string();
            }
            Err(err) => {
                warn!("i18n locale mutex poisoned while setting locale: {err}");
                *err.into_inner() = locale.to_string();
            }
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn all_keys() -> &'static [&'static str] {
        &ALL_KEYS
    }

    fn normalize_locale(locale: &str) -> Option<&'static str> {
        match locale {
            "zh-CN" => Some(DEFAULT_LOCALE),
            "en" | "en-US" => Some(ENGLISH_LOCALE),
            _ => None,
        }
    }

    fn translate(locale: &str, key: &str) -> &'static str {
        match (locale, key) {
            ("zh-CN", "tray.settings") => "设置",
            ("zh-CN", "tray.about") => "关于",
            ("zh-CN", "tray.pause") => "暂停",
            ("zh-CN", "tray.resume") => "继续",
            ("zh-CN", "tray.quit") => "退出",
            ("zh-CN", "tray.tooltip.working") => "工作中",
            ("zh-CN", "tray.tooltip.resting") => "休息中",
            ("zh-CN", "tray.tooltip.paused") => "已暂停",
            ("zh-CN", "tray.tooltip.alerting") => "该休息了",
            ("zh-CN", "tray.tooltip.pre_alert") => "即将休息",
            (_, "tray.tooltip.app_name") => "Eyezen",
            (_, "tray.settings") => "Settings",
            (_, "tray.about") => "About",
            (_, "tray.pause") => "Pause",
            (_, "tray.resume") => "Resume",
            (_, "tray.quit") => "Quit",
            (_, "tray.tooltip.working") => "Working",
            (_, "tray.tooltip.resting") => "Resting",
            (_, "tray.tooltip.paused") => "Paused",
            (_, "tray.tooltip.alerting") => "Time to rest",
            (_, "tray.tooltip.pre_alert") => "Break soon",
            _ => "???",
        }
    }
}

impl Service for I18nService {
    async fn init(&self, _ctx: &ServiceContext) -> Result<()> {
        Ok(())
    }

    async fn start(&self, _ctx: &ServiceContext) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_returns_zh_cn_by_default() {
        let svc = I18nService::new("zh-CN");
        assert_eq!(svc.get("tray.settings"), "设置");
        assert_eq!(svc.get("tray.quit"), "退出");
    }

    #[test]
    fn get_returns_en_after_set_locale() {
        let svc = I18nService::new("zh-CN");
        svc.set_locale("en");
        assert_eq!(svc.get("tray.settings"), "Settings");
        assert_eq!(svc.get("tray.quit"), "Quit");
    }

    #[test]
    fn en_us_alias_maps_to_en() {
        let svc = I18nService::new("en-US");
        assert_eq!(svc.get("tray.settings"), "Settings");
    }

    #[test]
    fn unknown_locale_defaults_to_zh_cn() {
        let svc = I18nService::new("fr-FR");
        assert_eq!(svc.get("tray.settings"), "设置");
    }

    #[test]
    fn unknown_key_returns_fallback() {
        let svc = I18nService::new("zh-CN");
        assert_eq!(svc.get("nonexistent.key"), "???");
    }

    #[test]
    fn unsupported_runtime_locale_is_ignored() {
        let svc = I18nService::new("zh-CN");
        svc.set_locale("ja-JP");
        assert_eq!(svc.get("tray.settings"), "设置");
    }

    #[test]
    fn all_zh_cn_keys_have_en_counterparts() {
        let svc_zh = I18nService::new("zh-CN");
        let svc_en = I18nService::new("en");
        let keys = I18nService::all_keys();
        for key in keys {
            let zh = svc_zh.get(key);
            let en = svc_en.get(key);
            assert_ne!(zh, "???", "zh-CN missing key: {key}");
            assert_ne!(en, "???", "en missing key: {key}");
        }
    }
}
