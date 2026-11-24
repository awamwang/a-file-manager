// SPDX-License-Identifier: GPL-3.0-or-later

use super::config::{config_file_path, load_from_path, ToolRuntimeConfig};
use super::gateway::{health_check, HealthOk};
use super::TOOL_RUNTIME_PACK_ID;
use crate::extension_host::pack_fs::{pack_dir, packs_root, read_pack_json};
use crate::extension_host::types::{
    ExtensionPermission, ExtensionRuntimeState, InstalledPackRecord, SharedRuntime,
    SidecarContribution, SidecarRuntime,
};
use crate::managed_dependency::registry::{self, DependencyMode, ManagedDependencyRecord};
use crate::win_process;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// 工具页托管 spawn 端口被占时最多顺延跨度（与 managed_dependency 命令层保持一致语义）。
const TOOL_PORT_SPAN: u16 = 32;

/// 占位 sidecar 贡献。真实启动命令以 awam-tool-manager README 为准：`python server.py`（stdio MCP）。
/// HTTP sidecar（计划中的 `--transport http` / 端口 18778）尚未写入该 README，故此处不写死仓库路径。
pub fn placeholder_sidecar_contribution() -> SidecarContribution {
    SidecarContribution {
        id: "awam-tool-manager".to_string(),
        command: crate::platform::python_command().to_string(),
        args: vec![
            "-m".to_string(),
            "awam_tool_manager_placeholder".to_string(),
        ],
        working_dir_rel: Some("sidecar".to_string()),
        runtime: SidecarRuntime::Self_,
        env: Default::default(),
        port_env: Some("AWAM_HTTP_PORT".to_string()),
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
///
/// 决策核心已泛化到 `managed_dependency::orchestrator::attach_or_spawn`（Q6/0013）。
/// C-4：进程态唯一权威是 managed_dependency record（`record.id = pack id`），spawn 后
/// 写回 `record.spawned_pid`；`ToolRuntimeConfig` 退化为连接偏好，不再承载 pid。
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
    let mut record = tool_runtime_record(app_data);
    // 端口回退（ADR 0022）：托管且声明了 port_env 时，若目标端口探活不通且被占，顺延到可绑端口，
    // 写回 record.base_url 并注入子进程 env；探活健康（附着）则绝不改端口。
    let mut contrib_owned = contrib.clone();
    let mut base_url = effective_base_url(&cfg, &record);
    if record.mode == crate::managed_dependency::registry::DependencyMode::Managed {
        if let Some(var) = contrib_owned.port_env.clone() {
            if probe.health_check(&base_url).is_err() {
                base_url = crate::managed_dependency::net_port::relocate_if_occupied(
                    &base_url,
                    TOOL_PORT_SPAN,
                );
                record.base_url = base_url.clone();
            }
            if let Some(port) = crate::managed_dependency::net_port::port_of(&base_url) {
                contrib_owned.env.insert(var, port.to_string());
            }
        }
    }
    let outcome = crate::managed_dependency::orchestrator::attach_or_spawn(
        &base_url,
        record.spawned_pid,
        pack_install_dir,
        &contrib_owned,
        |url| probe.health_check(url),
        spawn,
    )?;
    if outcome.spawned {
        record.spawned_pid = Some(outcome.pid);
        registry::save_record(app_data, &record)?;
    }
    Ok(outcome.pid)
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
    let record = tool_runtime_record(app_data);
    let base_url = effective_base_url(&cfg, &record);
    if probe.health_check(&base_url).is_ok() {
        return Ok(EnsureGatewayResult {
            pid: record.spawned_pid.unwrap_or(0),
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

/// managed 模式以 feed 写入的 `record.base_url` 为权威（避免与可编辑的 ToolRuntimeConfig 漂移）；
/// 否则用 `ToolRuntimeConfig.base_url`（config-mode 用户自连）。ADR 0022 网关地址单一来源。
fn effective_base_url(cfg: &ToolRuntimeConfig, record: &ManagedDependencyRecord) -> String {
    if record.mode == DependencyMode::Managed && !record.base_url.trim().is_empty() {
        return record.base_url.clone();
    }
    cfg.base_url.clone()
}

/// 跨命令统一的「当前生效网关地址」：managed→feed 写入的 `record.base_url`，否则→`ToolRuntimeConfig.base_url`。
/// status / mcp 调用 / ensure 都走它，避免 config 残留地址与 feed 权威漂移。
pub(crate) fn effective_base_url_for(app_data: &Path) -> String {
    let cfg = load_config(app_data);
    let record = tool_runtime_record(app_data);
    effective_base_url(&cfg, &record)
}

/// 取/建 tool-runtime pack 的托管记录（C-4：进程态唯一权威，`record.id = pack id`）。
/// 启用时 `register_for_enabled_pack` 尽力建记录；缺失时这里就地兜底建一条空记录，
/// 保证 ensure/spawn 有处持久化 `spawned_pid`。
fn tool_runtime_record(app_data: &Path) -> ManagedDependencyRecord {
    registry::load_record(app_data, TOOL_RUNTIME_PACK_ID)
        .ok()
        .flatten()
        .unwrap_or_else(|| ManagedDependencyRecord {
            id: TOOL_RUNTIME_PACK_ID.to_string(),
            pack_id: TOOL_RUNTIME_PACK_ID.to_string(),
            ..Default::default()
        })
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

/// 按 runtime 档解析真正的可执行与参数（ADR 0022：shared-first + 自带优先）。
/// - `self`：产物自带解释器，`command` 即 argv0、`args` 原样（与迁移前行为一致）。
/// - `shared(python|node)`：用 AFM 提供的共享解释器作 argv0，把 `command` 当脚本/入口
///   前置到 `args` 之前。共享解释器目前取平台默认（`python3` / `node`）；AFM 自装共享
///   运行时的精确定位留 `managed_dependency::provision` 接线（见 ADR 0022 落地记录）。
pub(crate) fn resolve_spawn_command(contrib: &SidecarContribution) -> (String, Vec<String>) {
    match contrib.runtime {
        SidecarRuntime::Self_ => (contrib.command.clone(), contrib.args.clone()),
        SidecarRuntime::Shared(kind) => {
            let exe = match kind {
                SharedRuntime::Python => crate::platform::python_command().to_string(),
                SharedRuntime::Node => crate::platform::node_command().to_string(),
            };
            let mut argv = Vec::with_capacity(1 + contrib.args.len());
            argv.push(contrib.command.clone());
            argv.extend(contrib.args.iter().cloned());
            (exe, argv)
        }
    }
}

/// 生产 sidecar 拉起实现（CREATE_NO_WINDOW + workingDirRel）。
/// `pub(crate)`：managed_dependency 命令层复用同一实现，避免双份 spawn 逻辑。
pub(crate) fn spawn_sidecar_process(
    pack_install_dir: &Path,
    contrib: &SidecarContribution,
) -> Result<u32, String> {
    let (exe, argv) = resolve_spawn_command(contrib);
    if exe.trim().is_empty() {
        return Err("sidecar command is empty".to_string());
    }
    let cwd = resolve_working_dir(pack_install_dir, contrib);
    let mut cmd = win_process::command(&exe);
    cmd.args(&argv)
        .current_dir(&cwd)
        .envs(&contrib.env)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = cmd.spawn().map_err(|err| {
        format!(
            "Failed to spawn sidecar '{exe}' (cwd {cwd}): {err}",
            cwd = cwd.display()
        )
    })?;
    Ok(child.id())
}

#[cfg(test)]
mod tests {
    use super::super::config::save_to_path;
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
            command: crate::platform::python_command().to_string(),
            args: vec![
                "-m".to_string(),
                "awam_tool_manager_placeholder".to_string(),
            ],
            working_dir_rel: Some("sidecar".to_string()),
            runtime: SidecarRuntime::Self_,
            env: Default::default(),
            port_env: None,
        }
    }

    fn write_last_pid(app_data: &Path, pid: u32) {
        let mut record = ManagedDependencyRecord {
            id: TOOL_RUNTIME_PACK_ID.to_string(),
            pack_id: TOOL_RUNTIME_PACK_ID.to_string(),
            ..Default::default()
        };
        record.spawned_pid = Some(pid);
        registry::save_record(app_data, &record).unwrap();
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
        let record = registry::load_record(app.path(), TOOL_RUNTIME_PACK_ID)
            .unwrap()
            .expect("record written on spawn");
        assert_eq!(record.spawned_pid, Some(42));
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
            assert_eq!(contrib.command, crate::platform::python_command());
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
        assert_eq!(contrib.command, crate::platform::python_command());
        assert_eq!(
            contrib.args,
            vec![
                "-m".to_string(),
                "awam_tool_manager_placeholder".to_string()
            ]
        );
        assert_eq!(
            crate::tool_runtime::DEFAULT_BASE_URL,
            "http://127.0.0.1:18778"
        );
    }

    #[test]
    fn sidecar_env_defaults_empty_when_absent_in_manifest() {
        let json = r#"{"id":"x","command":"tool.exe","args":[],"runtime":"self"}"#;
        let c: SidecarContribution = serde_json::from_str(json).unwrap();
        assert!(c.env.is_empty(), "env 向后兼容：旧清单缺字段=空");
    }

    #[test]
    fn runtime_self_passthrough_command_as_argv0() {
        let c = SidecarContribution {
            id: "x".into(),
            command: "tool.exe".into(),
            args: vec!["http".into()],
            working_dir_rel: None,
            runtime: SidecarRuntime::Self_,
            env: Default::default(),
            port_env: None,
        };
        let (exe, argv) = resolve_spawn_command(&c);
        assert_eq!(exe, "tool.exe");
        assert_eq!(argv, vec!["http".to_string()]);
    }

    #[test]
    fn runtime_shared_python_uses_interpreter_and_prepends_entry() {
        let c = SidecarContribution {
            id: "x".into(),
            command: "server.py".into(),
            args: vec!["--transport".into(), "http".into()],
            working_dir_rel: None,
            runtime: SidecarRuntime::Shared(SharedRuntime::Python),
            env: Default::default(),
            port_env: None,
        };
        let (exe, argv) = resolve_spawn_command(&c);
        assert_eq!(exe, crate::platform::python_command());
        assert_eq!(
            argv,
            vec!["server.py".to_string(), "--transport".into(), "http".into()]
        );
    }

    #[test]
    fn runtime_shared_node_uses_node_and_prepends_entry() {
        let c = SidecarContribution {
            id: "x".into(),
            command: "http-server.js".into(),
            args: vec![],
            working_dir_rel: None,
            runtime: SidecarRuntime::Shared(SharedRuntime::Node),
            env: Default::default(),
            port_env: None,
        };
        let (exe, argv) = resolve_spawn_command(&c);
        // macOS 可能命中 Homebrew 绝对路径；与 python 测试一样跟随平台解析结果。
        assert_eq!(exe, crate::platform::node_command());
        assert_eq!(argv, vec!["http-server.js".to_string()]);
    }

    #[test]
    fn sidecar_runtime_defaults_to_self_when_absent_in_manifest() {
        // 旧 pack.json 没有 runtime 字段 → 必须解析为 self（向后兼容）。
        let legacy = r#"{"id":"s","command":"tool.exe","args":["http"]}"#;
        let parsed: SidecarContribution = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.runtime, SidecarRuntime::Self_);
        // 显式 shared(python) 能解析。
        let shared = r#"{"id":"s","command":"server.py","args":[],"runtime":{"shared":"python"}}"#;
        let parsed: SidecarContribution = serde_json::from_str(shared).unwrap();
        assert_eq!(
            parsed.runtime,
            SidecarRuntime::Shared(SharedRuntime::Python)
        );
    }

    #[test]
    fn effective_base_url_prefers_feed_record_when_managed() {
        let cfg = ToolRuntimeConfig {
            base_url: "http://127.0.0.1:18777".into(),
            ..Default::default()
        };
        let mut rec = ManagedDependencyRecord {
            id: "awam.tool-runtime".into(),
            pack_id: "awam.tool-runtime".into(),
            base_url: "http://127.0.0.1:18778".into(),
            ..Default::default()
        };
        // managed → 以 feed 写入的 record.base_url 为权威，忽略可编辑的 config 残留。
        rec.mode = DependencyMode::Managed;
        assert_eq!(effective_base_url(&cfg, &rec), "http://127.0.0.1:18778");
        // config-mode → 用 ToolRuntimeConfig.base_url。
        rec.mode = DependencyMode::Config;
        assert_eq!(effective_base_url(&cfg, &rec), "http://127.0.0.1:18777");
        // managed 但 record 无地址 → 回退 config（不误探空地址）。
        rec.mode = DependencyMode::Managed;
        rec.base_url = String::new();
        assert_eq!(effective_base_url(&cfg, &rec), "http://127.0.0.1:18777");
    }
}
