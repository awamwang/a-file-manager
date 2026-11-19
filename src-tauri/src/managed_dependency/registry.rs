// SPDX-License-Identifier: GPL-3.0-or-later
//! `ManagedDependency` 注册表：连接/进程/健康/credential 的权威态（ADR 0020）。
//!
//! - 与 `InstalledPackRecord` 1:1（按 `pack_id`），**只能由 pack 启用/禁用 API 建/销**（Q21），
//!   注册表不开放独立增删；启动时再 `prune_orphans` 兜底删孤儿。
//! - 进程/健康态存于此、不并入 `InstalledPackRecord`（保持解耦）。
//! - 存储：`{app_data}/managed-dependencies/{id}.json`，先写临时再 rename 原子落盘（仿 pack_fs）。
//! - 本模块纯依赖 `&Path`，不触及 Tauri 运行时，便于单测。

use crate::extension_host::types::{ExtensionRuntimeState, InstalledPackRecord};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// 托管依赖解析模式（带 fallback 的偏好，非互斥；见 ADR 0013/0020）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DependencyMode {
    /// 连接远程 / 已有实例、不拉起进程（默认入口）。
    Config,
    /// 由 AFM 拉起 / 守护本机自带隔离产物（R3）。
    Managed,
}

/// 探活/附着得到的健康态（进程权威，与包记录分离）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HealthState {
    /// 尚未探测。
    Unknown,
    /// 可达（config-mode 附着、或 managed-mode 自启后探活通过）。
    Healthy,
    /// 不可达。
    Unhealthy,
}

/// credential 来源（差异点统一承载：tool-manager token 文件/`AWAM_HTTP_TOKEN`、knowledge token、file 无）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CredentialSource {
    /// 无凭据（如 awam-file）。
    None,
    /// 从固定 token 文件读取。
    TokenFile { path: String },
    /// 从指定环境变量读取。
    EnvVar { name: String },
}

impl Default for CredentialSource {
    fn default() -> Self {
        CredentialSource::None
    }
}

/// 一条托管依赖的权威记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedDependencyRecord {
    /// 稳定 id（供 i18n / deps_probe 投影引用），如 `awam.file`。
    pub id: String,
    /// 绑定的 pack id（1:1），与 `InstalledPackRecord.id` 对应。
    pub pack_id: String,
    #[serde(default = "default_mode")]
    pub mode: DependencyMode,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub credential: CredentialSource,
    /// AFM 自己 spawn 且记录 pid 的进程；stop 只作用于此（Q8）。附着 / 未拉起时为 None。
    #[serde(default)]
    pub spawned_pid: Option<u32>,
    #[serde(default = "default_health")]
    pub health: HealthState,
    /// 元清单为本次产物声明的版本，供声明版兼容门（Q15=a / Q19）。
    #[serde(default)]
    pub declared_version: String,
    /// 已落盘产物的版本（provision 或手动升级成功后写入）。与 `available_version` 配合展示「当前 → 可用」。
    #[serde(default)]
    pub installed_version: String,
    /// feed 声明了比磁盘产物更新的版本（ensure 时磁盘 sha ≠ entry sha）→ 置为该版本号；**空=无可用升级**。
    /// 由 ensure / 启动恢复计算，**不静默替换**产物；用户点「升级」触发 `managed_dependency_upgrade` 后清空（ADR 0022）。
    #[serde(default)]
    pub available_version: String,
}

fn default_mode() -> DependencyMode {
    DependencyMode::Config
}

fn default_health() -> HealthState {
    HealthState::Unknown
}

impl Default for ManagedDependencyRecord {
    fn default() -> Self {
        Self {
            id: String::new(),
            pack_id: String::new(),
            mode: DependencyMode::Config,
            base_url: String::new(),
            credential: CredentialSource::None,
            spawned_pid: None,
            health: default_health(),
            declared_version: String::new(),
            installed_version: String::new(),
            available_version: String::new(),
        }
    }
}

/// `{app_data}/managed-dependencies`
pub fn records_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("managed-dependencies")
}

/// `{app_data}/managed-dependencies/{id}.json`
pub fn record_path(app_data_dir: &Path, id: &str) -> PathBuf {
    records_root(app_data_dir).join(format!("{id}.json"))
}

fn err(context: &str, error: &std::io::Error) -> String {
    format!("managed_dependency {context}: {error}")
}

/// 读取单条记录；不存在返回 `Ok(None)`。
pub fn load_record(app_data_dir: &Path, id: &str) -> Result<Option<ManagedDependencyRecord>, String> {
    let path = record_path(app_data_dir, id);
    if !path.exists() {
        return Ok(None);
    }
    let text =
        fs::read_to_string(&path).map_err(|e| err(&format!("read '{id}'"), &e))?;
    let record: ManagedDependencyRecord = serde_json::from_str(&text)
        .map_err(|e| format!("managed_dependency parse '{id}': {e}"))?;
    Ok(Some(record))
}

/// 列出全部记录（按 id 稳定排序）。
pub fn list_records(app_data_dir: &Path) -> Result<Vec<ManagedDependencyRecord>, String> {
    let root = records_root(app_data_dir);
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut records = Vec::new();
    let entries = fs::read_dir(&root).map_err(|e| err("list dir", &e))?;
    for entry in entries {
        let entry = entry.map_err(|e| err("list entry", &e))?;
        let path = entry.path();
        // 跳过临时文件与目录，仅取 *.json。
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let text = fs::read_to_string(&path)
            .map_err(|e| err(&format!("read {:?}", path.file_name()), &e))?;
        let record: ManagedDependencyRecord = serde_json::from_str(&text)
            .map_err(|e| format!("managed_dependency parse {:?}: {e}", path.file_name()))?;
        records.push(record);
    }
    records.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(records)
}

/// 原子写：先写 `{id}.json.tmp` 再 rename 覆盖目标。
pub fn save_record(app_data_dir: &Path, record: &ManagedDependencyRecord) -> Result<(), String> {
    if record.id.is_empty() {
        return Err("managed_dependency save: empty id".to_string());
    }
    let root = records_root(app_data_dir);
    fs::create_dir_all(&root).map_err(|e| err("create dir", &e))?;
    let path = record_path(app_data_dir, &record.id);
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(record)
        .map_err(|e| format!("managed_dependency serialize '{}': {e}", record.id))?;
    fs::write(&tmp, text).map_err(|e| err("write tmp", &e))?;
    fs::rename(&tmp, &path).map_err(|e| err("rename onto record", &e))?;
    Ok(())
}

/// 删除记录文件。
pub fn delete_record(app_data_dir: &Path, id: &str) -> Result<(), String> {
    let path = record_path(app_data_dir, id);
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(&path).map_err(|e| err(&format!("remove '{id}'"), &e))
}

/// pack 启用时建/更新其托管记录（1:1 单写入口）。
///
/// - 拒绝：`pack.state != Enabled`（Q21：未启用不得建 managed 记录）。
/// - 拒绝：已有**另一条**记录占用同一 `pack_id`（保证 1:1）。
pub fn register_for_enabled_pack(
    app_data_dir: &Path,
    pack: &InstalledPackRecord,
    id: &str,
    mode: DependencyMode,
    base_url: String,
    credential: CredentialSource,
    declared_version: String,
) -> Result<ManagedDependencyRecord, String> {
    if pack.state != ExtensionRuntimeState::Enabled {
        return Err(format!(
            "managed_dependency register: pack '{}' not enabled",
            pack.id
        ));
    }
    for existing in list_records(app_data_dir)? {
        if existing.pack_id == pack.id && existing.id != id {
            return Err(format!(
                "managed_dependency register: pack '{}' already bound to '{}'",
                pack.id, existing.id
            ));
        }
    }
    let record = ManagedDependencyRecord {
        id: id.to_string(),
        pack_id: pack.id.clone(),
        mode,
        base_url,
        credential,
        spawned_pid: None,
        health: default_health(),
        declared_version,
        installed_version: String::new(),
        available_version: String::new(),
    };
    save_record(app_data_dir, &record)?;
    Ok(record)
}

/// pack 禁用/卸载时销其记录（1:1 单写出口）。返回是否删除了记录。
pub fn unregister_for_pack(app_data_dir: &Path, pack_id: &str) -> Result<bool, String> {
    let mut removed = false;
    for record in list_records(app_data_dir)? {
        if record.pack_id == pack_id {
            delete_record(app_data_dir, &record.id)?;
            removed = true;
        }
    }
    Ok(removed)
}

/// 启动兜底：删除 `pack_id` 不在有效集合内的孤儿记录，返回被删 id 列表。
pub fn prune_orphans(
    app_data_dir: &Path,
    valid_pack_ids: &BTreeSet<String>,
) -> Result<Vec<String>, String> {
    let mut pruned = Vec::new();
    for record in list_records(app_data_dir)? {
        if !valid_pack_ids.contains(&record.pack_id) {
            delete_record(app_data_dir, &record.id)?;
            pruned.push(record.id);
        }
    }
    Ok(pruned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-managed-dep-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn pack(state: ExtensionRuntimeState) -> InstalledPackRecord {
        InstalledPackRecord {
            id: "awam.file-gateway".to_string(),
            name: "awam-file".to_string(),
            version: "0.1.0".to_string(),
            state,
            permissions: vec![crate::extension_host::types::ExtensionPermission::SpawnSidecar],
            install_path: "/tmp/pack".to_string(),
            last_error: None,
        }
    }

    #[test]
    fn record_round_trip_and_path() {
        let dir = temp_root("roundtrip");
        let mut record = ManagedDependencyRecord::default();
        record.id = "awam.file".to_string();
        record.pack_id = "awam.file-gateway".to_string();
        record.mode = DependencyMode::Managed;
        record.base_url = "http://127.0.0.1:18787".to_string();
        record.credential = CredentialSource::None;
        record.declared_version = "0.4.2".to_string();
        save_record(&dir, &record).unwrap();

        let loaded = load_record(&dir, "awam.file").unwrap().unwrap();
        assert_eq!(loaded, record);
        assert_eq!(record_path(&dir, "awam.file"), dir.join("managed-dependencies").join("awam.file.json"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn credential_json_is_internally_tagged_camel_case() {
        let dir = temp_root("cred");
        let mut record = ManagedDependencyRecord {
            id: "awam.tools".to_string(),
            pack_id: "awam.tool-runtime".to_string(),
            ..Default::default()
        };
        record.credential = CredentialSource::TokenFile {
            path: "C:/tokens/http-token".to_string(),
        };
        save_record(&dir, &record).unwrap();
        let text = fs::read_to_string(record_path(&dir, "awam.tools")).unwrap();
        assert!(text.contains("\"baseUrl\""), "{text}");
        assert!(text.contains("\"packId\""), "{text}");
        assert!(text.contains("\"spawnedPid\""), "{text}");
        assert!(text.contains("\"kind\":\"tokenFile\"") || text.contains("\"kind\": \"tokenFile\""), "{text}");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn register_requires_enabled_pack() {
        let dir = temp_root("enable");
        let disabled = pack(ExtensionRuntimeState::Disabled);
        let result = register_for_enabled_pack(
            &dir,
            &disabled,
            "awam.file",
            DependencyMode::Managed,
            "http://127.0.0.1:18787".to_string(),
            CredentialSource::None,
            "0.1.0".to_string(),
        );
        assert!(result.is_err(), "must reject non-enabled pack");
        assert!(result.unwrap_err().contains("not enabled"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn register_enforces_one_to_one_pack_binding() {
        let dir = temp_root("oneone");
        let enabled = pack(ExtensionRuntimeState::Enabled);
        register_for_enabled_pack(
            &dir,
            &enabled,
            "awam.file",
            DependencyMode::Config,
            String::new(),
            CredentialSource::None,
            "0.1.0".to_string(),
        )
        .unwrap();
        // 另一条记录想占用同一 pack_id → 拒绝。
        let conflict = register_for_enabled_pack(
            &dir,
            &enabled,
            "awam.file.alt",
            DependencyMode::Config,
            String::new(),
            CredentialSource::None,
            "0.1.0".to_string(),
        );
        assert!(conflict.is_err(), "1:1 must reject second record on same pack");
        // 同 id 覆盖更新 → 允许。
        let again = register_for_enabled_pack(
            &dir,
            &enabled,
            "awam.file",
            DependencyMode::Managed,
            String::new(),
            CredentialSource::None,
            "0.2.0".to_string(),
        );
        assert!(again.is_ok());
        assert_eq!(load_record(&dir, "awam.file").unwrap().unwrap().declared_version, "0.2.0");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unregister_and_prune_orphans() {
        let dir = temp_root("prune");
        let enabled = pack(ExtensionRuntimeState::Enabled);
        register_for_enabled_pack(
            &dir,
            &enabled,
            "awam.file",
            DependencyMode::Config,
            String::new(),
            CredentialSource::None,
            "0.1.0".to_string(),
        )
        .unwrap();

        // 卸载该 pack 的记录应被销。
        assert!(unregister_for_pack(&dir, "awam.file-gateway").unwrap());
        assert!(load_record(&dir, "awam.file").unwrap().is_none());

        // 重建后，prune 删除不在有效集合的孤儿。
        register_for_enabled_pack(
            &dir,
            &enabled,
            "awam.file",
            DependencyMode::Config,
            String::new(),
            CredentialSource::None,
            "0.1.0".to_string(),
        )
        .unwrap();
        let mut valid = BTreeSet::new();
        valid.insert("awam.file-gateway".to_string());
        assert!(prune_orphans(&dir, &valid).unwrap().is_empty());

        valid.remove("awam.file-gateway");
        let pruned = prune_orphans(&dir, &valid).unwrap();
        assert_eq!(pruned, vec!["awam.file".to_string()]);
        assert_eq!(list_records(&dir).unwrap().len(), 0);

        let _ = fs::remove_dir_all(&dir);
    }
}
