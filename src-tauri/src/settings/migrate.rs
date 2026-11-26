// SPDX-License-Identifier: GPL-3.0-or-later

use super::defaults::default_document;
use super::sanitize::sanitize_document;
use super::types::{SettingsDocument, SettingsMigrateResult, SETTINGS_SCHEMA_VERSION};
use serde_json::Value;
use std::collections::BTreeMap;

/// Accept `{ version, values }` or a flat map with optional `version` / `__schemaVersion`.
pub fn parse_raw_document(raw: &Value) -> Result<(u32, BTreeMap<String, Value>), String> {
    let object = raw
        .as_object()
        .ok_or_else(|| "Settings document must be a JSON object".to_string())?;

    if let (Some(version_value), Some(values_value)) = (object.get("version"), object.get("values"))
    {
        let version = version_value
            .as_u64()
            .ok_or_else(|| "settings.version must be a number".to_string())?
            as u32;
        let values_obj = values_value
            .as_object()
            .ok_or_else(|| "settings.values must be an object".to_string())?;
        let mut values = BTreeMap::new();
        for (key, value) in values_obj {
            values.insert(key.clone(), value.clone());
        }
        return Ok((version, values));
    }

    let mut values = BTreeMap::new();
    let mut version = 0u32;
    for (key, value) in object {
        if key == "__schemaVersion" || key == "version" {
            if let Some(number) = value.as_u64() {
                version = number as u32;
            }
            continue;
        }
        if key == "values" {
            continue;
        }
        values.insert(key.clone(), value.clone());
    }
    Ok((version, values))
}

pub fn migrate_to_latest(raw: &Value) -> Result<SettingsMigrateResult, String> {
    let (mut from_version, mut values) = parse_raw_document(raw)?;
    let original_version = from_version;
    let mut notes = Vec::new();

    if from_version > SETTINGS_SCHEMA_VERSION {
        return Err(format!(
            "Settings schema version {from_version} is newer than supported {SETTINGS_SCHEMA_VERSION}"
        ));
    }

    while from_version < SETTINGS_SCHEMA_VERSION {
        let next = from_version + 1;
        migrate_step(&mut values, from_version, next, &mut notes)?;
        from_version = next;
    }

    let document = sanitize_document(SettingsDocument {
        version: SETTINGS_SCHEMA_VERSION,
        values,
    });

    Ok(SettingsMigrateResult {
        document,
        from_version: original_version,
        to_version: SETTINGS_SCHEMA_VERSION,
        migrated: original_version != SETTINGS_SCHEMA_VERSION,
        notes,
    })
}

fn migrate_step(
    values: &mut BTreeMap<String, Value>,
    from: u32,
    to: u32,
    notes: &mut Vec<String>,
) -> Result<(), String> {
    match (from, to) {
        (0, 1) => {
            if let Some(theme) = values.remove("theme") {
                values.insert("core.theme".to_string(), theme);
                notes.push("Renamed theme → core.theme".to_string());
            }
            if let Some(language) = values.remove("language") {
                let locale = language
                    .as_str()
                    .map(|text| Value::String(text.to_string()))
                    .or_else(|| {
                        language
                            .get("locale")
                            .and_then(|v| v.as_str())
                            .map(|text| Value::String(text.to_string()))
                    })
                    .unwrap_or(Value::String("en".to_string()));
                values.insert("core.language".to_string(), locale);
                notes.push("Renamed language → core.language".to_string());
            }
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push("Migrated flat/legacy settings to schema v1".to_string());
            Ok(())
        }
        (1, 2) => {
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push("Added new-file/folder default name settings (schema v2)".to_string());
            Ok(())
        }
        (2, 3) => {
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push("Added experimental.previewPane (schema v3)".to_string());
            Ok(())
        }
        (3, 4) => {
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push(
                "Added open-tool toggles/paths + navigator view mode (schema v4)".to_string(),
            );
            Ok(())
        }
        (4, 5) => {
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            // Preserve list/details; new icon sizes are opt-in via UI.
            notes.push(
                "Expanded navigator view modes (icons large/medium/small; schema v5)".to_string(),
            );
            Ok(())
        }
        (5, 6) => {
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push("Added core.newTabBehavior (guide | duplicate; schema v6)".to_string());
            Ok(())
        }
        (6, 7) => {
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push(
                "Added preview.externalProvider (auto|peek|quicklook|builtin|off; schema v7)"
                    .to_string(),
            );
            Ok(())
        }
        (7, 8) => {
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push(
                "Added power.providerPreloadKnowledge (schema v8)".to_string(),
            );
            Ok(())
        }
        (8, 9) => {
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push(
                "Added core.projectsRoot (Projects Root; schema v9)".to_string(),
            );
            Ok(())
        }
        (9, 10) => {
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push(
                "Added power.awamGatewayUrl (schema v10)".to_string(),
            );
            Ok(())
        }
        (10, 11) => {
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push(
                "Added core.openTool.qwenwork.* + core.openTool.custom (schema v11)".to_string(),
            );
            Ok(())
        }
        (11, 12) => {
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push(
                "Added power.awamKnowledgeServiceUrl + power.awamKnowledgeServiceToken (schema v12)"
                    .to_string(),
            );
            Ok(())
        }
        (12, 13) => {
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push(
                "Added voice input keys power.voiceSearchAutoStart + power.sttBaseUrl/sttModel/sttApiKey (schema v13)"
                    .to_string(),
            );
            Ok(())
        }
        (13, 14) => {
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push(
                "Added power.awamKnowledgeDataRoot + power.awamKnowledgeSourceId (schema v14)"
                    .to_string(),
            );
            Ok(())
        }
        (14, 15) => {
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push("Added core.openTool.order (schema v15)".to_string());
            Ok(())
        }
        (15, 16) => {
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push(
                "Added power.awamKnowledgeSecretScript/SecretAgentToken/SecretId (schema v16)"
                    .to_string(),
            );
            Ok(())
        }
        (16, 17) => {
            // ADR 0030：布尔分组开关 → 分组字段枚举（true→type；false/缺省→none）。
            // 新键（排序字段/方向、分组方向）由 defaults 补齐；or_insert 不会覆盖
            // 已迁移出的 navigatorGroupBy。
            let group_by = match values.get("core.navigatorGroupByType") {
                Some(Value::Bool(true)) => "type",
                _ => "none",
            };
            if !values.contains_key("core.navigatorGroupBy") {
                values.insert(
                    "core.navigatorGroupBy".to_string(),
                    Value::String(group_by.to_string()),
                );
                notes.push(format!(
                    "Converted core.navigatorGroupByType → core.navigatorGroupBy={group_by} (schema v17)"
                ));
            }
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            Ok(())
        }
        (17, 18) => {
            // ADR 0031: drill depth level (1..5, default 2). Additive only —
            // no old value is migrated; the drill switch itself is session state.
            for (key, value) in default_document().values {
                values.entry(key).or_insert(value);
            }
            notes.push("Added core.navigatorDrillDepth (schema v18)".to_string());
            Ok(())
        }
        _ => Err(format!("No migration path from {from} → {to}")),
    }
}

pub fn apply_patch(
    mut document: SettingsDocument,
    patch: BTreeMap<String, Value>,
) -> Result<SettingsDocument, String> {
    for (key, value) in patch {
        document.values.insert(key, value);
    }
    Ok(sanitize_document(document))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn migrates_legacy_flat_map() {
        let raw = json!({
            "theme": "dark",
            "language": "en"
        });
        let result = migrate_to_latest(&raw).unwrap();
        assert!(result.migrated);
        assert_eq!(result.document.version, SETTINGS_SCHEMA_VERSION);
        assert_eq!(result.document.values.get("core.theme"), Some(&json!("dark")));
        assert_eq!(
            result.document.values.get("core.listDensity"),
            Some(&json!("standard"))
        );
    }

    #[test]
    fn apply_patch_round_trips_core_field() {
        let doc = default_document();
        let mut patch = BTreeMap::new();
        patch.insert("core.showHidden".to_string(), json!(true));
        let next = apply_patch(doc, patch).unwrap();
        assert_eq!(next.values.get("core.showHidden"), Some(&json!(true)));
        assert_eq!(next.values.get("core.listDensity"), Some(&json!("standard")));
    }

    fn v16_doc(group_by_type: Option<bool>) -> Value {
        let mut values = json!({});
        if let Some(flag) = group_by_type {
            values["core.navigatorGroupByType"] = json!(flag);
        }
        json!({ "version": 16, "values": values })
    }

    #[test]
    fn migrates_v16_group_by_type_true_to_type() {
        let result = migrate_to_latest(&v16_doc(Some(true))).unwrap();
        assert_eq!(result.from_version, 16);
        assert_eq!(
            result.document.values.get("core.navigatorGroupBy"),
            Some(&json!("type"))
        );
    }

    #[test]
    fn migrates_v16_group_by_type_false_to_none() {
        let result = migrate_to_latest(&v16_doc(Some(false))).unwrap();
        assert_eq!(
            result.document.values.get("core.navigatorGroupBy"),
            Some(&json!("none"))
        );
    }

    #[test]
    fn migrates_v16_missing_group_by_type_to_none_and_fills_sort_keys() {
        let result = migrate_to_latest(&v16_doc(None)).unwrap();
        assert_eq!(
            result.document.values.get("core.navigatorGroupBy"),
            Some(&json!("none"))
        );
        assert_eq!(
            result.document.values.get("core.navigatorSortColumn"),
            Some(&json!("name"))
        );
        assert_eq!(
            result.document.values.get("core.navigatorSortDirection"),
            Some(&json!("asc"))
        );
        assert_eq!(
            result.document.values.get("core.navigatorGroupDirection"),
            Some(&json!("asc"))
        );
    }

    #[test]
    fn migration_keeps_existing_v17_group_by_value() {
        let raw = json!({
            "version": 16,
            "values": { "core.navigatorGroupBy": "size" }
        });
        let result = migrate_to_latest(&raw).unwrap();
        assert_eq!(
            result.document.values.get("core.navigatorGroupBy"),
            Some(&json!("size"))
        );
    }

    #[test]
    fn migrates_v17_to_v18_filling_default_drill_depth() {
        let raw = json!({ "version": 17, "values": {} });
        let result = migrate_to_latest(&raw).unwrap();
        assert_eq!(result.from_version, 17);
        assert_eq!(result.to_version, SETTINGS_SCHEMA_VERSION);
        // Number keys are sanitized through f64, so the stored value is 2.0.
        assert_eq!(
            result.document.values.get("core.navigatorDrillDepth"),
            Some(&json!(2.0))
        );
    }

    #[test]
    fn migration_keeps_existing_v18_drill_depth() {
        let raw = json!({
            "version": 17,
            "values": { "core.navigatorDrillDepth": 4 }
        });
        let result = migrate_to_latest(&raw).unwrap();
        assert_eq!(
            result.document.values.get("core.navigatorDrillDepth"),
            Some(&json!(4.0))
        );
    }
}
