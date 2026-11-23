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
        "awam.knowledge" => Some("knowledge_service"),
        "awam.tool-runtime" => Some("tool_gateway"),
        _ => None,
    }
}

/// 健康态 → deps_probe status 字符串（注入卡片初始态用）。Unknown 视作「启动中」（尚未探活）。
fn status_from_health(health: HealthState) -> &'static str {
    match health {
        HealthState::Healthy => "ok",
        HealthState::Starting => "starting",
        HealthState::Unhealthy => "missing",
        HealthState::Unknown => "starting",
    }
}

/// 为「已启用但不在静态探测列表」的托管服务注入一张健康页卡片：kind=optional，初始 status 取
/// 注册表健康态（含 Starting 过渡态），mode/actionable/unavailableReason 随后由投影覆写。
/// title/detail/fix 为后端兜底文案（前端 i18n 覆盖 title）。
fn injected_item(item_id: &str, record: &registry::ManagedDependencyRecord) -> DependencyItem {
    let (title, detail, fix) = match item_id {
        "knowledge_service" => (
            "Knowledge service",
            "本地知识库内容检索服务（awam-knowledge）。",
            "可在下方启动/停止；或到「扩展 → 服务配置」检查地址与口令。",
        ),
        "tool_gateway" => (
            "Tool gateway",
            "本机工具管理网关（awam-tool-manager）。",
            "可在下方启动/停止；或到「扩展 → 服务配置」检查地址。",
        ),
        _ => (item_id, "", ""),
    };
    DependencyItem {
        id: item_id.to_string(),
        kind: "optional".to_string(),
        status: status_from_health(record.health).to_string(),
        title: title.to_string(),
        detail: detail.to_string(),
        fix: fix.to_string(),
        mode: None,
        actionable: false,
        unavailable_reason: None,
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

/// 就地投影：对映射到注册表记录的条目覆写 mode / actionable / status / unavailableReason；
/// 已启用但不在静态列表中的托管服务则注入新卡片（故需 `&mut Vec`）。
pub fn enrich_with_registry(items: &mut Vec<DependencyItem>, app_data: &Path) {
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
        if !items.iter().any(|item| item.id == item_id) {
            // 已启用（有托管记录）但不在静态探测列表 → 注入卡片，使其同样享受 managed 权威状态
            // 与 Starting 过渡态。未启用的 pack 无记录，故不会注入、不会误报。
            items.push(injected_item(item_id, &record));
        }
        let Some(item) = items.iter_mut().find(|item| item.id == item_id) else {
            continue;
        };
        item.mode = Some(mode_str(record.mode).to_string());
        item.actionable = true;
        // 注册表为权威：health 已知时覆写探活结论；Unknown（尚未探测）保留 probe 结果。
        // Starting（预热过渡态）投影为「启动中」，不落入「不可用」。
        match record.health {
            HealthState::Healthy => item.status = "ok".to_string(),
            HealthState::Starting => item.status = "starting".to_string(),
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
    fn starting_record_projects_starting_status_not_missing() {
        let dir = temp_root("starting");
        register_for_enabled_pack(&dir, &enabled_pack("awam.file-gateway"), "awam.file-gateway",
            DependencyMode::Managed, "http://127.0.0.1:18787".to_string(), CredentialSource::None,
            "0.1.0".to_string()).unwrap();
        let mut rec = registry::load_record(&dir, "awam.file-gateway").unwrap().unwrap();
        rec.health = HealthState::Starting;
        rec.spawned_pid = Some(9);
        registry::save_record(&dir, &rec).unwrap();
        // 探活原本 missing，注册表 Starting 权威覆写为 starting，且不归因为不可用。
        let mut items = sample_items("missing");
        enrich_with_registry(&mut items, &dir);
        assert_eq!(items[0].status, "starting");
        assert_eq!(items[0].unavailable_reason, None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn managed_record_without_probe_item_injects_card_with_transition_state() {
        let dir = temp_root("inject");
        register_for_enabled_pack(&dir, &enabled_pack("awam.knowledge"), "awam.knowledge",
            DependencyMode::Managed, "http://127.0.0.1:18790".to_string(), CredentialSource::None,
            "0.1.0".to_string()).unwrap();
        let mut rec = registry::load_record(&dir, "awam.knowledge").unwrap().unwrap();
        rec.health = HealthState::Starting;
        rec.spawned_pid = Some(11);
        registry::save_record(&dir, &rec).unwrap();
        // 静态探测列表里没有 knowledge_service 条目。
        let mut items = sample_items("ok");
        assert!(items.iter().all(|i| i.id != "knowledge_service"));
        enrich_with_registry(&mut items, &dir);
        let injected = items
            .iter()
            .find(|i| i.id == "knowledge_service")
            .expect("managed record without a probe item should inject a card");
        assert_eq!(injected.status, "starting");
        assert_eq!(injected.mode.as_deref(), Some("managed"));
        assert!(injected.actionable);
        assert_eq!(injected.unavailable_reason, None);
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
