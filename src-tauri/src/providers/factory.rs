// SPDX-License-Identifier: GPL-3.0-or-later

use super::browser::BrowserProvider;
use super::rclone::RcloneProvider;
use super::registry::ProviderRegistry;
use std::sync::Arc;

/// 内置 registry：rclone + browser。不注册 `local`（本机仍走 Navigator）。
/// knowledge / downloads / prowlarr 仅在 awam-file 网关在线时由网关发现。
pub fn build_builtin_registry() -> ProviderRegistry {
    let registry = ProviderRegistry::new();
    registry
        .register(Arc::new(RcloneProvider::system_default()))
        .expect("register rclone");
    registry
        .register(Arc::new(BrowserProvider::system_default()))
        .expect("register browser");
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::browser::BrowserProvider;
    use crate::providers::rclone::{CommandOutput, RcloneProvider, RcloneRunner};
    use std::time::Duration;

    struct EmptyRclone;

    impl RcloneRunner for EmptyRclone {
        fn run(&self, args: &[&str], _timeout: Duration) -> Result<CommandOutput, String> {
            if args.first() == Some(&"listremotes") {
                return Ok(CommandOutput {
                    status_code: 0,
                    stdout: "demo:\n".to_string(),
                    stderr: String::new(),
                });
            }
            Ok(CommandOutput {
                status_code: 0,
                stdout: "[]".to_string(),
                stderr: String::new(),
            })
        }
    }

    #[test]
    fn builtin_has_rclone_browser_not_local_or_gateway_only() {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-providers-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let registry = ProviderRegistry::new();
        registry
            .register(Arc::new(RcloneProvider::new(Arc::new(EmptyRclone))))
            .unwrap();
        registry
            .register(Arc::new(BrowserProvider::with_user_data_roots(
                dir.join("edge-missing"),
                dir.join("chrome-missing"),
            )))
            .unwrap();

        assert!(registry.contains("rclone"));
        assert!(registry.contains("browser"));
        assert!(!registry.contains("local"));
        assert!(!registry.contains("knowledge"));
        assert!(!registry.contains("downloads"));
        assert!(!registry.contains("prowlarr"));

        let backends = registry.backends().unwrap();
        assert!(backends.iter().any(|b| b.id == "rclone" && !b.interface_only));
        assert_eq!(backends.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
