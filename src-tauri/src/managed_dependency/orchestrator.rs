// SPDX-License-Identifier: GPL-3.0-or-later
//! pack 无关的托管依赖编排原语（ADR 0020）。泛化自 `tool_runtime::sidecar`：
//! - `attach_or_spawn`：先探 config-mode 实例，健康即附着，否则拉起自启产物（Q6/0013）。
//! - `stop_self_spawned`：只 kill AFM 自己 spawn 且记录 pid 的进程；非自启一律拒绝（Q8）。
//! - `find_pack_by_id` / `pick_sidecar_for_pack`：按 pack_id 取任意 pack 的 sidecar 贡献，不再硬编码 `awam.tool-runtime`。
//!
//! probe / spawn / kill 均以闭包注入，纯逻辑可单测。

use crate::extension_host::pack_fs::read_pack_json;
use crate::extension_host::types::{InstalledPackRecord, SidecarContribution};
use std::path::Path;

/// attach-or-spawn 结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachOutcome {
    /// 附着时为上次记录的 pid（无则 0）；拉起时为新进程 pid。
    pub pid: u32,
    /// 是否本次新拉起（附着为 false）。
    pub spawned: bool,
    /// 探活是否可达。
    pub reachable: bool,
}

/// 先探 `base_url`：可达 → 附着（返回 `last_pid`，未拉起）；不可达 → `spawn` 自启产物。
pub fn attach_or_spawn<Payload, P, S>(
    base_url: &str,
    last_pid: Option<u32>,
    pack_install_dir: &Path,
    contrib: &SidecarContribution,
    probe: P,
    spawn: S,
) -> Result<AttachOutcome, String>
where
    P: Fn(&str) -> Result<Payload, String>,
    S: FnOnce(&Path, &SidecarContribution) -> Result<u32, String>,
{
    if probe(base_url).is_ok() {
        return Ok(AttachOutcome {
            pid: last_pid.unwrap_or(0),
            spawned: false,
            reachable: true,
        });
    }
    let pid = spawn(pack_install_dir, contrib)?;
    Ok(AttachOutcome {
        pid,
        spawned: true,
        reachable: false,
    })
}

/// 停止：只 kill AFM 自启且记录的 `spawned_pid`；无记录（附着 / 远程实例）→ **拒绝**，绝不碰非 AFM 持有的进程（Q8）。
pub fn stop_self_spawned<K>(spawned_pid: Option<u32>, kill: K) -> Result<u32, String>
where
    K: FnOnce(u32),
{
    match spawned_pid {
        Some(pid) => {
            kill(pid);
            Ok(pid)
        }
        None => Err(
            "managed_dependency stop: refusing to stop non-self-spawned process".to_string(),
        ),
    }
}

/// 按 pack_id 取已安装 pack 记录（pack 无关，取代 `find_tool_runtime_pack`）。
pub fn find_pack_by_id(pack_id: &str) -> Option<InstalledPackRecord> {
    crate::extension_host::extension_host_list_packs()
        .ok()?
        .into_iter()
        .find(|pack| pack.id == pack_id)
}

/// 从 pack 安装目录的 `pack.json` 取第一条 sidecar 贡献（不硬编码 sidecar id）。
pub fn pick_sidecar_for_pack(pack: &InstalledPackRecord) -> Option<SidecarContribution> {
    let manifest = read_pack_json(Path::new(&pack.install_path)).ok()?;
    manifest.contributions.sidecars.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension_host::types::{ExtensionPermission, ExtensionRuntimeState};
    use std::cell::Cell;

    fn contrib() -> SidecarContribution {
        SidecarContribution {
            id: "some-service".to_string(),
            command: "python".to_string(),
            args: vec!["server.py".to_string()],
            working_dir_rel: Some("sidecar".to_string()),
            runtime: Default::default(),
            env: Default::default(),
        }
    }

    fn health_ok(_url: &str) -> Result<(), String> {
        Ok(())
    }

    fn health_down(_url: &str) -> Result<(), String> {
        Err("unreachable".to_string())
    }

    #[test]
    fn attach_or_spawn_attaches_when_reachable_without_spawn() {
        let dir = std::env::temp_dir();
        let spawned = Cell::new(false);
        let outcome = attach_or_spawn(
            "http://127.0.0.1:18787",
            Some(7),
            &dir,
            &contrib(),
            health_ok,
            |_d, _c| {
                spawned.set(true);
                Ok(99)
            },
        )
        .unwrap();
        assert!(!spawned.get(), "reachable must not spawn");
        assert_eq!(outcome, AttachOutcome { pid: 7, spawned: false, reachable: true });
    }

    #[test]
    fn attach_or_spawn_spawns_when_unreachable_and_uses_new_pid() {
        let dir = std::env::temp_dir();
        let spawned = Cell::new(false);
        let outcome = attach_or_spawn(
            "http://127.0.0.1:18787",
            None,
            &dir,
            &contrib(),
            health_down,
            |_d, _c| {
                spawned.set(true);
                Ok(42)
            },
        )
        .unwrap();
        assert!(spawned.get());
        assert_eq!(outcome, AttachOutcome { pid: 42, spawned: true, reachable: false });
    }

    #[test]
    fn stop_self_spawned_kills_only_recorded_pid() {
        let killed = Cell::new(0u32);
        let pid = stop_self_spawned(Some(123), |p| killed.set(p)).unwrap();
        assert_eq!(pid, 123);
        assert_eq!(killed.get(), 123);
    }

    #[test]
    fn stop_self_spawned_refuses_when_not_self_spawned() {
        let killed = Cell::new(false);
        let err = stop_self_spawned(None, |_| killed.set(true)).unwrap_err();
        assert!(err.contains("refusing to stop non-self-spawned"), "{err}");
        assert!(!killed.get(), "must not kill anything when not self-spawned");
    }

    #[test]
    fn pick_sidecar_for_pack_reads_first_contribution_from_pack_json() {
        let dir = std::env::temp_dir().join(format!(
            "afm-orchestrator-pack-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = r#"{
            "id": "awam.file-gateway",
            "name": "awam-file",
            "version": "0.1.0",
            "apiVersion": "feature-pack.1",
            "permissions": ["network", "spawnSidecar"],
            "contributions": {
                "sidecars": [
                    { "id": "awam-file", "command": "awam-file-gateway.exe", "args": ["http"] }
                ]
            }
        }"#;
        std::fs::write(dir.join("pack.json"), manifest).unwrap();

        let pack = InstalledPackRecord {
            id: "awam.file-gateway".to_string(),
            name: "awam-file".to_string(),
            version: "0.1.0".to_string(),
            state: ExtensionRuntimeState::Enabled,
            permissions: vec![ExtensionPermission::SpawnSidecar],
            install_path: dir.to_string_lossy().into_owned(),
            last_error: None,
        };
        let picked = pick_sidecar_for_pack(&pack).expect("should read first sidecar");
        assert_eq!(picked.command, "awam-file-gateway.exe");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
