// SPDX-License-Identifier: GPL-3.0-or-later

use super::config::{config_file_path, load_from_path, save_to_path, ToolRuntimeConfig};
use super::gateway::{health_check, HealthOk};
use super::TOOL_RUNTIME_PACK_ID;
use crate::extension_host::pack_fs::{pack_dir, packs_root, read_pack_json};
use crate::extension_host::types::{
    ExtensionPermission, ExtensionRuntimeState, InstalledPackRecord, SidecarContribution,
};
use crate::win_process;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// 占位 sidecar 贡献。真实启动命令以 awam-tool-manager README 为准：`python server.py`（stdio MCP）。
/// HTTP sidecar（计划中的 `--transport http` / 端口 8766）尚未写入该 README，故此处不写死仓库路径。
pub fn placeholder_sidecar_contribution() -> SidecarContribution {
    SidecarContribution {
        id: "awam-tool-manager".to_string(),
        command: "python".to_string(),
        args: vec![
            "-m".to_string(),
            "awam_tool_manager_placeholder".to_string(),
        ],
        working_dir_rel: Some("sidecar".to_string()),
    }
}

/// 网关探活注入点，便于单测在 reachable 时断言不 spawn。
pub trait GatewayProbe {
    fn health_check(&self, base_url: &str) -> Result<HealthOk, String>;
}

impl<F> GatewayProbe for F
where
    F: Fn(&str) -> Result<HealthOk, String>,
{
    fn health_check(&self, base_url: &str) -> Result<HealthOk, String> {
        self(base_url)
    }
}

/// `tool_runtime_ensure_gateway` 的 IPC 返回。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnsureGatewayResult {
    pub pid: u32,
    pub spawned: bool,
    pub reachable: bool,
    pub detail: String,
}

/// 优先附着已有网关；不可达时按贡献拉起 sidecar，返回进程 pid（附着时为上次记录或 0）。
pub fn start_sidecar(
    app_data: &Path,
    pack_install_dir: &Path,
    contrib: &SidecarContribution,
) -> Result<u32, String> {
    start_sidecar_with_probe(
        app_data,
        pack_install_dir,
        contrib,
        &health_check,
        spawn_sidecar_process,
    )
}

/// 可注入探活与 spawn，供单测断言 reachable 时不拉起进程。
pub fn start_sidecar_with_probe<P, S>(
    app_data: &Path,
    pack_install_dir: &Path,
    contrib: &SidecarContribution,
    probe: &P,
    spawn: S,
) -> Result<u32, String>
where
    P: GatewayProbe,
    S: FnOnce(&Path, &SidecarContribution) -> Result<u32, String>,
{
    let cfg = load_config(app_data);
    if probe.health_check(&cfg.base_url).is_ok() {
        return Ok(cfg.last_sidecar_pid.unwrap_or(0));
    }
    let pid = spawn(pack_install_dir, contrib)?;
    let mut cfg = cfg;
    cfg.last_sidecar_pid = Some(pid);
    save_to_path(app_data, &cfg)?;
    Ok(pid)
}

/// 先探活附着；失败则校验 spawnSidecar 后按 Pack 贡献拉起。
pub fn ensure_gateway(app_data: &Path) -> Result<EnsureGatewayResult, String> {
    ensure_gateway_with(app_data, &health_check, spawn_sidecar_process)
}

pub fn ensure_gateway_with<P, S>(
    app_data: &Path,
    probe: &P,
    spawn: S,
) -> Result<EnsureGatewayResult, String>
where
    P: GatewayProbe,
    S: FnOnce(&Path, &SidecarContribution) -> Result<u32, String>,
{
    let cfg = load_config(app_data);
    if probe.health_check(&cfg.base_url).is_ok() {
        return Ok(EnsureGatewayResult {
            pid: cfg.last_sidecar_pid.unwrap_or(0),
            spawned: false,
            reachable: true,
            detail: "attached".to_string(),
        });
    }

    let pack = find_tool_runtime_pack();
    authorize_sidecar_spawn(pack.as_ref(), cfg.auto_start_sidecar)?;

    let pack_install_dir = pack
        .as_ref()
        .map(|record| PathBuf::from(&record.install_path))
        .unwrap_or_else(|| pack_dir(&packs_root(app_data), TOOL_RUNTIME_PACK_ID));
    let contrib = load_sidecar_contribution(pack.as_ref());
    let pid = start_sidecar_with_probe(app_data, &pack_install_dir, &contrib, probe, spawn)?;
    Ok(EnsureGatewayResult {
        pid,
        spawned: true,
        reachable: false,
        detail: "spawned".to_string(),
    })
}

fn load_config(app_data: &Path) -> ToolRuntimeConfig {
    if !config_file_path(app_data).exists() {
        return ToolRuntimeConfig::default();
    }
    load_from_path(app_data).unwrap_or_default()
}

pub(crate) fn find_tool_runtime_pack() -> Option<InstalledPackRecord> {
    crate::extension_host::extension_host_list_packs()
        .ok()?
        .into_iter()
        .find(|pack| pack.id == TOOL_RUNTIME_PACK_ID)
}

fn load_sidecar_contribution(pack: Option<&InstalledPackRecord>) -> SidecarContribution {
    if let Some(pack) = pack {
        if let Ok(manifest) = read_pack_json(Path::new(&pack.install_path)) {
            if let Some(sidecar) = pick_sidecar(manifest.contributions.sidecars) {
                return sidecar;
            }
        }
        if let Ok(aggregated) = crate::extension_host::extension_host_list_contributions() {
            if let Some(sidecar) = pick_sidecar(aggregated.sidecars) {
                return sidecar;
            }
        }
    }
    placeholder_sidecar_contribution()
}

fn pick_sidecar(sidecars: Vec<SidecarContribution>) -> Option<SidecarContribution> {
    sidecars
        .iter()
        .find(|item| item.id == "awam-tool-manager")
        .cloned()
        .or_else(|| sidecars.into_iter().next())
}

/// 已启用 Pack 需含 spawnSidecar；尚未安装时仅在 autoStartSidecar 下放行（便于单测）。
pub(crate) fn authorize_sidecar_spawn(
    pack: Option<&InstalledPackRecord>,
    auto_start_sidecar: bool,
) -> Result<(), String> {
    match pack {
        Some(pack) if pack.state == ExtensionRuntimeState::Enabled => {
            if pack
                .permissions
                .contains(&ExtensionPermission::SpawnSidecar)
            {
                Ok(())
            } else {
                Err("missing permission: spawnSidecar".to_string())
            }
        }
        Some(_) => Err("tool_runtime_disabled".to_string()),
        None => {
            if auto_start_sidecar {
                Ok(())
            } else {
                Err("tool_runtime_disabled".to_string())
            }
        }
    }
}

fn resolve_working_dir(pack_install_dir: &Path, contrib: &SidecarContribution) -> PathBuf {
    match contrib.working_dir_rel.as_deref() {
        Some(rel) if !rel.trim().is_empty() => pack_install_dir.join(rel),
        _ => pack_install_dir.to_path_buf(),
    }
}

fn spawn_sidecar_process(
    pack_install_dir: &Path,
    contrib: &SidecarContribution,
) -> Result<u32, String> {
    if contrib.command.trim().is_empty() {
        return Err("sidecar command is empty".to_string());
    }
    let cwd = resolve_working_dir(pack_install_dir, contrib);
    let mut cmd = win_process::command(&contrib.command);
    cmd.args(&contrib.args)
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = cmd.spawn().map_err(|err| {
        format!(
            "Failed to spawn sidecar '{command}' (cwd {cwd}): {err}",
            command = contrib.command,
            cwd = cwd.display()
        )
    })?;
    Ok(child.id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn health_ok(_url: &str) -> Result<HealthOk, String> {
        Ok(HealthOk {
            status: "ok".to_string(),
            transport: Some("http".to_string()),
            version: None,
        })
    }

    fn health_down(_url: &str) -> Result<HealthOk, String> {
        Err("unreachable".to_string())
    }

    fn sample_contrib() -> SidecarContribution {
        SidecarContribution {
            id: "awam-tool-manager".to_string(),
            command: "python".to_string(),
            args: vec![
                "-m".to_string(),
                "awam_tool_manager_placeholder".to_string(),
            ],
            working_dir_rel: Some("sidecar".to_string()),
        }
    }

    fn write_last_pid(app_data: &Path, pid: u32) {
        let mut cfg = ToolRuntimeConfig::default();
        cfg.last_sidecar_pid = Some(pid);
        save_to_path(app_data, &cfg).unwrap();
    }

    #[test]
    fn start_sidecar_skips_spawn_when_health_reachable() {
        let app = tempfile::tempdir().unwrap();
        let pack = tempfile::tempdir().unwrap();
        write_last_pid(app.path(), 7);
        let spawned = Cell::new(false);
        let pid = start_sidecar_with_probe(
            app.path(),
            pack.path(),
            &sample_contrib(),
            &health_ok,
            |_dir, _contrib| {
                spawned.set(true);
                Ok(99)
            },
        )
        .unwrap();
        assert_eq!(pid, 7);
        assert!(!spawned.get(), "reachable gateway must not spawn sidecar");
    }

    #[test]
    fn start_sidecar_spawns_when_unreachable() {
        let app = tempfile::tempdir().unwrap();
        let pack = tempfile::tempdir().unwrap();
        let spawned = Cell::new(false);
        let pid = start_sidecar_with_probe(
            app.path(),
            pack.path(),
            &sample_contrib(),
            &health_down,
            |dir, contrib| {
                spawned.set(true);
                assert_eq!(
                    resolve_working_dir(dir, contrib),
                    dir.join("sidecar"),
                    "workingDirRel is relative to pack install dir"
                );
                Ok(42)
            },
        )
        .unwrap();
        assert!(spawned.get());
        assert_eq!(pid, 42);
        let loaded = load_from_path(app.path()).unwrap();
        assert_eq!(loaded.last_sidecar_pid, Some(42));
    }

    #[test]
    fn ensure_gateway_attaches_without_pack_when_reachable() {
        let app = tempfile::tempdir().unwrap();
        let result = ensure_gateway_with(app.path(), &health_ok, |_dir, _contrib| {
            panic!("reachable ensure must not spawn");
        })
        .unwrap();
        assert!(!result.spawned);
        assert!(result.reachable);
        assert_eq!(result.detail, "attached");
    }

    #[test]
    fn ensure_gateway_allows_uninstalled_pack_when_auto_start() {
        let app = tempfile::tempdir().unwrap();
        let mut cfg = ToolRuntimeConfig::default();
        cfg.auto_start_sidecar = true;
        save_to_path(app.path(), &cfg).unwrap();
        let result = ensure_gateway_with(app.path(), &health_down, |_dir, contrib| {
            assert_eq!(contrib.command, "python");
            Ok(11)
        })
        .unwrap();
        assert!(result.spawned);
        assert_eq!(result.pid, 11);
    }

    #[test]
    fn authorize_requires_enabled_spawn_sidecar() {
        let pack = InstalledPackRecord {
            id: TOOL_RUNTIME_PACK_ID.to_string(),
            name: "工具管理运行时".to_string(),
            version: "0.1.0".to_string(),
            state: ExtensionRuntimeState::Enabled,
            permissions: vec![ExtensionPermission::Network],
            install_path: "/tmp/pack".to_string(),
            last_error: None,
        };
        let err = authorize_sidecar_spawn(Some(&pack), false).unwrap_err();
        assert_eq!(err, "missing permission: spawnSidecar");

        let mut enabled = pack.clone();
        enabled.permissions.push(ExtensionPermission::SpawnSidecar);
        assert!(authorize_sidecar_spawn(Some(&enabled), false).is_ok());

        let mut disabled = enabled.clone();
        disabled.state = ExtensionRuntimeState::Disabled;
        assert_eq!(
            authorize_sidecar_spawn(Some(&disabled), false).unwrap_err(),
            "tool_runtime_disabled"
        );
        assert_eq!(
            authorize_sidecar_spawn(None, false).unwrap_err(),
            "tool_runtime_disabled"
        );
        assert!(authorize_sidecar_spawn(None, true).is_ok());
    }

    #[test]
    fn placeholder_sidecar_matches_plan_until_http_start_is_documented() {
        let contrib = placeholder_sidecar_contribution();
        assert_eq!(contrib.id, "awam-tool-manager");
        assert_eq!(contrib.command, "python");
        assert_eq!(
            contrib.args,
            vec![
                "-m".to_string(),
                "awam_tool_manager_placeholder".to_string()
            ]
        );
        assert_eq!(
            crate::tool_runtime::DEFAULT_BASE_URL,
            "http://127.0.0.1:8766"
        );
    }
}
