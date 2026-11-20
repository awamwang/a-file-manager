// SPDX-License-Identifier: GPL-3.0-or-later
//! 托管依赖 IPC 命令（Task 5 后端接线）。逻辑内核（`ensure_record_with` / `stop_record_with`）
//! 纯 `&Path` + 注入闭包，离线单测；Tauri 包装仅做目录解析与 spawn_blocking。

use super::feed;
use super::orchestrator::{
    attach_or_spawn, find_pack_by_id, pick_sidecar_for_pack, stop_self_spawned,
};
use super::provision;
use super::registry::{self, DependencyMode, HealthState, ManagedDependencyRecord};
use super::store;
use crate::extension_host::types::SidecarContribution;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// 端口被占时，从默认端口起最多顺延尝试的跨度（ADR 0022 端口回退）。
const MANAGED_PORT_SPAN: u16 = 32;
/// awam-file 网关 pack id（managed 客户端同步用）。
const FILE_GATEWAY_PACK_ID: &str = "awam.file-gateway";
/// awam-knowledge pack id（managed 客户端同步用）。
const KNOWLEDGE_PACK_ID: &str = "awam.knowledge";

/// 取当前宿主平台的 feed 产物条目。所有真正下载/落盘/探活的路径都必须走这里，
/// 避免多平台 feed 下 macOS 选中 Windows `.exe`（ADR 0022 macOS packaging）。
fn find_host_entry<'a>(
    manifest: &'a store::StoreManifest,
    pack_id: &str,
) -> Option<&'a store::StoreEntry> {
    store::find_entry_for_platform(manifest, pack_id, &crate::platform::artifact_platform_key())
}

// ---- 可测内核 ----

/// attach-or-spawn 并把结果回写记录。
///
/// - `base_url` 未配置 → `Err`（不 spawn）。
/// - 可达 → 附着：保留既有 `spawned_pid`（不清），`health=Healthy`。
/// - 不可达 → 拉起：`spawned_pid=Some(新 pid)`，`health=Unhealthy`（真实健康由下一次探活/list 刷新）。
pub(crate) fn ensure_record_with<Payload, P, S>(
    record: &mut ManagedDependencyRecord,
    pack_install_dir: &Path,
    contrib: &SidecarContribution,
    probe: P,
    spawn: S,
) -> Result<(), String>
where
    P: Fn(&str) -> Result<Payload, String>,
    S: FnOnce(&Path, &SidecarContribution) -> Result<u32, String>,
{
    let base_url = record.base_url.trim();
    if base_url.is_empty() {
        return Err("managed_dependency ensure: base_url not configured".to_string());
    }
    let outcome = attach_or_spawn(
        base_url,
        record.spawned_pid,
        pack_install_dir,
        contrib,
        probe,
        spawn,
    )?;
    if outcome.spawned {
        record.spawned_pid = Some(outcome.pid);
        record.health = HealthState::Unhealthy;
    } else {
        record.health = HealthState::Healthy;
    }
    Ok(())
}

/// 停止并回写记录：只 kill 自启 pid；成功后清 pid、置 Unhealthy。非自启 → `Err` 拒绝（Q8）。
pub(crate) fn stop_record_with<K>(
    record: &mut ManagedDependencyRecord,
    kill: K,
) -> Result<(), String>
where
    K: FnOnce(u32),
{
    stop_self_spawned(record.spawned_pid, kill)?;
    record.spawned_pid = None;
    record.health = HealthState::Unhealthy;
    Ok(())
}

/// 解析 sidecar env 值里的 `${afm:settings:<key>}` 占位（从 AFM 设置读，密钥不写进 pack.json、
/// 不进 record/磁盘、不进日志）。非占位值原样返回。`lookup` 注入便于离线单测。
pub(crate) fn resolve_env_value<F>(raw: &str, mut lookup: F) -> String
where
    F: FnMut(&str) -> String,
{
    const PREFIX: &str = "${afm:settings:";
    if let Some(rest) = raw.strip_prefix(PREFIX) {
        if let Some(key) = rest.strip_suffix('}') {
            return lookup(key.trim());
        }
    }
    raw.to_string()
}

/// 就地把 contrib.env 中的占位用 AFM 设置解析。仅在有 env 时访问设置。
fn expand_contrib_env(contrib: &mut SidecarContribution, app: &AppHandle) {
    if contrib.env.is_empty() {
        return;
    }
    let env: &mut BTreeMap<String, String> = &mut contrib.env;
    for value in env.values_mut() {
        *value = resolve_env_value(value, |key| crate::settings::read_string_setting(app, key));
    }
}

/// ensure 的公共前置：按 `record.pack_id` 解析启用 pack + 其 sidecar 贡献（1:1 校验）。
fn resolve_pack_sidecar(
    record: &ManagedDependencyRecord,
) -> Result<(PathBuf, SidecarContribution), String> {
    let pack = find_pack_by_id(&record.pack_id).ok_or_else(|| {
        format!(
            "managed_dependency ensure: pack '{}' not installed",
            record.pack_id
        )
    })?;
    if pack.state != crate::extension_host::types::ExtensionRuntimeState::Enabled {
        return Err(format!(
            "managed_dependency ensure: pack '{}' not enabled",
            record.pack_id
        ));
    }
    let contrib = pick_sidecar_for_pack(&pack).ok_or_else(|| {
        format!(
            "managed_dependency ensure: pack '{}' declares no sidecar",
            record.pack_id
        )
    })?;
    Ok((PathBuf::from(&pack.install_path), contrib))
}

/// managed 拉起前准备 sidecar 贡献（ADR 0022 接线）。返回 `Ok(None)` 表示「managed 但产物暂不可得 →
/// 跳过 spawn」（feed 不可达 / 无 entry 且盘上无产物），避免 spawn 一个缺失的相对 exe 反复报错。
/// - 非 `Managed` → `Ok(Some(contrib))` 原样返回。
/// - `Managed` → 读 feed entry：命中则 [`provision::ensure_artifact`]（缺则下载校验；已在则纯读 sha 比对，
///   **非静默**——sha 不符只置 `available_version`，不替换）；command 改写为盘上绝对路径。
///   feed 不可达 / 无 entry → 回退盘上期望路径：存在则用，否则 `Ok(None)` 跳过。
fn prepare_contrib_for_spawn(
    app_data: &Path,
    record: &mut ManagedDependencyRecord,
    pack_install_dir: &Path,
    mut contrib: SidecarContribution,
) -> Result<Option<SidecarContribution>, String> {
    if record.mode != DependencyMode::Managed {
        return Ok(Some(contrib));
    }
    let feed_cfg = feed::load_feed(app_data);
    let manifest = match feed::fetch_manifest(&feed_cfg, store::fetch_manifest_text) {
        Ok(m) => m,
        Err(_) => return Ok(resolve_offline_or_skip(pack_install_dir, contrib)),
    };
    let entry = manifest
        .as_ref()
        .and_then(|m| find_host_entry(m, &record.pack_id));
    match provision::ensure_artifact(
        entry,
        &feed_cfg.allow_hosts,
        pack_install_dir,
        contrib.working_dir_rel.as_deref(),
        store::http_get_bytes,
    )? {
        Some(state) => {
            contrib.command = state.path.to_string_lossy().into_owned();
            // 升级标记（仅当 feed 声明了本 pack 的产物 entry）。
            if let Some(entry) = entry {
                if state.up_to_date {
                    record.installed_version = entry.artifact_version.clone();
                    record.available_version = String::new();
                } else {
                    // 磁盘产物落后于 feed：不替换，仅标记可用升级版本供前端提示。
                    record.available_version = entry.artifact_version.clone();
                }
            }
            Ok(Some(contrib))
        }
        // 无 entry（feed 未列该 pack）：产物应随 pack 内置；盘上没有则跳过 spawn。
        None => Ok(resolve_offline_or_skip(pack_install_dir, contrib)),
    }
}

/// feed 不可达 / 无 entry 时的兜底：把 `contrib.command` 解析到 sidecar 期望绝对路径，存在则用，
/// 不存在则 `None`（调用方跳过 spawn）。
fn resolve_offline_or_skip(
    pack_install_dir: &Path,
    mut contrib: SidecarContribution,
) -> Option<SidecarContribution> {
    let rel = contrib
        .working_dir_rel
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(provision::DEFAULT_SIDECAR_DIR);
    let name = Path::new(&contrib.command)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| contrib.command.clone());
    let expected = pack_install_dir.join(rel).join(&name);
    if expected.exists() {
        contrib.command = expected.to_string_lossy().into_owned();
        Some(contrib)
    } else {
        None
    }
}

// ---- Tauri 命令 ----

/// 通用探活（`GET {base}/health`，免鉴权约定与三个依赖一致）。
fn probe(url: &str) -> Result<crate::tool_runtime::HealthOk, String> {
    crate::tool_runtime::health_check(url)
}

/// 生产 spawn：复用 tool_runtime 的进程拉起实现（CREATE_NO_WINDOW、workingDirRel 解析）。
fn native_spawn(pack_install_dir: &Path, contrib: &SidecarContribution) -> Result<u32, String> {
    crate::tool_runtime::spawn_sidecar_process(pack_install_dir, contrib)
}

/// 托管 spawn 前的端口回退（ADR 0022）。仅对 `Managed` 且 pack 声明了 `port_env` 的 contrib 生效：
/// - 目标端口若探活健康 → 视为附着，**绝不改端口**（避免误伤用户自跑的同一服务）。
/// - 否则若被**非本服务**占用 → 顺延到一个可绑端口并写回 `record.base_url`。
/// - 无论是否顺延，都把最终端口注入 `contrib.env[port_env]`，使 managed 记录的端口成为子进程的实际监听端口。
pub(crate) fn apply_managed_port(
    record: &mut ManagedDependencyRecord,
    contrib: &mut SidecarContribution,
) {
    if record.mode != DependencyMode::Managed {
        return;
    }
    let Some(var) = contrib.port_env.clone() else {
        return;
    };
    if probe(&record.base_url).is_err() {
        record.base_url =
            super::net_port::relocate_if_occupied(&record.base_url, MANAGED_PORT_SPAN);
    }
    if let Some(port) = super::net_port::port_of(&record.base_url) {
        contrib.env.insert(var, port.to_string());
    }
}

/// managed 权威同步：把生效 URL 推入对应后端的客户端解析全局，使换端口后 AFM 客户端连到实际端口。
/// file→`providers::gateway`、knowledge→`knowledge_service`；tool-runtime 客户端本就读 record 无需推。
/// config-mode 不推（由设置/默认主导）。仅 `Managed` 且 `base_url` 非空时动作。
pub(crate) fn sync_managed_client_url(record: &ManagedDependencyRecord) {
    if record.mode != DependencyMode::Managed || record.base_url.trim().is_empty() {
        return;
    }
    match record.pack_id.as_str() {
        FILE_GATEWAY_PACK_ID => {
            crate::providers::gateway::set_configured_gateway_url(Some(&record.base_url))
        }
        KNOWLEDGE_PACK_ID => crate::knowledge_service::set_effective_base_url(&record.base_url),
        _ => {}
    }
}

fn app_config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|err| format!("managed_dependency: app config dir: {err}"))
}

/// 列出全部托管依赖记录（权威态；投影给 Health/deps_probe）。
#[tauri::command]
pub async fn managed_dependency_list(
    app: AppHandle,
) -> Result<Vec<ManagedDependencyRecord>, String> {
    let dir = app_config_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || registry::list_records(&dir))
        .await
        .map_err(|err| format!("managed_dependency_list worker failed: {err}"))?
}

/// 设置 managed 产物 feed（AFM 自有发布通道）的元清单 URL、可选对象存储回落 URL 与 allow-list（ADR 0022）。
/// 写入后，下一次 pack 启用即据此把有 entry 的 pack 切 Managed、ensure 据此 provision 产物。
#[tauri::command]
pub async fn managed_dependency_set_feed(
    app: AppHandle,
    manifest_url: String,
    fallback_manifest_url: Option<String>,
    allow_hosts: Vec<String>,
) -> Result<feed::FeedConfig, String> {
    let dir = app_config_dir(&app)?;
    let cfg = feed::FeedConfig {
        manifest_url,
        fallback_manifest_url: fallback_manifest_url.unwrap_or_default(),
        allow_hosts,
    };
    feed::save_feed(&dir, &cfg)?;
    Ok(cfg)
}

/// 读取当前 managed 产物 feed 配置。
#[tauri::command]
pub async fn managed_dependency_get_feed(app: AppHandle) -> Result<feed::FeedConfig, String> {
    let dir = app_config_dir(&app)?;
    Ok(feed::load_feed(&dir))
}

/// 停止指定托管依赖（仅 AFM 自启 pid；附着/远程实例一律拒绝）。
#[tauri::command]
pub async fn managed_dependency_stop(
    app: AppHandle,
    id: String,
) -> Result<ManagedDependencyRecord, String> {
    let dir = app_config_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut record = registry::load_record(&dir, &id)?
            .ok_or_else(|| format!("managed_dependency stop: unknown id '{id}'"))?;
        stop_record_with(&mut record, |pid| {
            crate::win_process::kill_process_tree(pid)
        })?;
        registry::save_record(&dir, &record)?;
        Ok(record)
    })
    .await
    .map_err(|err| format!("managed_dependency_stop worker failed: {err}"))?
}

/// 确保指定托管依赖可用：先探 config-mode，健康附着；否则按 pack sidecar 贡献拉起。
#[tauri::command]
pub async fn managed_dependency_ensure(
    app: AppHandle,
    id: String,
) -> Result<ManagedDependencyRecord, String> {
    let dir = app_config_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut record = registry::load_record(&dir, &id)?
            .ok_or_else(|| format!("managed_dependency ensure: unknown id '{id}'"))?;
        // config-mode 不自动拉起（由工具页手动附着/连接）；仅托管记录走 provision+spawn。
        if record.mode != DependencyMode::Managed {
            return Ok(record);
        }
        let (pack_install_dir, contrib) = resolve_pack_sidecar(&record)?;
        let Some(mut contrib) =
            prepare_contrib_for_spawn(&dir, &mut record, &pack_install_dir, contrib)?
        else {
            // 产物暂不可得（feed 不可达且盘上无产物）：不 spawn，落库后返回（下次启动/检查/手动再试）。
            registry::save_record(&dir, &record)?;
            return Ok(record);
        };
        expand_contrib_env(&mut contrib, &app);
        apply_managed_port(&mut record, &mut contrib);
        ensure_record_with(
            &mut record,
            &pack_install_dir,
            &contrib,
            probe,
            native_spawn,
        )?;
        registry::save_record(&dir, &record)?;
        sync_managed_client_url(&record);
        Ok(record)
    })
    .await
    .map_err(|err| format!("managed_dependency_ensure worker failed: {err}"))?
}

/// 手动升级托管产物（ADR 0022「非静默升级」）。仅当 `record.available_version` 非空（ensure 检出磁盘
/// 产物落后于 feed）时由用户点击触发。流程：**停旧进程 → 从 feed 强制重下校验替换产物 → 重新拉起**。
/// 停旧是必需：Windows 上运行中的 exe 无法被原子 rename 覆盖，先 `stop_self_spawned` 释放文件锁。
#[tauri::command]
pub async fn managed_dependency_upgrade(
    app: AppHandle,
    id: String,
) -> Result<ManagedDependencyRecord, String> {
    let dir = app_config_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut record = registry::load_record(&dir, &id)?
            .ok_or_else(|| format!("managed_dependency upgrade: unknown id '{id}'"))?;
        if record.mode != DependencyMode::Managed {
            return Err(format!("managed_dependency upgrade: '{id}' is not managed"));
        }
        let (pack_install_dir, contrib) = resolve_pack_sidecar(&record)?;
        let feed_cfg = feed::load_feed(&dir);
        let manifest = feed::fetch_manifest(&feed_cfg, store::fetch_manifest_text)?
            .ok_or_else(|| "managed_dependency upgrade: feed not configured".to_string())?;
        let entry = find_host_entry(&manifest, &record.pack_id)
            .ok_or_else(|| {
                format!(
                    "managed_dependency upgrade: no feed entry for '{}' on {}",
                    record.pack_id,
                    crate::platform::artifact_platform_key()
                )
            })?
            .clone();
        // 1. 停旧（仅自启 pid）；附着/未拉起时 pid 为 None，跳过。
        if record.spawned_pid.is_some() {
            stop_record_with(&mut record, |pid| {
                crate::win_process::kill_process_tree(pid)
            })?;
        }
        // 2. 强制重下 + 校验 + 原子替换落盘。
        let path = provision::force_provision(
            &entry,
            &feed_cfg.allow_hosts,
            &pack_install_dir,
            contrib.working_dir_rel.as_deref(),
            store::http_get_bytes,
        )?;
        // 3. 更新版本标记（升级完成 → 无可用升级），再用新产物重新拉起。
        record.installed_version = entry.artifact_version.clone();
        record.available_version = String::new();
        let mut contrib = contrib;
        contrib.command = path.to_string_lossy().into_owned();
        expand_contrib_env(&mut contrib, &app);
        apply_managed_port(&mut record, &mut contrib);
        ensure_record_with(
            &mut record,
            &pack_install_dir,
            &contrib,
            probe,
            native_spawn,
        )?;
        registry::save_record(&dir, &record)?;
        sync_managed_client_url(&record);
        Ok(record)
    })
    .await
    .map_err(|err| format!("managed_dependency_upgrade worker failed: {err}"))?
}

/// 据 feed entry + 离线产物状态刷新记录的升级标记（纯读结果落地，不下载）。
/// up_to_date → 记 `installed_version`、清 `available_version`；否则（落后或缺产物）置
/// `available_version` 为 feed 声明版本作为升级目标（[`managed_dependency_upgrade`] 覆盖式安装对两种情况都成立）。
fn apply_feed_check(
    record: &mut ManagedDependencyRecord,
    entry: &store::StoreEntry,
    status: provision::OfflineArtifactStatus,
) {
    if status.up_to_date {
        record.installed_version = entry.artifact_version.clone();
        record.available_version = String::new();
    } else {
        record.available_version = entry.artifact_version.clone();
    }
}

/// 纯读「检查所有托管记录是否有可用更新」：清单**只拉一次**，逐条 `inspect_artifact_offline` 比对
/// 盘上产物 sha，更新并落库 `available_version`/`installed_version`。**不下载、不 spawn、不改进程**。
/// 缺产物也记为可用升级（升级动作即覆盖式安装）。feed 不可达 → `Err`（调用方决定提示或忽略）；
/// 单条 pack 未启用 / 无 sidecar / 无 entry → 跳过该条。启动恢复与手动「检查更新」共用。
pub fn check_updates(app_data: &Path) -> Result<(), String> {
    let managed: Vec<ManagedDependencyRecord> = registry::list_records(app_data)?
        .into_iter()
        .filter(|r| r.mode == DependencyMode::Managed)
        .collect();
    if managed.is_empty() {
        return Ok(());
    }
    let feed_cfg = feed::load_feed(app_data);
    let manifest = match feed::fetch_manifest(&feed_cfg, store::fetch_manifest_text)? {
        Some(m) => m,
        None => return Ok(()), // 未配置 feed：无从检查。
    };
    for mut record in managed {
        let Ok((pack_install_dir, contrib)) = resolve_pack_sidecar(&record) else {
            continue;
        };
        let Some(entry) = find_host_entry(&manifest, &record.pack_id) else {
            continue;
        };
        let status = provision::inspect_artifact_offline(
            entry,
            &pack_install_dir,
            contrib.working_dir_rel.as_deref(),
        );
        apply_feed_check(&mut record, entry, status);
        // 单条落库失败不影响其它记录（尽力而为）。
        let _ = registry::save_record(app_data, &record);
    }
    Ok(())
}

/// 检查单个托管依赖是否有可用更新（纯读：拉 feed + 哈希盘上产物，不下载、不 spawn）。返回更新后的记录。
#[tauri::command]
pub async fn managed_dependency_check_update(
    app: AppHandle,
    id: String,
) -> Result<ManagedDependencyRecord, String> {
    let dir = app_config_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut record = registry::load_record(&dir, &id)?
            .ok_or_else(|| format!("managed_dependency check: unknown id '{id}'"))?;
        if record.mode != DependencyMode::Managed {
            return Ok(record);
        }
        let (pack_install_dir, contrib) = resolve_pack_sidecar(&record)?;
        let feed_cfg = feed::load_feed(&dir);
        let manifest = feed::fetch_manifest(&feed_cfg, store::fetch_manifest_text)?;
        if let Some(entry) = manifest
            .as_ref()
            .and_then(|m| find_host_entry(m, &record.pack_id))
        {
            let status = provision::inspect_artifact_offline(
                entry,
                &pack_install_dir,
                contrib.working_dir_rel.as_deref(),
            );
            apply_feed_check(&mut record, entry, status);
            registry::save_record(&dir, &record)?;
        }
        Ok(record)
    })
    .await
    .map_err(|err| format!("managed_dependency_check_update worker failed: {err}"))?
}

/// 检查全部托管记录的可用更新（[`check_updates`]），返回检查后的最新记录列表（供前端刷新标记）。
#[tauri::command]
pub async fn managed_dependency_check_all_updates(
    app: AppHandle,
) -> Result<Vec<ManagedDependencyRecord>, String> {
    let dir = app_config_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        check_updates(&dir)?;
        registry::list_records(&dir)
    })
    .await
    .map_err(|err| format!("managed_dependency_check_all_updates worker failed: {err}"))?
}

/// 启动时恢复托管服务（ADR 0022 / 选项 A）：对所有 `Managed` 记录逐个 ensure
/// （probe→附着，或 provision 产物+spawn）。已运行的服务会被附着而非重复拉起。
/// 尽力而为——单条失败只记录、不影响其它、不阻断启动。应在 spawn_blocking 后台线程调用。
pub fn reconcile_managed_on_startup(app_data: &Path, app: &AppHandle) {
    let records = match registry::list_records(app_data) {
        Ok(records) => records,
        Err(err) => {
            eprintln!("managed startup reconcile: list failed: {err}");
            return;
        }
    };
    for mut record in records {
        if record.mode != DependencyMode::Managed {
            continue;
        }
        let outcome = (|| -> Result<(), String> {
            let (pack_install_dir, contrib) = resolve_pack_sidecar(&record)?;
            let Some(mut contrib) =
                prepare_contrib_for_spawn(app_data, &mut record, &pack_install_dir, contrib)?
            else {
                // 产物暂不可得（feed 不可达且盘上无产物）：跳过 spawn，不算错误、不刷日志。
                return registry::save_record(app_data, &record);
            };
            expand_contrib_env(&mut contrib, app);
            apply_managed_port(&mut record, &mut contrib);
            ensure_record_with(
                &mut record,
                &pack_install_dir,
                &contrib,
                probe,
                native_spawn,
            )?;
            registry::save_record(app_data, &record)?;
            sync_managed_client_url(&record);
            Ok(())
        })();
        if let Err(err) = outcome {
            eprintln!("managed startup reconcile '{}': {err}", record.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn record(base_url: &str) -> ManagedDependencyRecord {
        ManagedDependencyRecord {
            id: "awam.file-gateway".to_string(),
            pack_id: "awam.file-gateway".to_string(),
            base_url: base_url.to_string(),
            ..Default::default()
        }
    }

    fn contrib() -> SidecarContribution {
        SidecarContribution {
            id: "awam-file".to_string(),
            command: "tool.exe".to_string(),
            args: vec!["http".to_string()],
            working_dir_rel: None,
            runtime: Default::default(),
            env: Default::default(),
            port_env: None,
        }
    }

    #[test]
    fn ensure_requires_configured_base_url() {
        let mut rec = record("");
        let called = Cell::new(false);
        let err = ensure_record_with(
            &mut rec,
            Path::new("/tmp"),
            &contrib(),
            |_| Ok(()),
            |_d, _c| {
                called.set(true);
                Ok(1)
            },
        )
        .unwrap_err();
        assert!(err.contains("base_url not configured"), "{err}");
        assert!(!called.get());
    }

    #[test]
    fn ensure_attach_keeps_existing_pid_and_sets_healthy() {
        let mut rec = record("http://127.0.0.1:18787");
        rec.spawned_pid = Some(7);
        let spawned = Cell::new(false);
        ensure_record_with(
            &mut rec,
            Path::new("/tmp"),
            &contrib(),
            |_| Ok(()),
            |_d, _c| {
                spawned.set(true);
                Ok(99)
            },
        )
        .unwrap();
        assert!(!spawned.get());
        assert_eq!(rec.spawned_pid, Some(7), "attach must not clobber pid");
        assert_eq!(rec.health, HealthState::Healthy);
    }

    #[test]
    fn ensure_spawn_records_new_pid() {
        let mut rec = record("http://127.0.0.1:18787");
        ensure_record_with(
            &mut rec,
            Path::new("/tmp"),
            &contrib(),
            |_| Err::<(), _>("down".to_string()),
            |_d, _c| Ok(42),
        )
        .unwrap();
        assert_eq!(rec.spawned_pid, Some(42));
        assert_eq!(rec.health, HealthState::Unhealthy);
    }

    #[test]
    fn stop_clears_pid_and_refuses_non_self_spawned() {
        let mut rec = record("http://x");
        rec.spawned_pid = Some(5);
        let killed = Cell::new(0u32);
        stop_record_with(&mut rec, |p| killed.set(p)).unwrap();
        assert_eq!(killed.get(), 5);
        assert_eq!(rec.spawned_pid, None);
        assert_eq!(rec.health, HealthState::Unhealthy);

        let mut not_spawned = record("http://x");
        assert!(stop_record_with(&mut not_spawned, |_| panic!("must not kill")).is_err());
    }

    fn entry(version: &str) -> store::StoreEntry {
        store::StoreEntry {
            pack_id: "awam.tool-runtime".to_string(),
            artifact_version: version.to_string(),
            asset_url: "http://127.0.0.1:8078/artifacts/awam-tool-manager.exe".to_string(),
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            file_name: Some("awam-tool-manager.exe".to_string()),
            base_url: "http://127.0.0.1:18778".to_string(),
            asset_fallback_url: None,
            platform: None,
        }
    }

    #[test]
    fn apply_feed_check_current_clears_available_and_records_installed() {
        let mut rec = record("http://x");
        rec.mode = DependencyMode::Managed;
        rec.available_version = "0.0.9".to_string();
        apply_feed_check(
            &mut rec,
            &entry("0.1.0"),
            provision::OfflineArtifactStatus {
                present: true,
                up_to_date: true,
            },
        );
        assert_eq!(rec.installed_version, "0.1.0");
        assert_eq!(rec.available_version, "");
    }

    #[test]
    fn apply_feed_check_stale_or_absent_marks_available_without_touching_installed() {
        let mut stale = record("http://x");
        stale.installed_version = "0.1.0".to_string();
        apply_feed_check(
            &mut stale,
            &entry("0.2.0"),
            provision::OfflineArtifactStatus {
                present: true,
                up_to_date: false,
            },
        );
        assert_eq!(stale.available_version, "0.2.0");
        assert_eq!(
            stale.installed_version, "0.1.0",
            "stale keeps known installed version"
        );

        let mut absent = record("http://x");
        apply_feed_check(
            &mut absent,
            &entry("0.2.0"),
            provision::OfflineArtifactStatus {
                present: false,
                up_to_date: false,
            },
        );
        assert_eq!(
            absent.available_version, "0.2.0",
            "absent artifact is an installable update target"
        );
    }

    #[test]
    fn resolve_env_value_expands_setting_placeholder() {
        let got = resolve_env_value("${afm:settings:power.awamKnowledgeServiceToken}", |k| {
            assert_eq!(k, "power.awamKnowledgeServiceToken");
            "secret-123".to_string()
        });
        assert_eq!(got, "secret-123");
    }

    #[test]
    fn resolve_env_value_passthrough_without_lookup() {
        assert_eq!(
            resolve_env_value("plain-value", |_| panic!("must not look up")),
            "plain-value"
        );
        // 前缀不匹配 / 缺尾 `}` 都原样返回。
        assert_eq!(
            resolve_env_value("${afm:settings:x", |_| panic!("no lookup")),
            "${afm:settings:x"
        );
    }
}
