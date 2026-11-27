// SPDX-License-Identifier: GPL-3.0-or-later
//! Platform policy for the settings schema — the single place where the host
//! decides which values/defaults/rows exist. Both the schema served to the UI
//! and the sanitizer applied on disk go through `apply()`, so a Windows-only
//! choice can never survive as an invisible-but-"active" preference on macOS.
//!
//! Rules (6.5 §4): read `PlatformCapabilities`, never `#[cfg]`; unknown hosts
//! (`os == ""`, vite-only) keep the Windows-first default set intact.

use crate::platform::PlatformCapabilities;
use super::types::SettingDefinition;
use serde_json::{json, Value};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SettingPlatformRule {
    /// Keep the key in the document, but do not render a row in Settings.
    pub hidden: bool,
    /// Replace the schema enum list (e.g. drop PowerToys Peek on macOS).
    pub enum_values: Option<Vec<String>>,
    /// Replace the schema default on this host.
    pub default: Option<Value>,
    /// Neutral host-specific fallback copy (the UI still prefers i18n).
    pub description: Option<String>,
}

/// Build the rule for one definition. Pure: safe to unit-test with synthetic
/// capabilities for Windows, macOS and the unknown sentinel.
pub fn rule_for(key: &str, definition: &SettingDefinition, caps: &PlatformCapabilities) -> SettingPlatformRule {
    // Unknown host: never remove anything, so a vite-only preview cannot
    // silently hide Windows settings before the capability IPC answers.
    if caps.os.is_empty() {
        return SettingPlatformRule::default();
    }

    let mut rule = SettingPlatformRule::default();
    match key {
        "core.launchAtStartup" | "core.launchAtStartupHidden" => {
            // Store (MSIX) builds own autostart via windows.startupTask; the
            // in-app HKCU Run toggle is virtualised there, so hide it and let
            // the UI point users to Settings → Apps → Startup instead.
            rule.hidden = caps.store_build
                || (!caps.registry_autostart && !caps.native_autostart);
        }
        "core.everythingPreferred" => {
            rule.hidden = caps.global_search_engine != "everything";
            if rule.hidden {
                rule.default = Some(json!(false));
            }
        }
        "core.fileDoubleClickAction" => {
            if !caps.native_open_with_dialog {
                rule.enum_values = Some(
                    definition
                        .enum_values
                        .clone()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|value| value != "openWithDialog")
                        .collect(),
                );
            }
        }
        "core.closeBehavior" => {
            if !caps.system_tray {
                rule.enum_values = Some(
                    definition
                        .enum_values
                        .clone()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|value| value != "tray")
                        .collect(),
                );
            }
        }
        "preview.externalProvider" => {
            let allowed = allowed_preview_values(caps);
            rule.enum_values = Some(allowed);
            if !caps.peek_preview {
                rule.description = Some(
                    "Space uses the system Quick Look when available, then the built-in preview pane. Peek is Windows-only."
                        .to_string(),
                );
            }
        }
        _ => {}
    }
    rule
}

/// Ordered, host-valid values for `preview.externalProvider`. Reused by the
/// bridge so a persisted Windows-only `peek` degrades to `auto` before use.
pub fn allowed_preview_values(caps: &PlatformCapabilities) -> Vec<String> {
    let mut values = vec!["auto".to_string()];
    if caps.quick_look_preview {
        values.push("quicklook".to_string());
    }
    if caps.peek_preview {
        values.push("peek".to_string());
    }
    values.extend(["builtin".to_string(), "off".to_string()]);
    values
}

/// Apply the rule in place. Every schema/document path calls this.
pub fn apply(definition: &mut SettingDefinition, caps: &PlatformCapabilities) {
    let rule = rule_for(&definition.key, definition, caps);
    definition.hidden = rule.hidden;
    if let Some(values) = rule.enum_values {
        definition.enum_values = Some(values);
    }
    if let Some(default) = rule.default {
        definition.default = default;
    }
    if let Some(description) = rule.description {
        definition.description = description;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::defaults::raw_setting_definitions;
    use serde_json::json;

    fn caps(os: &str) -> PlatformCapabilities {
        let mut caps = PlatformCapabilities::neutral(os, "ctrl", '\\');
        caps.registry_autostart = true;
        caps.system_tray = true;
        caps.native_open_with_dialog = true;
        caps.quick_look_preview = true;
        caps.peek_preview = true;
        caps.global_search_engine = "everything".to_string();
        caps
    }

    fn definition(key: &str) -> SettingDefinition {
        raw_setting_definitions()
            .into_iter()
            .find(|def| def.key == key)
            .unwrap()
    }

    #[test]
    fn unknown_host_keeps_windows_first_schema() {
        let mut def = definition("preview.externalProvider");
        apply(&mut def, &caps(""));
        assert!(!def.hidden);
        assert_eq!(
            def.enum_values,
            Some(vec![
                "auto".into(),
                "peek".into(),
                "quicklook".into(),
                "builtin".into(),
                "off".into(),
            ])
        );
    }

    #[test]
    fn mac_drops_peek_and_hides_windows_only_rows() {
        let mut mac = caps("macos");
        mac.registry_autostart = false;
        mac.native_open_with_dialog = false;
        mac.peek_preview = false;
        mac.global_search_engine = "spotlight".to_string();

        let mut preview = definition("preview.externalProvider");
        apply(&mut preview, &mac);
        assert_eq!(
            preview.enum_values,
            Some(vec![
                "auto".into(),
                "quicklook".into(),
                "builtin".into(),
                "off".into(),
            ])
        );
        assert!(!preview.description.contains("Peek CLI"));

        let mut startup = definition("core.launchAtStartup");
        apply(&mut startup, &mac);
        assert!(startup.hidden);

        let mut everything = definition("core.everythingPreferred");
        apply(&mut everything, &mac);
        assert!(everything.hidden);
        assert_eq!(everything.default, json!(false));

        let mut dblclick = definition("core.fileDoubleClickAction");
        apply(&mut dblclick, &mac);
        assert!(!dblclick.enum_values.unwrap().contains(&"openWithDialog".to_string()));
    }

    #[test]
    fn store_build_hides_in_app_autostart_rows() {
        let mut store = caps("windows");
        store.store_build = true;
        for key in ["core.launchAtStartup", "core.launchAtStartupHidden"] {
            let mut def = definition(key);
            apply(&mut def, &store);
            assert!(def.hidden, "{key} must be hidden in a Store build");
        }
    }

    #[test]
    fn windows_schema_stays_unchanged() {
        let mut preview = definition("preview.externalProvider");
        apply(&mut preview, &caps("windows"));
        assert!(!preview.hidden);
        assert!(preview.enum_values.unwrap().contains(&"peek".to_string()));
        assert_eq!(preview.default, json!("auto"));
    }
}
