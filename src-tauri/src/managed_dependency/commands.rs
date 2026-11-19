// SPDX-License-Identifier: GPL-3.0-or-later
//! 托管依赖 IPC 命令（Task 5 后端接线）。逻辑内核（`ensure_record_with` / `stop_record_with`）
//! 纯 `&Path` + 注入闭包，离线单测；Tauri 包装仅做目录解析与 spawn_blocking。

use super::feed;
use super::orchestrator::{attach_or_spawn, find_pack_by_id, pick_sidecar_for_pack, stop_self_spawned};
use super::provision;
use super::registry::{self, DependencyMode, HealthState, ManagedDependencyRecord};
use super::store;
use crate::extension_host::types::SidecarContribution;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

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
pub(crate) fn stop_record_with<K>(record: &mut ManagedDependencyRecord, kill: K) -> Result<(), String>
where
    K: FnOnce(u32),
{
    stop_self_spawned(record.spawned_pid, kill)?;
    record.spawned_pid = None;
    record.health = HealthState::Unhealthy;
    Ok(())
}

/// ensure 的公共前置：按 `record.pack_id` 解析启用 pack + 其 sidecar 贡献（1:1 校验）。
fn resolve_pack_sidecar(record: &ManagedDependencyRecord) -> Result<(PathBuf, SidecarContribution), String> {
    let pack = find_pack_by_id(&record.pack_id)
        .ok_or_else(|| format!("managed_dependency ensure: pack '{}' not installed", record.pack_id))?;
    if pack.state != crate::extension_host::types::ExtensionRuntimeState::Enabled {
        return Err(format!(
            "managed_dependency ensure: pack '{}' not enabled",
            record.pack_id
        ));
    }
    let contrib = pick_sidecar_for_pack(&pack)
        .ok_or_else(|| format!("managed_dependency ensure: pack '{}' declares no sidecar", record.pack_id))?;
    Ok((PathBuf::from(&pack.install_path), contrib))
}

/// managed 拉起前准备 sidecar 贡献（ADR 0022 接线）：
/// - 非 `Managed` → 原样返回（产物随 pack 内置 / 用户 config 连接，无需 feed 下载）。
/// - `Managed` → 读 feed 元清单找本 pack entry；命中且本地产物缺失则下载校验落盘，并把
///   `contrib.command` 改写为落盘产物绝对路径（`runtime:self` 档用之作 argv0）。
///   元清单不可达 / 未配置 / 无 entry → 原样返回（尽力而为，不误伤已有产物；前端据 mode 展示 managed）。
fn prepare_contrib_for_spawn(
    app_data: &Path,
    record: &ManagedDependencyRecord,
    pack_install_dir: &Path,
    mut contrib: SidecarContribution,
) -> Result<SidecarContribution, String> {
    if record.mode != DependencyMode::Managed {
        return Ok(contrib);
    }
    let feed_cfg = feed::load_feed(app_data);
    let manifest = match feed::fetch_manifest(&feed_cfg, store::fetch_manifest_text) {
        Ok(m) => m,
        Err(_) => return Ok(contrib),
    };
    let entry = manifest.as_ref().and_then(|m| store::find_entry(m, &record.pack_id));
    match provision::ensure_artifact(
        entry,
        &feed_cfg.allow_hosts,
        pack_install_dir,
        contrib.working_dir_rel.as_deref(),
        store::http_get_bytes,
    )? {
        Some(path) => {
            contrib.command = path.to_string_lossy().into_owned();
            Ok(contrib)
        }
        None => Ok(contrib),
    }
}

// ---- Tauri 命令 ----

/// 通用探活（`GET {base}/health`，免鉴权约定与三个依赖一致）。
fn probe(url: &str) -> Result<crate::tool_runtime::HealthOk, String> {
    crate::tool_runtime::health_check(url)
}

/// 生产 spawn：复用 tool_runtime 的进程拉起实现（CREATE_NO_WINDOW、workingDirRel 解析）。
fn native_spawn(
    pack_install_dir: &Path,
    contrib: &SidecarContribution,
) -> Result<u32, String> {
    crate::tool_runtime::spawn_sidecar_process(pack_install_dir, contrib)
}

fn app_config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|err| format!("managed_dependency: app config dir: {err}"))
}

/// 列出全部托管依赖记录（权威态；投影给 Health/deps_probe）。
#[tauri::command]
pub async fn managed_dependency_list(app: AppHandle) -> Result<Vec<ManagedDependencyRecord>, String> {
    let dir = app_config_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || registry::list_records(&dir))
        .await
        .map_err(|err| format!("managed_dependency_list worker failed: {err}"))?
}

/// 设置 managed 产物 feed（AFM 自有发布通道）的元清单 URL 与 allow-list 主机（ADR 0022）。
/// 写入后，下一次 pack 启用即据此把有 entry 的 pack 切 Managed、ensure 据此 provision 产物。
#[tauri::command]
pub async fn managed_dependency_set_feed(
    app: AppHandle,
    manifest_url: String,
    allow_hosts: Vec<String>,
) -> Result<feed::FeedConfig, String> {
    let dir = app_config_dir(&app)?;
    let cfg = feed::FeedConfig { manifest_url, allow_hosts };
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
pub async fn managed_dependency_stop(app: AppHandle, id: String) -> Result<ManagedDependencyRecord, String> {
    let dir = app_config_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut record = registry::load_record(&dir, &id)?
            .ok_or_else(|| format!("managed_dependency stop: unknown id '{id}'"))?;
        stop_record_with(&mut record, |pid| crate::win_process::kill_process_tree(pid))?;
        registry::save_record(&dir, &record)?;
        Ok(record)
    })
    .await
    .map_err(|err| format!("managed_dependency_stop worker failed: {err}"))?
}

/// 确保指定托管依赖可用：先探 config-mode，健康附着；否则按 pack sidecar 贡献拉起。
#[tauri::command]
pub async fn managed_dependency_ensure(app: AppHandle, id: String) -> Result<ManagedDependencyRecord, String> {
    let dir = app_config_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut record = registry::load_record(&dir, &id)?
            .ok_or_else(|| format!("managed_dependency ensure: unknown id '{id}'"))?;
        // config-mode 不自动拉起（由工具页手动附着/连接）；仅托管记录走 provision+spawn。
        if record.mode != DependencyMode::Managed {
            return Ok(record);
        }
        let (pack_install_dir, contrib) = resolve_pack_sidecar(&record)?;
        let contrib = prepare_contrib_for_spawn(&dir, &record, &pack_install_dir, contrib)?;
        ensure_record_with(&mut record, &pack_install_dir, &contrib, probe, native_spawn)?;
        registry::save_record(&dir, &record)?;
        Ok(record)
    })
    .await
    .map_err(|err| format!("managed_dependency_ensure worker failed: {err}"))?
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
        }
    }

    #[test]
    fn ensure_requires_configured_base_url() {
        let mut rec = record("");
        let called = Cell::new(false);
        let err = ensure_record_with(&mut rec, Path::new("/tmp"), &contrib(),
            |_| Ok(()), |_d, _c| { called.set(true); Ok(1) })
            .unwrap_err();
        assert!(err.contains("base_url not configured"), "{err}");
        assert!(!called.get());
    }

    #[test]
    fn ensure_attach_keeps_existing_pid_and_sets_healthy() {
        let mut rec = record("http://127.0.0.1:18787");
        rec.spawned_pid = Some(7);
        let spawned = Cell::new(false);
        ensure_record_with(&mut rec, Path::new("/tmp"), &contrib(),
            |_| Ok(()), |_d, _c| { spawned.set(true); Ok(99) })
            .unwrap();
        assert!(!spawned.get());
        assert_eq!(rec.spawned_pid, Some(7), "attach must not clobber pid");
        assert_eq!(rec.health, HealthState::Healthy);
    }

    #[test]
    fn ensure_spawn_records_new_pid() {
        let mut rec = record("http://127.0.0.1:18787");
        ensure_record_with(&mut rec, Path::new("/tmp"), &contrib(),
            |_| Err::<(), _>("down".to_string()), |_d, _c| Ok(42))
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
}
