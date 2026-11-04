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
            "UI language: en | zh-CN",
            SettingValueKind::Enum,
            Some(vec!["en".to_string(), "zh-CN".to_string()]),
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
