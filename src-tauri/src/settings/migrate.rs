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
}
