// SPDX-License-Identifier: GPL-3.0-or-later

use super::permissions::{parse_permissions, requires};
use super::types::{
    ExtensionHostStatus, ExtensionManifestVNext, ExtensionPermission, ExtensionRuntimeState,
    FsProviderContribution, InstallResult, InstalledExtensionRecord, MarketplaceCatalog,
    UiCommandRequest, UiCommandResult, EXTENSION_HOST_API_VERSION,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

/// In-process ExtensionHost vNext.
/// UI command failures mark the extension crashed but never panic the host
/// (Navigator / ProviderRegistry stay up).
#[derive(Default)]
pub struct ExtensionHost {
    extensions: Mutex<BTreeMap<String, InstalledExtensionRecord>>,
    /// extension_id → backend ids contributed as FS provider hooks (metadata only).
    fs_hooks: Mutex<BTreeMap<String, BTreeSet<String>>>,
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
}
