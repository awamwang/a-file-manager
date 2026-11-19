// SPDX-License-Identifier: GPL-3.0-or-later
//! 统一托管依赖（ADR 0019 / 0020）：awam-file / awam-tool-manager / awam-knowledge
//! 三常驻服务的连接、进程、健康、credential 权威。
//!
//! 当前进度：P1 注册表（`registry`）。后续按 Pilot 计划补：
//! - `orchestrator`（pack 无关 attach-or-spawn，泛化自 `tool_runtime/sidecar.rs`）
//! - `store`（元清单 allow-list + release 资产下载 + 摘要校验）
//! - `credential`（来源解析；目前 `CredentialSource` 已在 registry 定义）

pub mod commands;
pub mod compat;
pub mod credential;
pub mod orchestrator;
pub mod provision;
pub mod projection;
pub mod registry;
pub mod store;

// 预留：fetch_manifest_text / http_get_bytes 等真网络函数接线于前端/IPC 消费后移除 allow。
#[allow(unused_imports)]
pub use compat::{ensure_declared_version_compat, VersionRange};
#[allow(unused_imports)]
pub use credential::{resolve_credential, resolve_credential_native};
#[allow(unused_imports)]
pub use orchestrator::{
    attach_or_spawn, find_pack_by_id, pick_sidecar_for_pack, stop_self_spawned,
    stop_self_spawned_native, AttachOutcome,
};
#[allow(unused_imports)]
pub use store::{
    download_and_verify, find_entry, is_host_allowed, materialize, parse_manifest, verify_sha256,
    StoreEntry, StoreManifest,
};
#[allow(unused_imports)]
pub use provision::{artifact_file_name, provision_artifact, ARTIFACT_SUBDIR};
#[allow(unused_imports)]
pub use registry::{
    delete_record, list_records, load_record, prune_orphans, record_path, records_root,
    register_for_enabled_pack, save_record, unregister_for_pack, CredentialSource, DependencyMode,
    HealthState, ManagedDependencyRecord,
};
