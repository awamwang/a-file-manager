// SPDX-License-Identifier: GPL-3.0-or-later
//! Pack 配置项的动态 schema overlay（发现机制的落盘支撑）。
//!
//! Feature Pack 在 `pack.json` 的 `contributions.config[]` 声明「需要用户填的配置项」。
//! 这些项的值仍存进 settings 文档（与内置字段统一存储、统一 sanitize），但它们的 schema
//! 定义不在内置静态 list 里——`sanitize_document` 会丢弃任何不在 `definition_map()` 中的 key。
//! 因此本模块维护一个由「当前已启用 Pack 的 config 声明」派生的 definitions 叠加层：
//! - `definition_map()` / `schema_manifest()` / `sanitize` 合并内置 + 本叠加层；
//! - 叠加层项一律 `hidden = true`，故不进主设置页（SettingsView 过滤 hidden），由服务配置页专管；
//! - 内置已有的 key（迁移既有 `power.*` 字段时）以内置定义为准，叠加层不覆盖、不重复。
//!
//! 叠加层由 `extension_host` 在 Pack 加载/启用/禁用后整体重算（`sync_pack_configs`），
//! 纯进程内全局态，不持久化（settings 文档才是值的真相源；重启后由启用 Pack 重新派生）。

use super::types::{SettingDefinition, SettingValueKind, SettingsTier};
use crate::extension_host::types::{ConfigContribution, ConfigValueKind};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

/// 缺省 settingKey 时的命名空间前缀：`ext.<packId>.<configId>`。
pub const EXT_SETTING_PREFIX: &str = "ext.";

fn overlay() -> &'static Mutex<BTreeMap<String, SettingDefinition>> {
    static OVERLAY: OnceLock<Mutex<BTreeMap<String, SettingDefinition>>> = OnceLock::new();
    OVERLAY.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// 计算某条 config 声明生效的 settings key：显式 `settingKey` 优先，否则 `ext.<packId>.<id>`。
pub fn effective_setting_key(pack_id: &str, contrib: &ConfigContribution) -> String {
    match contrib.setting_key.as_deref().map(str::trim) {
        Some(explicit) if !explicit.is_empty() => explicit.to_string(),
        _ => format!("{EXT_SETTING_PREFIX}{pack_id}.{}", contrib.id),
    }
}

fn value_kind_from_config(kind: ConfigValueKind) -> SettingValueKind {
    match kind {
        ConfigValueKind::String | ConfigValueKind::Secret => SettingValueKind::String,
        ConfigValueKind::Number => SettingValueKind::Number,
        ConfigValueKind::Boolean => SettingValueKind::Boolean,
        ConfigValueKind::Enum => SettingValueKind::Enum,
    }
}

/// 把字符串化默认值按 kind 转成 settings Value；缺省/不可解析时回退到该 kind 的中性默认。
fn default_value_from_config(kind: ConfigValueKind, raw: Option<&str>) -> Value {
    let raw = raw.map(str::trim).filter(|s| !s.is_empty());
    match kind {
        ConfigValueKind::Number => raw
            .and_then(|s| s.parse::<f64>().ok())
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or_else(|| json!(0)),
        ConfigValueKind::Boolean => match raw {
            Some("true") | Some("1") | Some("yes") => json!(true),
            _ => json!(false),
        },
        ConfigValueKind::Enum => {
            // 默认值必须是候选之一，否则回退首个候选 / 空串。
            raw.map(|text| Value::String(text.to_string())).unwrap_or_else(|| json!(""))
        }
        ConfigValueKind::String | ConfigValueKind::Secret => {
            raw.map(|text| Value::String(text.to_string())).unwrap_or_else(|| json!(""))
        }
    }
}

/// 单条 config 声明 → settings schema definition（hidden，不进主设置页）。
pub fn definition_from_config(pack_id: &str, contrib: &ConfigContribution) -> SettingDefinition {
    SettingDefinition {
        key: effective_setting_key(pack_id, contrib),
        tier: SettingsTier::Power,
        title: if contrib.title.is_empty() {
            contrib.id.clone()
        } else {
            contrib.title.clone()
        },
        description: contrib.description.clone(),
        value_kind: value_kind_from_config(contrib.kind),
        enum_values: contrib.enum_values.clone(),
        default: default_value_from_config(contrib.kind, contrib.default.as_deref()),
        hidden: true,
    }
}

/// 由「已启用 Pack 的 config 声明」整体重算叠加层。`entries` 为 (packId, contributions) 列表。
/// 同一 settings key 去重（首个 pack 胜出，避免冲突覆盖）。
pub fn sync_pack_configs(entries: Vec<(String, Vec<ConfigContribution>)>) {
    let mut next = BTreeMap::new();
    for (pack_id, contributions) in entries {
        for contrib in contributions {
            let definition = definition_from_config(&pack_id, &contrib);
            next.entry(definition.key.clone()).or_insert(definition);
        }
    }
    if let Ok(mut guard) = overlay().lock() {
        *guard = next;
    }
}

/// 当前叠加层 definitions（按 key 排序）。内置已有的 key 不重复返回（由 `definition_map` 合并时去重）。
pub fn overlay_definitions() -> Vec<SettingDefinition> {
    overlay()
        .lock()
        .map(|guard| guard.values().cloned().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension_host::types::ConfigValueKind;

    fn contrib(id: &str, kind: ConfigValueKind, setting_key: Option<&str>) -> ConfigContribution {
        ConfigContribution {
            id: id.to_string(),
            title: format!("Title {id}"),
            title_key: None,
            description: String::new(),
            description_key: None,
            kind,
            enum_values: None,
            required: false,
            managed_readonly: false,
            default: None,
            env: None,
            setting_key: setting_key.map(str::to_string),
            order: 0,
            pack_id: None,
            pack_name: None,
        }
    }

    #[test]
    fn effective_setting_key_uses_explicit_or_namespaced() {
        let explicit = contrib("token", ConfigValueKind::Secret, Some("power.awamKnowledgeServiceToken"));
        assert_eq!(
            effective_setting_key("awam.knowledge", &explicit),
            "power.awamKnowledgeServiceToken"
        );
        let implicit = contrib("prowlarr-api-key", ConfigValueKind::Secret, None);
        assert_eq!(
            effective_setting_key("awam.file-gateway", &implicit),
            "ext.awam.file-gateway.prowlarr-api-key"
        );
    }

    #[test]
    fn definition_from_config_maps_kind_and_defaults_and_is_hidden() {
        let secret = definition_from_config("p", &contrib("k", ConfigValueKind::Secret, None));
        assert_eq!(secret.value_kind, SettingValueKind::String);
        assert_eq!(secret.default, json!(""));
        assert!(secret.hidden);

        let mut num = contrib("n", ConfigValueKind::Number, None);
        num.default = Some("18787".to_string());
        let number = definition_from_config("p", &num);
        assert_eq!(number.value_kind, SettingValueKind::Number);
        // Number 默认值经 from_f64 落为浮点形态（与 sanitize 的 Number coerce 一致）。
        assert_eq!(number.default, json!(18787.0));

        let mut boolean = contrib("b", ConfigValueKind::Boolean, None);
        boolean.default = Some("true".to_string());
        assert_eq!(definition_from_config("p", &boolean).default, json!(true));
    }
}
