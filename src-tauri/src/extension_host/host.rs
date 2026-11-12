// SPDX-License-Identifier: GPL-3.0-or-later

use super::pack_fs;
use super::permissions::{parse_permissions, requires};
use super::types::{
    AggregatedContributions, ExtensionHostStatus, ExtensionManifestVNext, ExtensionPermission,
    ExtensionRuntimeState, FeaturePackContributions, FeaturePackManifest, FsProviderContribution,
    InstallResult, InstalledExtensionRecord, InstalledPackRecord, MarketplaceCatalog,
    PackInstallResult, UiCommandRequest, UiCommandResult, EXTENSION_HOST_API_VERSION,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// In-process ExtensionHost vNext.
/// UI command failures mark the extension crashed but never panic the host
/// (Navigator / ProviderRegistry stay up).
#[derive(Default)]
pub struct ExtensionHost {
    extensions: Mutex<BTreeMap<String, InstalledExtensionRecord>>,
    /// extension_id → backend ids contributed as FS provider hooks (metadata only).
    fs_hooks: Mutex<BTreeMap<String, BTreeSet<String>>>,
    /// 本机 Feature Pack 安装根对应的 APPDATA 目录。
    app_data_dir: Mutex<Option<PathBuf>>,
    packs: Mutex<BTreeMap<String, InstalledPackRecord>>,
    pack_contributions: Mutex<BTreeMap<String, FeaturePackContributions>>,
}

impl ExtensionHost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn marketplace_catalog(&self) -> MarketplaceCatalog {
        MarketplaceCatalog {
            api_version: EXTENSION_HOST_API_VERSION.to_string(),
            extensions: Vec::new(),
            empty: true,
            detail: "Marketplace is intentionally empty in ExtensionHost vNext; legacy Sigma marketplace packages are not migrated and no remote catalog is fetched".to_string(),
            fetches_remote: false,
        }
    }

    pub fn status(&self) -> Result<ExtensionHostStatus, String> {
        let extensions = self.extensions.lock().map_err(|err| err.to_string())?;
        let hooks = self.fs_hooks.lock().map_err(|err| err.to_string())?;
        let installed_count = extensions.len();
        let enabled_count = extensions
            .values()
            .filter(|ext| ext.state == ExtensionRuntimeState::Enabled)
            .count();
        let crashed_count = extensions
            .values()
            .filter(|ext| ext.state == ExtensionRuntimeState::Crashed)
            .count();
        let fs_provider_hooks = hooks.values().map(BTreeSet::len).sum();
        Ok(ExtensionHostStatus {
            api_version: EXTENSION_HOST_API_VERSION.to_string(),
            installed_count,
            enabled_count,
            crashed_count,
            marketplace_empty: true,
            fetches_remote: false,
            fs_provider_hooks,
        })
    }

    pub fn list_installed(&self) -> Result<Vec<InstalledExtensionRecord>, String> {
        let extensions = self.extensions.lock().map_err(|err| err.to_string())?;
        let mut list: Vec<_> = extensions.values().cloned().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(list)
    }

    pub fn install_from_manifest(&self, manifest: ExtensionManifestVNext) -> InstallResult {
        match self.install_from_manifest_inner(manifest) {
            Ok(extension) => InstallResult {
                success: true,
                extension: Some(extension),
                error: None,
            },
            Err(error) => InstallResult {
                success: false,
                extension: None,
                error: Some(error),
            },
        }
    }

    fn install_from_manifest_inner(
        &self,
        manifest: ExtensionManifestVNext,
    ) -> Result<InstalledExtensionRecord, String> {
        if manifest.id.trim().is_empty() {
            return Err("Extension id must not be empty".to_string());
        }
        if manifest.api_version != EXTENSION_HOST_API_VERSION {
            return Err(format!(
                "Unsupported apiVersion '{}'; vNext host requires '{EXTENSION_HOST_API_VERSION}' (legacy marketplace packages are not supported)",
                manifest.api_version
            ));
        }

        let permissions = parse_permissions(&manifest.permissions)?;
        if !manifest.fs_providers.is_empty()
            && !requires(&permissions, ExtensionPermission::FsProvider)
        {
            return Err(
                "fs_providers declared but permission 'fsProvider' was not requested".to_string(),
            );
        }
        if !manifest.ui_commands.is_empty()
            && !requires(&permissions, ExtensionPermission::UiCommands)
        {
            return Err(
                "ui_commands declared but permission 'uiCommands' was not requested".to_string(),
            );
        }

        let record = InstalledExtensionRecord {
            id: manifest.id.clone(),
            name: manifest.name,
            version: manifest.version,
            state: ExtensionRuntimeState::Installed,
            permissions,
            fs_providers: manifest.fs_providers.clone(),
            ui_commands: manifest.ui_commands,
            last_error: None,
        };

        let mut extensions = self.extensions.lock().map_err(|err| err.to_string())?;
        if extensions.contains_key(&manifest.id) {
            return Err(format!("Extension already installed: {}", manifest.id));
        }
        extensions.insert(manifest.id.clone(), record.clone());

        if !manifest.fs_providers.is_empty() {
            let mut hooks = self.fs_hooks.lock().map_err(|err| err.to_string())?;
            let set = hooks.entry(manifest.id.clone()).or_default();
            for backend_id in &manifest.fs_providers {
                set.insert(backend_id.clone());
            }
        }

        Ok(record)
    }

    pub fn set_enabled(
        &self,
        extension_id: &str,
        enabled: bool,
    ) -> Result<InstalledExtensionRecord, String> {
        let mut extensions = self.extensions.lock().map_err(|err| err.to_string())?;
        let extension = extensions
            .get_mut(extension_id)
            .ok_or_else(|| format!("Unknown extension: {extension_id}"))?;
        extension.state = if enabled {
            ExtensionRuntimeState::Enabled
        } else {
            ExtensionRuntimeState::Disabled
        };
        extension.last_error = None;
        Ok(extension.clone())
    }

    pub fn uninstall(&self, extension_id: &str) -> Result<bool, String> {
        let mut extensions = self.extensions.lock().map_err(|err| err.to_string())?;
        let removed = extensions.remove(extension_id).is_some();
        let mut hooks = self.fs_hooks.lock().map_err(|err| err.to_string())?;
        hooks.remove(extension_id);
        Ok(removed)
    }

    pub fn list_fs_provider_hooks(&self) -> Result<Vec<FsProviderContribution>, String> {
        let hooks = self.fs_hooks.lock().map_err(|err| err.to_string())?;
        let mut contributions = Vec::new();
        for (extension_id, backends) in hooks.iter() {
            for backend_id in backends {
                contributions.push(FsProviderContribution {
                    extension_id: extension_id.clone(),
                    backend_id: backend_id.clone(),
                    registered: true,
                });
            }
        }
        contributions.sort_by(|a, b| {
            a.extension_id
                .cmp(&b.extension_id)
                .then(a.backend_id.cmp(&b.backend_id))
        });
        Ok(contributions)
    }

    pub fn register_fs_provider_hook(
        &self,
        extension_id: &str,
        backend_id: &str,
    ) -> Result<FsProviderContribution, String> {
        {
            let extensions = self.extensions.lock().map_err(|err| err.to_string())?;
            let extension = extensions
                .get(extension_id)
                .ok_or_else(|| format!("Unknown extension: {extension_id}"))?;
            if !requires(&extension.permissions, ExtensionPermission::FsProvider) {
                return Err("Extension lacks fsProvider permission".to_string());
            }
        }

        let mut hooks = self.fs_hooks.lock().map_err(|err| err.to_string())?;
        hooks
            .entry(extension_id.to_string())
            .or_default()
            .insert(backend_id.to_string());
        Ok(FsProviderContribution {
            extension_id: extension_id.to_string(),
            backend_id: backend_id.to_string(),
            registered: true,
        })
    }

    /// Dispatch a JS/UI command. Failures isolate to the extension (host stays alive).
    pub fn dispatch_ui_command(&self, request: UiCommandRequest) -> Result<UiCommandResult, String> {
        let mut extensions = self.extensions.lock().map_err(|err| err.to_string())?;
        let extension = extensions
            .get_mut(&request.extension_id)
            .ok_or_else(|| format!("Unknown extension: {}", request.extension_id))?;

        if !requires(&extension.permissions, ExtensionPermission::UiCommands) {
            return Ok(UiCommandResult {
                ok: false,
                extension_id: request.extension_id,
                command_id: request.command_id,
                payload: None,
                error: Some("Extension lacks uiCommands permission".to_string()),
                isolated_failure: false,
            });
        }

        if extension.state == ExtensionRuntimeState::Disabled {
            return Ok(UiCommandResult {
                ok: false,
                extension_id: request.extension_id,
                command_id: request.command_id,
                payload: None,
                error: Some("Extension is disabled".to_string()),
                isolated_failure: false,
            });
        }

        if extension.state == ExtensionRuntimeState::Crashed {
            return Ok(UiCommandResult {
                ok: false,
                extension_id: request.extension_id,
                command_id: request.command_id,
                payload: None,
                error: Some(
                    extension
                        .last_error
                        .clone()
                        .unwrap_or_else(|| "Extension previously crashed".to_string()),
                ),
                isolated_failure: true,
            });
        }

        if !extension
            .ui_commands
            .iter()
            .any(|id| id == &request.command_id)
        {
            return Ok(UiCommandResult {
                ok: false,
                extension_id: request.extension_id,
                command_id: request.command_id,
                payload: None,
                error: Some("Command not declared in extension manifest".to_string()),
                isolated_failure: false,
            });
        }

        if request.command_id == "__crash__" || request.args.get("forceCrash") == Some(&json!(true))
        {
            extension.state = ExtensionRuntimeState::Crashed;
            extension.last_error = Some("Isolated extension worker fault (simulated)".to_string());
            return Ok(UiCommandResult {
                ok: false,
                extension_id: request.extension_id,
                command_id: request.command_id,
                payload: None,
                error: Some("Isolated extension worker fault (simulated)".to_string()),
                isolated_failure: true,
            });
        }

        if extension.state == ExtensionRuntimeState::Installed {
            extension.state = ExtensionRuntimeState::Enabled;
        }

        Ok(UiCommandResult {
            ok: true,
            extension_id: request.extension_id.clone(),
            command_id: request.command_id.clone(),
            payload: Some(Value::Object(
                request
                    .args
                    .into_iter()
                    .map(|(key, value)| (key, value))
                    .collect(),
            )),
            error: None,
            isolated_failure: false,
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
        Ok(())
    }

    pub fn uninstall_pack(&self, id: &str) -> Result<(), String> {
        let app_data = self.require_app_data_dir()?;
        pack_fs::uninstall_pack(&app_data, id)?;
        self.packs
            .lock()
            .map_err(|err| err.to_string())?
            .remove(id);
        self.pack_contributions
            .lock()
            .map_err(|err| err.to_string())?
            .remove(id);
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
        }
        aggregated
            .side_nav
            .sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
        Ok(aggregated)
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
    Ok(permissions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> ExtensionManifestVNext {
        ExtensionManifestVNext {
            id: "demo.ext".to_string(),
            name: "Demo".to_string(),
            version: "0.1.0".to_string(),
            api_version: EXTENSION_HOST_API_VERSION.to_string(),
            description: "test".to_string(),
            permissions: vec!["uiCommands".to_string(), "fsProvider".to_string()],
            fs_providers: vec!["demo-fs".to_string()],
            ui_commands: vec!["hello".to_string(), "__crash__".to_string()],
        }
    }

    #[test]
    fn marketplace_is_empty_and_local_only() {
        let host = ExtensionHost::new();
        let catalog = host.marketplace_catalog();
        assert!(catalog.empty);
        assert!(catalog.extensions.is_empty());
        assert!(!catalog.fetches_remote);
        let status = host.status().unwrap();
        assert!(status.marketplace_empty);
        assert!(!status.fetches_remote);
        assert_eq!(status.installed_count, 0);
    }

    #[test]
    fn rejects_legacy_api_version() {
        let host = ExtensionHost::new();
        let mut manifest = sample_manifest();
        manifest.api_version = "1.0.0".to_string();
        let result = host.install_from_manifest(manifest);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("legacy"));
    }

    #[test]
    fn install_registers_fs_hook_and_ui_dispatch() {
        let host = ExtensionHost::new();
        let installed = host.install_from_manifest(sample_manifest());
        assert!(installed.success);

        let hooks = host.list_fs_provider_hooks().unwrap();
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].backend_id, "demo-fs");

        let result = host
            .dispatch_ui_command(UiCommandRequest {
                extension_id: "demo.ext".to_string(),
                command_id: "hello".to_string(),
                args: BTreeMap::from([("n".to_string(), json!(1))]),
            })
            .unwrap();
        assert!(result.ok);
        assert!(!result.isolated_failure);
    }

    #[test]
    fn ui_crash_isolates_without_killing_host() {
        let host = ExtensionHost::new();
        host.install_from_manifest(sample_manifest());
        let crash = host
            .dispatch_ui_command(UiCommandRequest {
                extension_id: "demo.ext".to_string(),
                command_id: "__crash__".to_string(),
                args: BTreeMap::new(),
            })
            .unwrap();
        assert!(!crash.ok);
        assert!(crash.isolated_failure);

        let status = host.status().unwrap();
        assert_eq!(status.crashed_count, 1);
        assert_eq!(status.installed_count, 1);
        assert!(host.marketplace_catalog().empty);
        assert_eq!(host.list_fs_provider_hooks().unwrap().len(), 1);
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
}
