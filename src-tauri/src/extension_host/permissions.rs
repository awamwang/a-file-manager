// SPDX-License-Identifier: GPL-3.0-or-later

use super::types::ExtensionPermission;

/// vNext allowlist — unknown permission strings are rejected at install.
pub fn parse_permissions(raw: &[String]) -> Result<Vec<ExtensionPermission>, String> {
    let mut parsed = Vec::new();
    for item in raw {
        match ExtensionPermission::parse(item) {
            Some(permission) => {
                if !parsed.contains(&permission) {
                    parsed.push(permission);
                }
            }
            None => {
                return Err(format!(
                    "Unknown permission '{item}' (vNext allowlist: uiCommands, fsProvider, network, spawnSidecar)"
                ));
            }
        }
    }
    Ok(parsed)
}

pub fn requires(permissions: &[ExtensionPermission], needed: ExtensionPermission) -> bool {
    permissions.contains(&needed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_permission() {
        let err =
            parse_permissions(&["uiCommands".to_string(), "shellExec".to_string()]).unwrap_err();
        assert!(err.contains("shellExec"));
    }

    #[test]
    fn parse_spawn_sidecar_permission() {
        assert_eq!(
            ExtensionPermission::parse("spawnSidecar"),
            Some(ExtensionPermission::SpawnSidecar)
        );
    }

    #[test]
    fn accepts_allowlist() {
        let parsed = parse_permissions(&[
            "uiCommands".to_string(),
            "fsProvider".to_string(),
            "network".to_string(),
        ])
        .unwrap();
        assert_eq!(parsed.len(), 3);
    }
}
