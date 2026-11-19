// SPDX-License-Identifier: GPL-3.0-or-later
//! deps_probe 一次性投影（ADR 0020 / Q9a / Q18 / D 决策）：注册表为权威，
//! **有**记录的依赖由注册表覆写状态与可操作性；**无**记录的依赖维持现有探测
//! （即 config-mode 支撑现状，等待依赖项目 pack 化后自动点亮）。
//!
//! 异常处理：注册表读失败仅告警并原样返回，绝不破坏 Health 页。

use super::registry::{self, DependencyMode, HealthState};
use crate::deps_probe::DependencyItem;
use std::path::Path;

/// pack_id → deps_probe 条目 id 映射。新 pack 化依赖在此登记一行即可接入投影。
fn dep_item_id_for_pack(pack_id: &str) -> Option<&'static str> {
    match pack_id {
        "awam.file-gateway" => Some("awam_gateway"),
        // 后续 pack 化时补充，例如：
        // "awam.tool-runtime" => Some("tool_gateway"),
        // "awam.knowledge" => Some("knowledge_service"),
        _ => None,
    }
}

fn mode_str(mode: DependencyMode) -> &'static str {
    match mode {
        DependencyMode::Config => "config",
        DependencyMode::Managed => "managed",
    }
}

/// 双失败态归因（Q18）：status 枚举不扩，用正交 `unavailableReason` 表达。
fn unavailable_reason(record: &registry::ManagedDependencyRecord) -> Option<&'static str> {
    if record.health != HealthState::Unhealthy {
        return None;
    }
    Some(match record.mode {
        DependencyMode::Config => "configOnly",
        DependencyMode::Managed if record.spawned_pid.is_some() => "managedOnly",
        DependencyMode::Managed => "both",
    })
}

/// 就地投影：对映射到注册表记录的条目覆写 mode / actionable / status / unavailableReason。
pub fn enrich_with_registry(items: &mut [DependencyItem], app_data: &Path) {
    let records = match registry::list_records(app_data) {
        Ok(records) => records,
        Err(err) => {
            // 机制与异常处理（D 决策）：注册表不可读 → 告警并退回纯探测展示。
            eprintln!("managed_dependency projection: registry read failed: {err}");
            return;
        }
    };
    for record in records {
        let Some(item_id) = dep_item_id_for_pack(&record.pack_id) else {
            continue;
        };
        let Some(item) = items.iter_mut().find(|item| item.id == item_id) else {
            continue;
        };
        item.mode = Some(mode_str(record.mode).to_string());
        item.actionable = true;
        // 注册表为权威：health 已知时覆写探活结论；Unknown（尚未探测）保留 probe 结果。
        match record.health {
            HealthState::Healthy => item.status = "ok".to_string(),
            HealthState::Unhealthy => item.status = "missing".to_string(),
            HealthState::Unknown => {}
        }
        item.unavailable_reason = unavailable_reason(&record).map(str::to_string);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension_host::types::ExtensionRuntimeState;
    use crate::managed_dependency::registry::{
        register_for_enabled_pack, CredentialSource, ManagedDependencyRecord,
    };
    use std::fs;

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("afm-projection-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn gateway_item(status: &str) -> DependencyItem {
        DependencyItem {
            id: "awam_gateway".to_string(),
            kind: "optional".to_string(),
            status: status.to_string(),
            title: "awam-file gateway".to_string(),
            detail: String::new(),
            fix: String::new(),
            mode: None,
            actionable: false,
            unavailable_reason: None,
        }
    }

    fn sample_items(status: &str) -> Vec<DependencyItem> {
        let mut rclone = gateway_item(status);
        rclone.id = "rclone".to_string();
        rclone.title = "rclone CLI".to_string();
        vec![gateway_item(status), rclone]
    }

    fn enabled_pack(pack_id: &str) -> crate::extension_host::types::InstalledPackRecord {
        crate::extension_host::types::InstalledPackRecord {
            id: pack_id.to_string(),
            name: "awam-file".to_string(),
            version: "0.1.0".to_string(),
            state: ExtensionRuntimeState::Enabled,
            permissions: vec![crate::extension_host::types::ExtensionPermission::SpawnSidecar],
            install_path: "/tmp/pack".to_string(),
            last_error: None,
        }
    }

    #[test]
    fn no_records_leaves_probe_results_untouched() {
        let dir = temp_root("empty");
        let mut items = sample_items("missing");
        enrich_with_registry(&mut items, &dir);
        let gateway = items.iter().find(|i| i.id == "awam_gateway").unwrap();
        assert_eq!(gateway.mode, None);
        assert!(!gateway.actionable);
        assert_eq!(gateway.unavailable_reason, None);
        assert_eq!(gateway.status, "missing");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn managed_unhealthy_without_spawn_reports_both() {
        let dir = temp_root("both");
        register_for_enabled_pack(&dir, &enabled_pack("awam.file-gateway"), "awam.file-gateway",
            DependencyMode::Managed, "http://127.0.0.1:18787".to_string(), CredentialSource::None,
            "0.1.0".to_string()).unwrap();
        // 手写 Unhealthy：register 默认 Unknown。
        let mut rec: ManagedDependencyRecord = registry::load_record(&dir, "awam.file-gateway").unwrap().unwrap();
        rec.health = HealthState::Unhealthy;
        registry::save_record(&dir, &rec).unwrap();

        let mut items = sample_items("missing");
        enrich_with_registry(&mut items, &dir);
        let gateway = items.iter().find(|i| i.id == "awam_gateway").unwrap();
        assert_eq!(gateway.mode.as_deref(), Some("managed"));
        assert!(gateway.actionable);
        assert_eq!(gateway.status, "missing");
        assert_eq!(gateway.unavailable_reason.as_deref(), Some("both"));
        // 未映射的 rclone 不受影响。
        let rclone = items.iter().find(|i| i.id == "rclone").unwrap();
        assert_eq!(rclone.mode, None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_unhealthy_reports_config_only_and_managed_spawned_reports_managed_only() {
        // config-mode + Unhealthy → configOnly
        let dir = temp_root("configonly");
        register_for_enabled_pack(&dir, &enabled_pack("awam.file-gateway"), "awam.file-gateway",
            DependencyMode::Config, String::new(), CredentialSource::None, "0.1.0".to_string()).unwrap();
        let mut rec = registry::load_record(&dir, "awam.file-gateway").unwrap().unwrap();
        rec.health = HealthState::Unhealthy;
        registry::save_record(&dir, &rec).unwrap();
        let mut items = sample_items("missing");
        enrich_with_registry(&mut items, &dir);
        assert_eq!(items[0].unavailable_reason.as_deref(), Some("configOnly"));
        assert_eq!(items[0].mode.as_deref(), Some("config"));
        let _ = fs::remove_dir_all(&dir);

        // managed-mode + 有自启 pid + Unhealthy → managedOnly
        let dir2 = temp_root("managedonly");
        register_for_enabled_pack(&dir2, &enabled_pack("awam.file-gateway"), "awam.file-gateway",
            DependencyMode::Managed, String::new(), CredentialSource::None, "0.1.0".to_string()).unwrap();
        let mut rec2 = registry::load_record(&dir2, "awam.file-gateway").unwrap().unwrap();
        rec2.health = HealthState::Unhealthy;
        rec2.spawned_pid = Some(42);
        registry::save_record(&dir2, &rec2).unwrap();
        let mut items2 = sample_items("missing");
        enrich_with_registry(&mut items2, &dir2);
        assert_eq!(items2[0].unavailable_reason.as_deref(), Some("managedOnly"));
        let _ = fs::remove_dir_all(&dir2);
    }

    #[test]
    fn healthy_record_promotes_status_to_ok() {
        let dir = temp_root("healthy");
        register_for_enabled_pack(&dir, &enabled_pack("awam.file-gateway"), "awam.file-gateway",
            DependencyMode::Managed, "http://127.0.0.1:18787".to_string(), CredentialSource::None,
            "0.1.0".to_string()).unwrap();
        let mut rec = registry::load_record(&dir, "awam.file-gateway").unwrap().unwrap();
        rec.health = HealthState::Healthy;
        registry::save_record(&dir, &rec).unwrap();
        // 探活原本 missing，注册表 Healthy 权威覆写为 ok。
        let mut items = sample_items("missing");
        enrich_with_registry(&mut items, &dir);
        assert_eq!(items[0].status, "ok");
        assert_eq!(items[0].unavailable_reason, None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dependency_item_serializes_camel_case_optional_fields() {
        let mut item = gateway_item("missing");
        item.mode = Some("managed".to_string());
        item.actionable = true;
        item.unavailable_reason = Some("both".to_string());
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"unavailableReason\":\"both\""), "{json}");
        assert!(json.contains("\"actionable\":true"), "{json}");
        assert!(json.contains("\"mode\":\"managed\""), "{json}");
        // 未投影条目不输出这三个键（前端向后兼容）。
        let bare = serde_json::to_string(&gateway_item("ok")).unwrap();
        assert!(!bare.contains("unavailableReason"), "{bare}");
        assert!(!bare.contains("\"mode\""), "{bare}");
    }
}
