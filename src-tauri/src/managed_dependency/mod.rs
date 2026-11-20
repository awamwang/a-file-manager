// SPDX-License-Identifier: GPL-3.0-or-later
//! 统一托管依赖（ADR 0019 / 0020 / 0022）：awam-file / awam-tool-manager / awam-knowledge
//! 三常驻服务的连接、进程、健康、credential 权威，以及 managed 产物 feed 拉取/校验/落盘。
//!
//! 接线现状：`registry`（权威记录）、`orchestrator`（attach-or-spawn）、`store`（allow-list + 下载 + SHA）、
//! `provision`（ensure/force/inspect 产物）、`feed`（自有发布通道配置 + 元清单）、`projection`（Health 投影）、
//! `commands`（IPC）均已接入生产路径。`credential` 为来源解析抽象（暂由后端自管 token 文件、AFM 未注入）。
//! 版本兼容（gatewayCompat）唯一实现见 `tool_runtime`（`semver_compat` / `check_gateway_compat`，每次网关调用处执行）。

pub mod commands;
pub mod credential;
pub mod feed;
pub mod net_port;
pub mod orchestrator;
pub mod projection;
pub mod provision;
pub mod registry;
pub mod store;

#[allow(unused_imports)]
pub use credential::{resolve_credential, resolve_credential_native};
#[allow(unused_imports)]
pub use orchestrator::{
    attach_or_spawn, find_pack_by_id, pick_sidecar_for_pack, stop_self_spawned, AttachOutcome,
};
#[allow(unused_imports)]
pub use provision::{
    artifact_file_name, ensure_artifact, force_provision, inspect_artifact_offline, ArtifactState,
    OfflineArtifactStatus, DEFAULT_SIDECAR_DIR,
};
#[allow(unused_imports)]
pub use registry::{
    delete_record, list_records, load_record, prune_orphans, record_path, records_root,
    register_for_enabled_pack, save_record, unregister_for_pack, CredentialSource, DependencyMode,
    HealthState, ManagedDependencyRecord,
};
#[allow(unused_imports)]
pub use store::{
    download_and_verify, download_verify_url_with_fallback, find_entry, find_entry_for_platform,
    is_host_allowed, materialize, parse_manifest, verify_sha256, PackEntry, StoreEntry,
    StoreManifest,
};
