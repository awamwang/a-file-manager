// SPDX-License-Identifier: GPL-3.0-or-later

use super::browser::BrowserProvider;
use super::rclone::RcloneProvider;
use super::registry::ProviderRegistry;
use std::sync::Arc;

/// 内置 registry：rclone + browser 离线占位。不注册 `local`（本机仍走 Navigator）。
/// knowledge / downloads / prowlarr / 真实 browser 书签树仅在 awam-file 网关在线时由网关发现。
/// browser 内置项不解析 Bookmarks，只在网关离线时提示启动网关。
pub fn build_builtin_registry() -> ProviderRegistry {
    let registry = ProviderRegistry::new();
    registry
        .register(Arc::new(RcloneProvider::system_default()))
        .expect("register rclone");
    registry
        .register(Arc::new(BrowserProvider::system_default()))
        .expect("register browser offline stub");
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn builtin_has_rclone_browser_stub_not_local_or_gateway_only() {
        let registry = ProviderRegistry::new();
        registry
            .register(Arc::new(RcloneProvider::new(Arc::new(EmptyRclone))))
            .unwrap();
        registry
            .register(Arc::new(BrowserProvider::system_default()))
            .unwrap();

        assert!(registry.contains("rclone"));
        assert!(registry.contains("browser"));
        assert!(!registry.contains("local"));
        assert!(!registry.contains("knowledge"));
        assert!(!registry.contains("downloads"));
        assert!(!registry.contains("prowlarr"));

        let backends = registry.backends().unwrap();
        assert!(backends.iter().any(|b| b.id == "rclone" && !b.interface_only));
        let browser = backends.iter().find(|b| b.id == "browser").unwrap();
        assert!(!browser.healthy);
        assert!(browser
            .detail
            .as_deref()
            .is_some_and(|d| d.contains("awam-file")));
        assert_eq!(backends.len(), 2);
    }
}
