// SPDX-License-Identifier: GPL-3.0-or-later

use super::knowledge::KnowledgeProvider;
use super::rclone::RcloneProvider;
use super::registry::ProviderRegistry;
use super::stubs::InterfaceOnlyProvider;
use std::sync::Arc;

/// Day-1 registry: rclone + knowledge; downloads/prowlarr interface-only.
/// Does **not** register `local` (ADR 0001/0003).
pub fn build_day1_registry() -> ProviderRegistry {
    let registry = ProviderRegistry::new();
    registry
        .register(Arc::new(RcloneProvider::system_default()))
        .expect("register rclone");
    registry
        .register(Arc::new(KnowledgeProvider::from_env_or_default()))
        .expect("register knowledge");
    registry
        .register(Arc::new(InterfaceOnlyProvider::downloads()))
        .expect("register downloads stub");
    registry
        .register(Arc::new(InterfaceOnlyProvider::prowlarr()))
        .expect("register prowlarr stub");
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::rclone::{CommandOutput, RcloneProvider, RcloneRunner};
    use std::path::PathBuf;
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
    fn day1_has_rclone_knowledge_stubs_not_local() {
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
            .register(Arc::new(KnowledgeProvider::new(PathBuf::from(&dir))))
            .unwrap();
        registry
            .register(Arc::new(InterfaceOnlyProvider::downloads()))
            .unwrap();
        registry
            .register(Arc::new(InterfaceOnlyProvider::prowlarr()))
            .unwrap();

        assert!(registry.contains("rclone"));
        assert!(registry.contains("knowledge"));
        assert!(registry.contains("downloads"));
        assert!(registry.contains("prowlarr"));
        assert!(!registry.contains("local"));

        let backends = registry.backends().unwrap();
        assert!(backends
            .iter()
            .any(|b| b.id == "downloads" && b.interface_only));
        assert!(backends.iter().any(|b| b.id == "rclone" && !b.interface_only));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
