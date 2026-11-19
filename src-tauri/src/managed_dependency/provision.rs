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

/// sidecar 默认工作目录名（`workingDirRel` 缺省时产物落此，与 `pack.json` 约定一致）。
pub const DEFAULT_SIDECAR_DIR: &str = "sidecar";

/// managed 拉起前「确保产物就位」（ADR 0022）。幂等：本地产物已在则不下载。
///
/// - `entry == None`（feed 未列该 pack，或 config-mode）→ `Ok(None)`：产物随 pack 内置，spawn 用盘上 command。
/// - `entry == Some`：目标 = `{pack_install_dir}/{working_dir_rel|sidecar}/{产物文件名}`。
///   - 已存在 → `Ok(Some(目标))`（幂等，不发起 fetch）。
///   - 缺失 → allow-list + 下载 + SHA-256 校验 + 原子落盘，`Ok(Some(落盘路径))`。
///
/// 跳过声明版兼容门：自构建产物版本在构建时即锁定（与 ADR 0022 一致），门只服务 config-mode 外部实例。
/// `fetch` 注入真网络（`store::http_get_bytes`）或测试桩，单测全离线。
pub fn ensure_artifact<F>(
    entry: Option<&StoreEntry>,
    allow_hosts: &[String],
    pack_install_dir: &Path,
    working_dir_rel: Option<&str>,
    fetch: F,
) -> Result<Option<PathBuf>, String>
where
    F: FnOnce(&str) -> Result<Vec<u8>, String>,
{
    let entry = match entry {
        Some(e) => e,
        None => return Ok(None),
    };
    let rel = working_dir_rel.map(|r| r.trim()).filter(|r| !r.is_empty()).unwrap_or(DEFAULT_SIDECAR_DIR);
    let sidecar_dir = pack_install_dir.join(rel);
    let name = artifact_file_name(entry);
    let target = sidecar_dir.join(&name);
    if target.exists() {
        return Ok(Some(target));
    }
    let bytes = download_and_verify(entry, allow_hosts, fetch)?;
    let written = materialize(&sidecar_dir, &name, &bytes)?;
    Ok(Some(written))
}

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
            base_url: String::new(),
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

    #[test]
    fn ensure_artifact_none_entry_skips_without_fetch() {
        let called = std::cell::Cell::new(false);
        let out = ensure_artifact(None, &["github.com".to_string()], std::path::Path::new("/tmp/x"),
            Some("sidecar"), |_| { called.set(true); Ok(Vec::new()) })
            .unwrap();
        assert!(out.is_none());
        assert!(!called.get(), "no entry must not fetch");
    }

    #[test]
    fn ensure_artifact_downloads_to_sidecar_dir_when_missing() {
        let pack_dir = std::env::temp_dir().join(format!("afm-ensure-dl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pack_dir);
        let e = entry("https://github.com/awamwang/awam-file/releases/v0.4.2/tool.exe", "0.4.2");
        let out = ensure_artifact(Some(&e), &["github.com".to_string()], &pack_dir, Some("sidecar"),
            |_| Ok(Vec::new()))
            .unwrap()
            .expect("should materialize");
        assert_eq!(out, pack_dir.join("sidecar").join("awam-file-gateway.exe"));
        assert!(out.exists(), "artifact landed in sidecar dir");
        let _ = std::fs::remove_dir_all(&pack_dir);
    }

    #[test]
    fn ensure_artifact_is_idempotent_when_target_present() {
        let pack_dir = std::env::temp_dir().join(format!("afm-ensure-idem-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pack_dir);
        let sidecar = pack_dir.join("sidecar");
        std::fs::create_dir_all(&sidecar).unwrap();
        let target = sidecar.join("awam-file-gateway.exe");
        std::fs::write(&target, b"present").unwrap();
        let e = entry("https://github.com/awamwang/awam-file/releases/v0.4.2/tool.exe", "0.4.2");
        let called = std::cell::Cell::new(false);
        let out = ensure_artifact(Some(&e), &["github.com".to_string()], &pack_dir, Some("sidecar"),
            |_| { called.set(true); Ok(Vec::new()) })
            .unwrap()
            .expect("existing path");
        assert_eq!(out, target);
        assert!(!called.get(), "present artifact must not re-fetch");
        assert_eq!(std::fs::read(&target).unwrap(), b"present", "must not clobber existing");
        let _ = std::fs::remove_dir_all(&pack_dir);
    }

    /// ADR 0022 provision 接缝的端到端最小证伪：起本地 http feed，用 **AFM 真实的 ureq 网络函数**
    /// （`store::fetch_manifest_text` + `store::http_get_bytes`）拉元清单、下载产物、SHA-256 校验、落盘。
    /// gated（`#[ignore]`）：需本地 listener，默认 `cargo test` 不跑；`cargo test --lib -- --ignored` 触发。
    #[test]
    #[ignore = "起本地 http feed，真 ureq 下载"]
    fn ensure_artifact_real_http_feed_download_roundtrip() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use crate::managed_dependency::feed::{fetch_manifest, FeedConfig};
        use crate::managed_dependency::store::{self};
        use sha2::{Digest, Sha256};

        let artifact: Vec<u8> = b"AFM-INTEGRATED-ARTIFACT-BYTES-v1".to_vec();
        let mut h = Sha256::new();
        h.update(&artifact);
        let digest = hex::encode(h.finalize());

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let manifest_text = format!(
            r#"{{"schema":1,"entries":[{{"packId":"awam.tool-runtime","artifactVersion":"0.1.0","assetUrl":"http://127.0.0.1:{port}/artifacts/awam-tool-manager.exe","sha256":"{digest}","fileName":"awam-tool-manager.exe","baseUrl":"http://127.0.0.1:8766"}}]}}"#
        );
        let manifest_clone = manifest_text.clone();
        let artifact_clone = artifact.clone();
        let server = std::thread::spawn(move || {
            for stream in listener.incoming().take(2) {
                let Ok(mut s) = stream else { continue };
                let mut buf = [0u8; 1024];
                let _ = s.read(&mut buf);
                let body: &[u8] = if String::from_utf8_lossy(&buf).contains("/manifest.json") {
                    manifest_clone.as_bytes()
                } else {
                    artifact_clone.as_slice()
                };
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = s.write_all(head.as_bytes());
                let _ = s.write_all(body);
                let _ = s.flush();
            }
        });

        let cfg = FeedConfig {
            manifest_url: format!("http://127.0.0.1:{port}/manifest.json"),
            allow_hosts: vec!["127.0.0.1".to_string()],
        };
        // 真网络拉元清单（allow-list 命中 127.0.0.1）。
        let manifest = fetch_manifest(&cfg, store::fetch_manifest_text)
            .unwrap()
            .expect("manifest from live feed");
        let entry = store::find_entry(&manifest, "awam.tool-runtime").expect("entry");
        assert_eq!(entry.sha256, digest);

        let pack_dir = std::env::temp_dir().join(format!("afm-feed-dl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pack_dir);
        // 真网络下载产物 + 校验 + 落盘。
        let path = ensure_artifact(
            Some(entry),
            &cfg.allow_hosts,
            &pack_dir,
            Some("sidecar"),
            store::http_get_bytes,
        )
        .unwrap()
        .expect("downloaded artifact");
        assert_eq!(path, pack_dir.join("sidecar").join("awam-tool-manager.exe"));
        assert_eq!(std::fs::read(&path).unwrap(), artifact, "artifact bytes match");

        // 幂等：已就位时绝不再下载（即便 feed 已停）。
        let again = ensure_artifact(
            Some(entry),
            &cfg.allow_hosts,
            &pack_dir,
            Some("sidecar"),
            |_u| -> Result<Vec<u8>, String> { panic!("must not refetch when present") },
        )
        .unwrap()
        .expect("existing artifact");
        assert_eq!(again, path);

        let _ = std::fs::remove_dir_all(&pack_dir);
        let _ = server.join();
    }
}
