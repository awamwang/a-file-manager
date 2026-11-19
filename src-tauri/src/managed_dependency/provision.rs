// SPDX-License-Identifier: GPL-3.0-or-later
//! 托管产物分发胶合：把 Task 3（store）+ Task 4（compat/credential）按正确次序串成一条
//! provision 链路——**先声明版兼容门，再 allow-list，再下载，再 SHA-256 校验，最后原子落盘**。
//!
//! 顺序是安全关键：门不过 / 主机不在 allow-list 时**绝不发起网络请求**。`fetch` 注入，单测全离线。

use std::path::{Path, PathBuf};

use super::compat::{ensure_declared_version_compat, VersionRange};
use super::store::{download_and_verify, file_name_from_url, materialize, StoreEntry};

/// 产物落盘子目录名（位于 `{app_data}/managed-dependencies/{packId}/` 下）。
pub const ARTIFACT_SUBDIR: &str = "artifact";

/// 取产物落盘名：清单 `fileName` 优先，否则从 URL 末段推断。
pub fn artifact_file_name(entry: &StoreEntry) -> String {
    entry
        .file_name
        .clone()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| file_name_from_url(&entry.asset_url))
}

/// 完整 provision 链路。返回落盘后的产物路径；任一步失败即 `Err`，且不写盘、不下载（前置校验）。
pub fn provision_artifact<F>(
    entry: &StoreEntry,
    range: Option<&VersionRange>,
    allow_hosts: &[String],
    pack_dir: &Path,
    fetch: F,
) -> Result<PathBuf, String>
where
    F: FnOnce(&str) -> Result<Vec<u8>, String>,
{
    // 1. 声明版兼容门（不过则不下载）。
    ensure_declared_version_compat(&entry.artifact_version, range)?;
    // 2-3. allow-list + 下载 + SHA-256（download_and_verify 内部已是「先查主机再 fetch 再校验」）。
    let bytes = download_and_verify(entry, allow_hosts, fetch)?;
    // 4. 原子落盘到 artifact 子目录。
    let dir = pack_dir.join(ARTIFACT_SUBDIR);
    let name = artifact_file_name(entry);
    materialize(&dir, &name, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managed_dependency::compat::VersionRange;
    use std::cell::Cell;

    fn entry(asset_url: &str, version: &str) -> StoreEntry {
        StoreEntry {
            pack_id: "awam.file-gateway".to_string(),
            artifact_version: version.to_string(),
            asset_url: asset_url.to_string(),
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(), // sha256("")
            file_name: Some("awam-file-gateway.exe".to_string()),
        }
    }

    #[test]
    fn version_gate_mismatch_blocks_before_any_fetch() {
        let e = entry("https://github.com/awamwang/awam-file/releases/v0.4.2/tool.exe", "5.0.0");
        let range = VersionRange { min: "0.1.0".to_string(), max: "1.0.0".to_string() };
        let called = Cell::new(false);
        let err = provision_artifact(&e, Some(&range), &["github.com".to_string()],
            std::path::Path::new("/tmp/whatever"),
            |_| { called.set(true); Ok(Vec::new()) })
            .unwrap_err();
        assert!(err.contains("versionCompat mismatch"), "{err}");
        assert!(!called.get(), "gate fail must not fetch");
    }

    #[test]
    fn disallowed_host_blocks_before_fetch() {
        let e = entry("https://evil.tld/releases/v0.4.2/tool.exe", "0.4.2");
        let called = Cell::new(false);
        let err = provision_artifact(&e, None, &["github.com".to_string()],
            std::path::Path::new("/tmp/whatever"),
            |_| { called.set(true); Ok(Vec::new()) })
            .unwrap_err();
        assert!(err.contains("not allow-listed"), "{err}");
        assert!(!called.get(), "disallowed host must not fetch");
    }

    #[test]
    fn checksum_mismatch_prevents_write() {
        let pack_dir = std::env::temp_dir().join(format!("afm-provision-badck-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pack_dir);
        let e = entry("https://github.com/awamwang/awam-file/releases/v0.4.2/tool.exe", "0.4.2");
        // fetch 返回非空，与空串 sha 不符。
        let err = provision_artifact(&e, None, &["github.com".to_string()], &pack_dir, |_| Ok(vec![9; 4]))
            .unwrap_err();
        assert!(err.contains("checksum mismatch"), "{err}");
        assert!(!pack_dir.join(ARTIFACT_SUBDIR).join("awam-file-gateway.exe").exists(), "must not write on mismatch");
        let _ = std::fs::remove_dir_all(&pack_dir);
    }

    #[test]
    fn happy_path_writes_artifact_to_pack_dir() {
        let pack_dir = std::env::temp_dir().join(format!("afm-provision-ok-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pack_dir);
        let e = entry("https://github.com/awamwang/awam-file/releases/v0.4.2/tool.exe", "0.4.2");
        let path = provision_artifact(&e, None, &["github.com".to_string()], &pack_dir, |_| Ok(Vec::new()))
            .unwrap();
        assert_eq!(path, pack_dir.join(ARTIFACT_SUBDIR).join("awam-file-gateway.exe"));
        assert_eq!(std::fs::read(&path).unwrap(), Vec::<u8>::new());
        let _ = std::fs::remove_dir_all(&pack_dir);
    }

    #[test]
    fn artifact_file_name_prefers_list_then_url() {
        let with = entry("https://github.com/a/b/tool.exe", "1.0.0");
        assert_eq!(artifact_file_name(&with), "awam-file-gateway.exe");
        let mut without = with.clone();
        without.file_name = None;
        assert_eq!(artifact_file_name(&without), "tool.exe");
    }
}
