// SPDX-License-Identifier: GPL-3.0-or-later

use super::defaults::definition_map;
use super::types::{SettingDefinition, SettingValueKind, SettingsDocument, SETTINGS_SCHEMA_VERSION};
use serde_json::Value;
use std::collections::BTreeMap;

/// Drop unknown keys, coerce/repair bad values to defaults, stamp schema version.
pub fn sanitize_document(mut document: SettingsDocument) -> SettingsDocument {
    let definitions = definition_map();
    let mut sanitized = BTreeMap::new();

    for (key, definition) in &definitions {
        let value = document
            .values
            .remove(key)
            .map(|value| coerce_value(value, definition))
            .unwrap_or_else(|| definition.default.clone());
        sanitized.insert(key.clone(), value);
    }

    document.version = SETTINGS_SCHEMA_VERSION;
    document.values = sanitized;
    document
}

fn coerce_value(value: Value, definition: &SettingDefinition) -> Value {
    match definition.value_kind {
        SettingValueKind::Boolean => Value::Bool(
            value
                .as_bool()
                .unwrap_or_else(|| definition.default.as_bool().unwrap_or(false)),
        ),
        SettingValueKind::String => value
            .as_str()
            .map(|text| Value::String(text.to_string()))
            .unwrap_or_else(|| definition.default.clone()),
        SettingValueKind::Number => value
            .as_f64()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or_else(|| definition.default.clone()),
        SettingValueKind::StringArray => {
            if let Some(array) = value.as_array() {
                let strings: Vec<Value> = array
                    .iter()
                    .filter_map(|item| item.as_str().map(|text| Value::String(text.to_string())))
                    .collect();
                Value::Array(strings)
            } else {
                definition.default.clone()
            }
        }
        SettingValueKind::Enum => {
            let text = value.as_str().map(str::to_string);
            match (text, definition.enum_values.as_ref()) {
                (Some(candidate), Some(allowed))
                    if allowed.iter().any(|item| item == &candidate) =>
                {
                    Value::String(candidate)
                }
                _ => definition.default.clone(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn drops_unknown_and_repairs_enum() {
        let mut values = BTreeMap::new();
        values.insert("core.listDensity".to_string(), json!("ultra"));
        values.insert("legacy.junk".to_string(), json!(true));
        let doc = sanitize_document(SettingsDocument { version: 0, values });
        assert_eq!(doc.version, SETTINGS_SCHEMA_VERSION);
        assert_eq!(doc.values.get("core.listDensity"), Some(&json!("standard")));
        assert!(!doc.values.contains_key("legacy.junk"));
    }
}
