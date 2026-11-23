// SPDX-License-Identifier: GPL-3.0-or-later

use super::pack_fs;
use super::permissions::{parse_permissions, requires};
use super::types::{
    AggregatedContributions, ConfigContribution, ExtensionHostStatus, ExtensionPermission,
    ExtensionRuntimeState, FeaturePackContributions, FeaturePackManifest, InstallablePack,
    InstalledPackRecord, PackInstallResult, FEATURE_PACK_API_VERSION,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// In-process Feature Pack host.
#[derive(Default)]
pub struct ExtensionHost {
    /// 本机 Feature Pack 安装根对应的 APPDATA 目录。
    app_data_dir: Mutex<Option<PathBuf>>,
    packs: Mutex<BTreeMap<String, InstalledPackRecord>>,
    pack_contributions: Mutex<BTreeMap<String, FeaturePackContributions>>,
}

impl ExtensionHost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn status(&self) -> Result<ExtensionHostStatus, String> {
        let packs = self.packs.lock().map_err(|err| err.to_string())?;
        let installed_count = packs.len();
        let enabled_count = packs
            .values()
            .filter(|pack| pack.state == ExtensionRuntimeState::Enabled)
            .count();
        let crashed_count = packs
            .values()
            .filter(|pack| pack.state == ExtensionRuntimeState::Crashed)
            .count();
        Ok(ExtensionHostStatus {
            api_version: FEATURE_PACK_API_VERSION.to_string(),
            installed_count,
            enabled_count,
            crashed_count,
        })
    }

    /// 从 `{app_data_dir}/feature-packs` 加载已安装 Pack 与启用状态。
    pub fn load_from_disk(&self, app_data_dir: &Path) -> Result<(), String> {
        {
            let mut dir = self.app_data_dir.lock().map_err(|err| err.to_string())?;
            *dir = Some(app_data_dir.to_path_buf());
        }
        let root = pack_fs::packs_root(app_data_dir);
        let state = pack_fs::load_state(&root)?;
        let mut packs = BTreeMap::new();
        let mut contributions = BTreeMap::new();
        if root.is_dir() {
            for entry in fs::read_dir(&root)
                .map_err(|err| format!("Failed to read feature-packs directory: {err}"))?
            {
                let entry =
                    entry.map_err(|err| format!("Failed to read feature-packs entry: {err}"))?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with('.') {
                    continue;
                }
                match self.load_pack_record(&path, &state) {
                    Ok((record, contrib)) => {
                        let id = record.id.clone();
                        packs.insert(id.clone(), record);
                        contributions.insert(id, contrib);
                    }
                    Err(err) => {
                        let id = name_str.to_string();
                        packs.insert(
                            id.clone(),
                            InstalledPackRecord {
                                id,
                                name: name_str.to_string(),
                                version: String::new(),
                                state: ExtensionRuntimeState::Crashed,
                                permissions: Vec::new(),
                                install_path: path.to_string_lossy().into_owned(),
                                last_error: Some(err),
                            },
                        );
                    }
                }
            }
        }
        *self.packs.lock().map_err(|err| err.to_string())? = packs;
        *self
            .pack_contributions
            .lock()
            .map_err(|err| err.to_string())? = contributions;
        // 1:1 兜底（Q21）：清掉指向已卸载 pack 的孤儿托管记录。尽力而为，不阻断加载。
        let valid_pack_ids: std::collections::BTreeSet<String> = self
            .packs
            .lock()
            .map_err(|err| err.to_string())?
            .keys()
            .cloned()
            .collect();
        if let Err(err) = crate::managed_dependency::registry::prune_orphans(app_data_dir, &valid_pack_ids) {
            eprintln!("managed_dependency prune_orphans failed: {err}");
        }
        // 加载完成后按当前启用 Pack 重算 config schema overlay（使 pack 声明的配置项可落盘）。
        self.sync_config_overlay();
        Ok(())
    }

    /// 从目录或 zip 安装 Feature Pack（默认 Installed，需显式启用）。
    pub fn install_pack(&self, source_path: &Path) -> PackInstallResult {
        match self.install_pack_inner(source_path) {
            Ok(pack) => PackInstallResult {
                success: true,
                pack: Some(pack),
                error: None,
            },
            Err(error) => PackInstallResult {
                success: false,
                pack: None,
                error: Some(error),
            },
        }
    }

    /// feed「可安装 Store」列表：读 feed manifest 的 packs 段 + 标记本机已装状态（ADR 0022）。
    /// 网络（fetch manifest）由命令层放 spawn_blocking；feed 不可达时传播 Err 供前端提示。
    pub fn list_installable_packs(&self) -> Result<Vec<InstallablePack>, String> {
        let app_data = self.require_app_data_dir()?;
        let cfg = crate::managed_dependency::feed::load_feed(&app_data);
        let manifest = crate::managed_dependency::feed::fetch_manifest(
            &cfg,
            crate::managed_dependency::store::fetch_manifest_text,
        )?;
        let installed: BTreeMap<String, String> = self
            .packs
            .lock()
            .map_err(|err| err.to_string())?
            .iter()
            .map(|(id, rec)| (id.clone(), rec.version.clone()))
            .collect();
        let packs = manifest.map(|m| m.packs).unwrap_or_default();
        Ok(packs
            .into_iter()
            .map(|p| {
                let installed_version = installed.get(&p.pack_id).cloned();
                InstallablePack {
                    installed: installed_version.is_some(),
                    installed_version,
                    pack_id: p.pack_id,
                    name: p.name,
                    version: p.version,
                    install_url: p.install_url,
                    sha256: p.sha256,
                }
            })
            .collect())
    }

    /// 从 feed 安装指定 pack：按 installUrl 下载 zip → SHA-256 校验 → 落临时 → 复用 install_pack。
    /// 默认 Installed（与本地安装一致，需显式启用→切 Managed→provision 产物）。
    pub fn install_pack_from_feed(&self, pack_id: &str) -> Result<PackInstallResult, String> {
        let app_data = self.require_app_data_dir()?;
        let cfg = crate::managed_dependency::feed::load_feed(&app_data);
        let manifest = crate::managed_dependency::feed::fetch_manifest(
            &cfg,
            crate::managed_dependency::store::fetch_manifest_text,
        )?
        .ok_or_else(|| "extension_host: no feed manifest configured".to_string())?;
        let pack = manifest
            .packs
            .iter()
            .find(|p| p.pack_id == pack_id)
            .ok_or_else(|| format!("pack '{pack_id}' not in feed manifest"))?;
        let bytes = crate::managed_dependency::store::download_verify_url_with_fallback(
            &pack.install_url,
            pack.install_fallback_url.as_deref(),
            &pack.sha256,
            &cfg.allow_hosts,
            crate::managed_dependency::store::http_get_bytes,
        )?;
        let tmp_dir =
            std::env::temp_dir().join(format!("afm-pack-install-{}", std::process::id()));
        fs::create_dir_all(&tmp_dir)
            .map_err(|err| format!("extension_host: pack tmp dir: {err}"))?;
        let zip =
            crate::managed_dependency::store::materialize(&tmp_dir, &format!("{pack_id}.zip"), &bytes)?;
        let result = self.install_pack(&zip);
        let _ = fs::remove_dir_all(&tmp_dir);
        Ok(result)
    }

    pub fn list_packs(&self) -> Result<Vec<InstalledPackRecord>, String> {
        let packs = self.packs.lock().map_err(|err| err.to_string())?;
        let mut list: Vec<_> = packs.values().cloned().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(list)
    }

    pub fn set_pack_enabled(&self, id: &str, enabled: bool) -> Result<(), String> {
        let app_data = self.require_app_data_dir()?;
        {
            let packs = self.packs.lock().map_err(|err| err.to_string())?;
            let pack = packs
                .get(id)
                .ok_or_else(|| format!("Unknown Feature Pack: {id}"))?;
            if enabled && pack.last_error.is_some() {
                return Err(format!(
                    "Feature Pack '{id}' has no valid manifest and cannot be enabled"
                ));
            }
        }
        if enabled {
            let contribs = self
                .pack_contributions
                .lock()
                .map_err(|err| err.to_string())?;
            if !contribs.contains_key(id) {
                return Err(format!(
                    "Feature Pack '{id}' has no valid manifest and cannot be enabled"
                ));
            }
        }
        let root = pack_fs::packs_root(&app_data);
        pack_fs::write_enabled_state(&root, id, enabled)?;
        let mut packs = self.packs.lock().map_err(|err| err.to_string())?;
        let pack = packs
            .get_mut(id)
            .ok_or_else(|| format!("Unknown Feature Pack: {id}"))?;
        pack.state = if enabled {
            ExtensionRuntimeState::Enabled
        } else {
            ExtensionRuntimeState::Disabled
        };
        if enabled {
            pack.last_error = None;
        }
        let pack_snapshot = pack.clone();
        drop(packs);
        // 1:1 托管记录同步（Q21）：仅对「声明 sidecars 且有 spawnSidecar 权限」的 pack 建/销记录。
        // 尽力而为——注册失败不阻断启用结果（缺失记录时投影面自然回退 config-mode 展示）。
        let declares_sidecars = self
            .pack_contributions
            .lock()
            .map(|contribs| {
                contribs
                    .get(id)
                    .map(|contrib| !contrib.sidecars.is_empty())
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        if declares_sidecars
            && pack_snapshot
                .permissions
                .contains(&ExtensionPermission::SpawnSidecar)
        {
            let sync = if enabled {
                // ADR 0022：启用时按 feed 元清单决定该 pack 是否 managed。
                // 目标平台须有原生产物且宿主具备 sidecar 产物能力，才标 Managed；
                // 否则（未配置 / 清单不可达 / 无本平台 entry / 能力关闭）静默降级
                // Config。`resolve_managed_binding` 内部按 artifact_platform_key()
                // 精确选条目——多平台 feed 下 macOS 绝不会选中 Windows .exe。
                let host_can_run_feed_artifacts =
                    crate::platform::current().capabilities().sidecar_native_artifacts;
                let feed_cfg = crate::managed_dependency::feed::load_feed(&app_data);
                let manifest = crate::managed_dependency::feed::fetch_manifest(
                    &feed_cfg,
                    crate::managed_dependency::store::fetch_manifest_text,
                )
                .unwrap_or(None);
                let (mode, base_url, declared_version) = match manifest.as_ref()
                    .filter(|_| host_can_run_feed_artifacts)
                    .and_then(|m| crate::managed_dependency::feed::resolve_managed_binding(Some(m), id))
                {
                    Some(b) => (
                        crate::managed_dependency::registry::DependencyMode::Managed,
                        b.base_url,
                        b.declared_version,
                    ),
                    None => (
                        crate::managed_dependency::registry::DependencyMode::Config,
                        String::new(),
                        String::new(),
                    ),
                };
                crate::managed_dependency::registry::register_for_enabled_pack(
                    &app_data,
                    &pack_snapshot,
                    id,
                    mode,
                    base_url,
                    crate::managed_dependency::registry::CredentialSource::None,
                    declared_version,
                )
                .map(|_| ())
            } else {
                crate::managed_dependency::registry::unregister_for_pack(&app_data, id).map(|_| ())
            };
            if let Err(err) = sync {
                eprintln!("managed_dependency sync for pack '{id}' failed: {err}");
            }
        }
        // 启用/禁用改变 pack 集合，重算 config schema overlay。
        self.sync_config_overlay();
        Ok(())
    }

    pub fn uninstall_pack(&self, id: &str) -> Result<(), String> {
        let app_data = self.require_app_data_dir()?;
        // 卸载前先终止该 Pack 自启的 sidecar 进程，否则 Windows 文件锁导致目录删除失败 (os error 5)。
        if let Ok(Some(record)) =
            crate::managed_dependency::registry::load_record(&app_data, id)
        {
            if let Some(pid) = record.spawned_pid {
                crate::platform::kill_process_tree(pid);
                // 给 OS 短暂时间释放文件句柄。
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        }
        pack_fs::uninstall_pack(&app_data, id)?;
        self.packs
            .lock()
            .map_err(|err| err.to_string())?
            .remove(id);
        self.pack_contributions
            .lock()
            .map_err(|err| err.to_string())?
            .remove(id);
        // 1:1 单写出口（Q21）：卸载 pack 必销其托管记录。尽力而为。
        if let Err(err) = crate::managed_dependency::registry::unregister_for_pack(&app_data, id) {
            eprintln!("managed_dependency unregister for pack '{id}' failed: {err}");
        }
        // 卸载移除 pack，重算 config schema overlay。
        self.sync_config_overlay();
        Ok(())
    }

    /// 仅聚合 Enabled Pack 的贡献。
    pub fn list_contributions(&self) -> Result<AggregatedContributions, String> {
        let packs = self.packs.lock().map_err(|err| err.to_string())?;
        let contribs = self
            .pack_contributions
            .lock()
            .map_err(|err| err.to_string())?;
        let mut aggregated = AggregatedContributions::default();
        for (id, pack) in packs.iter() {
            if pack.state != ExtensionRuntimeState::Enabled {
                continue;
            }
            let Some(contrib) = contribs.get(id) else {
                continue;
            };
            aggregated.side_nav.extend(contrib.side_nav.iter().cloned());
            aggregated.routes.extend(contrib.routes.iter().cloned());
            aggregated
                .commands
                .extend(contrib.commands.iter().cloned());
            aggregated
                .sidecars
                .extend(contrib.sidecars.iter().cloned());
            aggregated
                .extensions_sections
                .extend(contrib.extensions_sections.iter().cloned());
            // 搜索条目：聚合时写入 packId / packName（副标题显示 Pack 名）。
            for entry in &contrib.search {
                let mut item = entry.clone();
                item.pack_id = Some(id.clone());
                item.pack_name = Some(pack.name.clone());
                aggregated.search.push(item);
            }
            // 配置项：聚合时写入 packId 并把 settingKey 解析为生效 key（显式优先，否则
            // `ext.<packId>.<id>`），与 schema overlay 共用同一真相源，前端直接据此读写。
            for entry in &contrib.config {
                let mut item = entry.clone();
                item.pack_id = Some(id.clone());
                item.pack_name = Some(pack.name.clone());
                item.setting_key = Some(crate::settings::overlay::effective_setting_key(id, entry));
                aggregated.config.push(item);
            }
        }
        aggregated
            .side_nav
            .sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
        aggregated
            .search
            .sort_by(|a, b| a.id.cmp(&b.id).then(a.route.cmp(&b.route)));
        aggregated
            .config
            .sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
        Ok(aggregated)
    }

    /// 由当前已启用 Pack 的 `config[]` 声明重算 settings schema overlay，使 pack 配置项
    /// 进入 `definition_map()`、从而通过 `sanitize_document` 落盘。尽力而为——失败仅记日志。
    pub fn sync_config_overlay(&self) {
        let entries = (|| -> Result<Vec<(String, Vec<ConfigContribution>)>, String> {
            let packs = self.packs.lock().map_err(|err| err.to_string())?;
            let contribs = self.pack_contributions.lock().map_err(|err| err.to_string())?;
            let mut entries = Vec::new();
            for (id, pack) in packs.iter() {
                if pack.state != ExtensionRuntimeState::Enabled {
                    continue;
                }
                if let Some(contrib) = contribs.get(id) {
                    if !contrib.config.is_empty() {
                        entries.push((id.clone(), contrib.config.clone()));
                    }
                }
            }
            Ok(entries)
        })();
        match entries {
            Ok(entries) => crate::settings::overlay::sync_pack_configs(entries),
            Err(err) => eprintln!("extension_host sync_config_overlay failed: {err}"),
        }
    }

    /// 取指定 Pack 清单声明的 `config[]`（无论启用与否，供托管 spawn 注入 env 时解析）。
    pub fn pack_config_contributions(&self, id: &str) -> Vec<ConfigContribution> {
        self.pack_contributions
            .lock()
            .ok()
            .and_then(|contribs| contribs.get(id).map(|contrib| contrib.config.clone()))
            .unwrap_or_default()
    }

    fn require_app_data_dir(&self) -> Result<PathBuf, String> {
        self.app_data_dir
            .lock()
            .map_err(|err| err.to_string())?
            .clone()
            .ok_or_else(|| "Feature Pack host has not loaded app data directory".to_string())
    }

    fn install_pack_inner(&self, source_path: &Path) -> Result<InstalledPackRecord, String> {
        let app_data = self.require_app_data_dir()?;
        if source_path.is_dir() {
            return self.install_from_pack_directory(&app_data, source_path);
        }
        if !source_path.is_file() {
            return Err(format!(
                "Feature Pack source not found: {}",
                source_path.display()
            ));
        }
        let root = pack_fs::packs_root(&app_data);
        fs::create_dir_all(&root)
            .map_err(|err| format!("Failed to create feature-packs directory: {err}"))?;
        let staging = root.join(format!(
            ".extract-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        ));
        if staging.exists() {
            let _ = fs::remove_dir_all(&staging);
        }
        let result = (|| {
            pack_fs::extract_pack_zip(source_path, &staging)?;
            let pack_root = pack_fs::find_pack_root(&staging)?;
            self.install_from_pack_directory(&app_data, &pack_root)
        })();
        let _ = fs::remove_dir_all(&staging);
        result
    }

    fn install_from_pack_directory(
        &self,
        app_data: &Path,
        source_dir: &Path,
    ) -> Result<InstalledPackRecord, String> {
        let manifest = pack_fs::read_pack_json(source_dir)?;
        let permissions = validate_pack_permissions(&manifest)?;
        let installed = pack_fs::install_from_directory(app_data, source_dir)?;
        self.remember_installed_pack(app_data, installed, permissions)
    }

    fn remember_installed_pack(
        &self,
        app_data: &Path,
        manifest: FeaturePackManifest,
        permissions: Vec<ExtensionPermission>,
    ) -> Result<InstalledPackRecord, String> {
        let dest = pack_fs::pack_dir(&pack_fs::packs_root(app_data), &manifest.id);
        let enabled = pack_fs::load_state(&pack_fs::packs_root(app_data))?
            .enabled
            .get(&manifest.id)
            .copied()
            .unwrap_or(false);
        let record = InstalledPackRecord {
            id: manifest.id.clone(),
            name: manifest.name,
            version: manifest.version,
            state: if enabled {
                ExtensionRuntimeState::Enabled
            } else {
                ExtensionRuntimeState::Installed
            },
            permissions,
            install_path: dest.to_string_lossy().into_owned(),
            last_error: None,
        };
        self.packs
            .lock()
            .map_err(|err| err.to_string())?
            .insert(record.id.clone(), record.clone());
        self.pack_contributions
            .lock()
            .map_err(|err| err.to_string())?
            .insert(manifest.id, manifest.contributions);
        Ok(record)
    }

    fn load_pack_record(
        &self,
        pack_path: &Path,
        state: &pack_fs::PackStateFile,
    ) -> Result<(InstalledPackRecord, FeaturePackContributions), String> {
        let manifest = pack_fs::read_pack_json(pack_path)?;
        let permissions = validate_pack_permissions(&manifest)?;
        let runtime_state = match state.enabled.get(&manifest.id) {
            Some(true) => ExtensionRuntimeState::Enabled,
            Some(false) => ExtensionRuntimeState::Disabled,
            None => ExtensionRuntimeState::Installed,
        };
        Ok((
            InstalledPackRecord {
                id: manifest.id,
                name: manifest.name,
                version: manifest.version,
                state: runtime_state,
                permissions,
                install_path: pack_path.to_string_lossy().into_owned(),
                last_error: None,
            },
            manifest.contributions,
        ))
    }
}

fn validate_pack_permissions(
    manifest: &FeaturePackManifest,
) -> Result<Vec<ExtensionPermission>, String> {
    let permissions = parse_permissions(&manifest.permissions)?;
    if !manifest.contributions.sidecars.is_empty()
        && !requires(&permissions, ExtensionPermission::SpawnSidecar)
    {
        return Err(
            "sidecars declared but permission 'spawnSidecar' was not requested".to_string(),
        );
    }
    // 搜索面板静态条目与命令/侧栏同属 UI 贡献，声明必须申请 uiCommands。
    if !manifest.contributions.search.is_empty()
        && !requires(&permissions, ExtensionPermission::UiCommands)
    {
        return Err(
            "search declared but permission 'uiCommands' was not requested".to_string(),
        );
    }
    Ok(permissions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_starts_empty_for_pack_host() {
        let host = ExtensionHost::new();
        let status = host.status().unwrap();
        assert_eq!(status.api_version, FEATURE_PACK_API_VERSION);
        assert_eq!(status.installed_count, 0);
        assert_eq!(status.enabled_count, 0);
        assert_eq!(status.crashed_count, 0);
    }

    fn write_side_nav_pack(dir: &std::path::Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("pack.json"),
            r#"{
                "id":"demo.pack",
                "name":"Demo",
                "version":"0.1.0",
                "apiVersion":"feature-pack.1",
                "permissions":[],
                "contributions":{
                    "sideNav":[
                        {
                            "id":"nav-tools",
                            "labelKey":"nav.tools",
                            "to":"/tools",
                            "order":10
                        }
                    ]
                }
            }"#,
        )
        .unwrap();
    }

    #[test]
    fn list_contributions_includes_side_nav_after_enable() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src-pack");
        write_side_nav_pack(&src);
        let app = tmp.path().join("appdata");
        let host = ExtensionHost::new();
        host.load_from_disk(&app).unwrap();
        let installed = host.install_pack(&src);
        assert!(installed.success, "{:?}", installed.error);
        host.set_pack_enabled("demo.pack", true).unwrap();
        let contribs = host.list_contributions().unwrap();
        assert!(
            contribs.side_nav.iter().any(|item| item.id == "nav-tools"),
            "enabled pack sideNav should be aggregated, got {:?}",
            contribs.side_nav
        );
    }

    fn write_search_pack(dir: &std::path::Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("pack.json"),
            r#"{
                "id":"demo.pack",
                "name":"Demo",
                "version":"0.1.0",
                "apiVersion":"feature-pack.1",
                "permissions":["uiCommands"],
                "contributions":{
                    "search":[
                        {
                            "id":"open-demo",
                            "labelKey":"demo.openEntry",
                            "keywords":["demo"],
                            "route":"/navigator"
                        }
                    ]
                }
            }"#,
        )
        .unwrap();
    }

    #[test]
    fn list_contributions_aggregates_search_with_pack_info() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src-pack");
        write_search_pack(&src);
        let app = tmp.path().join("appdata");
        let host = ExtensionHost::new();
        host.load_from_disk(&app).unwrap();
        let installed = host.install_pack(&src);
        assert!(installed.success, "{:?}", installed.error);
        // 未启用：不聚合。
        assert!(host.list_contributions().unwrap().search.is_empty());
        host.set_pack_enabled("demo.pack", true).unwrap();
        let contribs = host.list_contributions().unwrap();
        assert_eq!(contribs.search.len(), 1);
        let entry = &contribs.search[0];
        assert_eq!(entry.id, "open-demo");
        assert_eq!(entry.route, "/navigator");
        assert_eq!(entry.pack_id.as_deref(), Some("demo.pack"));
        assert_eq!(entry.pack_name.as_deref(), Some("Demo"));
    }

    #[test]
    fn search_contribution_requires_ui_commands_permission() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src-pack");
        write_search_pack(&src);
        // 去掉 uiCommands 权限后安装必须失败。
        let text = std::fs::read_to_string(src.join("pack.json"))
            .unwrap()
            .replace("\"permissions\":[\"uiCommands\"]", "\"permissions\":[]");
        std::fs::write(src.join("pack.json"), text).unwrap();
        let app = tmp.path().join("appdata");
        let host = ExtensionHost::new();
        host.load_from_disk(&app).unwrap();
        let installed = host.install_pack(&src);
        assert!(!installed.success);
        assert!(
            installed
                .error
                .unwrap()
                .contains("uiCommands' was not requested"),
            "install without uiCommands should fail"
        );
    }

    #[test]
    fn list_contributions_excludes_disabled_packs() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src-pack");
        write_side_nav_pack(&src);
        let app = tmp.path().join("appdata");
        let host = ExtensionHost::new();
        host.load_from_disk(&app).unwrap();
        assert!(host.install_pack(&src).success);
        let contribs = host.list_contributions().unwrap();
        assert!(contribs.side_nav.is_empty());
        host.set_pack_enabled("demo.pack", true).unwrap();
        host.set_pack_enabled("demo.pack", false).unwrap();
        let contribs = host.list_contributions().unwrap();
        assert!(contribs.side_nav.is_empty());
    }

    #[test]
    fn install_pack_from_zip_then_enable_contributes() {
        use std::io::Write;
        use zip::write::SimpleFileOptions;
        use zip::CompressionMethod;

        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src-pack");
        write_side_nav_pack(&src);
        let zip_path = tmp.path().join("demo.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("pack.json", options).unwrap();
        zip.write_all(&std::fs::read(src.join("pack.json")).unwrap())
            .unwrap();
        zip.finish().unwrap();

        let app = tmp.path().join("appdata");
        let host = ExtensionHost::new();
        host.load_from_disk(&app).unwrap();
        let installed = host.install_pack(&zip_path);
        assert!(installed.success, "{:?}", installed.error);
        host.set_pack_enabled("demo.pack", true).unwrap();
        let contribs = host.list_contributions().unwrap();
        assert!(contribs.side_nav.iter().any(|item| item.id == "nav-tools"));
    }

    #[test]
    fn load_from_disk_restores_enabled_contributions() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src-pack");
        write_side_nav_pack(&src);
        let app = tmp.path().join("appdata");
        let host = ExtensionHost::new();
        host.load_from_disk(&app).unwrap();
        assert!(host.install_pack(&src).success);
        host.set_pack_enabled("demo.pack", true).unwrap();

        let reloaded = ExtensionHost::new();
        reloaded.load_from_disk(&app).unwrap();
        let contribs = reloaded.list_contributions().unwrap();
        assert!(contribs.side_nav.iter().any(|item| item.id == "nav-tools"));
        assert_eq!(reloaded.list_packs().unwrap()[0].id, "demo.pack");
    }

    fn write_config_pack(dir: &std::path::Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("pack.json"),
            r#"{
                "id":"demo.pack",
                "name":"Demo",
                "version":"0.1.0",
                "apiVersion":"feature-pack.1",
                "permissions":[],
                "contributions":{
                    "config":[
                        {
                            "id":"api-key",
                            "title":"API Key",
                            "kind":"secret",
                            "env":"DEMO_API_KEY",
                            "order":5
                        }
                    ]
                }
            }"#,
        )
        .unwrap();
    }

    #[test]
    fn list_contributions_aggregates_config_with_effective_key() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src-pack");
        write_config_pack(&src);
        let app = tmp.path().join("appdata");
        let host = ExtensionHost::new();
        host.load_from_disk(&app).unwrap();
        assert!(host.install_pack(&src).success);
        host.set_pack_enabled("demo.pack", true).unwrap();
        let contribs = host.list_contributions().unwrap();
        assert_eq!(contribs.config.len(), 1);
        let item = &contribs.config[0];
        assert_eq!(item.id, "api-key");
        assert_eq!(item.pack_id.as_deref(), Some("demo.pack"));
        assert_eq!(item.pack_name.as_deref(), Some("Demo"));
        // 缺省 settingKey → 宿主解析为 ext.<packId>.<id> 命名空间生效 key。
        assert_eq!(item.setting_key.as_deref(), Some("ext.demo.pack.api-key"));
        assert_eq!(item.env.as_deref(), Some("DEMO_API_KEY"));
    }
}
