// SPDX-License-Identifier: GPL-3.0-or-later

use super::types::{
    SettingDefinition, SettingValueKind, SettingsDocument, SettingsSchemaManifest, SettingsTier,
    SETTINGS_SCHEMA_VERSION,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub fn schema_manifest() -> SettingsSchemaManifest {
    SettingsSchemaManifest {
        version: SETTINGS_SCHEMA_VERSION,
        tiers: vec![
            SettingsTier::Core,
            SettingsTier::Power,
            SettingsTier::Experimental,
        ],
        definitions: setting_definitions(),
    }
}

pub fn setting_definitions() -> Vec<SettingDefinition> {
    vec![
        def(
            "core.language",
            SettingsTier::Core,
            "Language",
            "UI language: en | zh-CN | ja | de | fr",
            SettingValueKind::Enum,
            Some(vec![
                "en".to_string(),
                "zh-CN".to_string(),
                "ja".to_string(),
                "de".to_string(),
                "fr".to_string(),
            ]),
            json!("en"),
        ),
        def(
            "core.theme",
            SettingsTier::Core,
            "Theme",
            "Color theme: light | dark | system",
            SettingValueKind::Enum,
            Some(vec![
                "light".to_string(),
                "dark".to_string(),
                "system".to_string(),
            ]),
            json!("system"),
        ),
        def(
            "core.defaultLocation",
            SettingsTier::Core,
            "Default location",
            "Default OS path opened in Navigator (empty = USERPROFILE)",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "core.startupPage",
            SettingsTier::Core,
            "Startup page",
            "First page after launch: navigator | projects | discovery",
            SettingValueKind::Enum,
            Some(vec![
                "navigator".to_string(),
                "projects".to_string(),
                "discovery".to_string(),
            ]),
            json!("navigator"),
        ),
        def(
            "core.showHidden",
            SettingsTier::Core,
            "Show hidden items",
            "Include hidden files/folders in Kernel listings",
            SettingValueKind::Boolean,
            None,
            json!(false),
        ),
        def(
            "core.listDensity",
            SettingsTier::Core,
            "List density",
            "File list density; chrome stays Standard. Default: standard",
            SettingValueKind::Enum,
            Some(vec!["standard".to_string(), "compact".to_string()]),
            json!("standard"),
        ),
        def(
            "core.everythingPreferred",
            SettingsTier::Core,
            "Everything preferred",
            "Prefer Everything IPC for Discovery when available",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
        def(
            "core.extensionsEnabled",
            SettingsTier::Core,
            "Extensions enabled",
            "Master switch for the extension host",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
        def(
            "core.launchAtStartup",
            SettingsTier::Core,
            "Launch at startup",
            "Start A File Manager when you sign in to Windows",
            SettingValueKind::Boolean,
            None,
            json!(false),
        ),
        def(
            "core.launchAtStartupHidden",
            SettingsTier::Core,
            "Start hidden to tray",
            "When launched at login, hide the main window to the system tray (taskbar notification area)",
            SettingValueKind::Boolean,
            None,
            json!(false),
        ),
        def(
            "core.closeBehavior",
            SettingsTier::Core,
            "When closing the window",
            "Ask each time, hide to the system tray, or quit the app",
            SettingValueKind::Enum,
            Some(vec![
                "ask".to_string(),
                "tray".to_string(),
                "quit".to_string(),
            ]),
            json!("ask"),
        ),
        def(
            "core.discoveryBlacklist",
            SettingsTier::Core,
            "Discovery directory blacklist",
            "Directories excluded from Discovery search results (one path per line in Settings)",
            SettingValueKind::StringArray,
            None,
            json!([]),
        ),
        def(
            "core.newFolderDefaultName",
            SettingsTier::Core,
            "Default new folder name",
            "Suggested name when creating a folder (empty = use current language default)",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "core.newFileDefaultName",
            SettingsTier::Core,
            "Default new file name",
            "Suggested name when creating a file (empty = use current language default)",
            SettingValueKind::String,
            None,
            json!(""),
        ),
        def(
            "core.fileDoubleClickAction",
            SettingsTier::Core,
            "File double-click action",
            "What happens when double-clicking a file in Navigator (AIA-118)",
            SettingValueKind::Enum,
            Some(vec!["systemDefault".into(), "none".into()]),
            json!("systemDefault"),
        ),
        def(
            "power.confirmBeforeDelete",
            SettingsTier::Power,
            "Confirm before delete",
            "Ask before permanent delete operations",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
        def(
            "power.perPathOverridesEnabled",
            SettingsTier::Power,
            "Per-path overrides",
            "Allow per-path navigator defaults to override group/global",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
        def(
            "experimental.debugStatusInTitlebar",
            SettingsTier::Experimental,
            "Debug status in titlebar",
            "Show phase/debug caption in the custom titlebar",
            SettingValueKind::Boolean,
            None,
            json!(true),
        ),
    ]
}

fn def(
    key: &str,
    tier: SettingsTier,
    title: &str,
    description: &str,
    value_kind: SettingValueKind,
    enum_values: Option<Vec<String>>,
    default: Value,
) -> SettingDefinition {
    SettingDefinition {
        key: key.to_string(),
        tier,
        title: title.to_string(),
        description: description.to_string(),
        value_kind,
        enum_values,
        default,
    }
}

pub fn default_document() -> SettingsDocument {
    let mut values = BTreeMap::new();
    for definition in setting_definitions() {
        values.insert(definition.key, definition.default);
    }
    SettingsDocument {
        version: SETTINGS_SCHEMA_VERSION,
        values,
    }
}

pub fn definition_map() -> BTreeMap<String, SettingDefinition> {
    setting_definitions()
        .into_iter()
        .map(|definition| (definition.key.clone(), definition))
        .collect()
}
