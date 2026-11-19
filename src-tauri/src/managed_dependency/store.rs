// SPDX-License-Identifier: GPL-3.0-or-later
//! R3 商店侧（ADR 0019）：解析元清单、强制 allow-list、下载 release 资产并按 SHA-256 校验。
//!
//! 纯逻辑（解析 / allow-list / 校验 / 下载编排）均以参数与可注入 `fetch` 表达，单测不触网。
//! 真网络函数（`fetch_manifest_text` / `http_get_bytes`）单独隔离，Task 5 的 IPC 才调用。
//!
//! **连不上降级**（ADR 0019 / 0020）：本模块只如实返回 `Err`；「元清单不可达→隐藏 managed 入口」
//! 的 UX 判定在控制面（Task 5）做，store 不自行吞错。

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// 当前受支持的元清单 schema 版本。
pub const STORE_MANIFEST_SCHEMA: u32 = 1;

/// 一份元清单：固定远端拉回，列出被批准的 pack 产物。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreManifest {
    pub schema: u32,
    pub entries: Vec<StoreEntry>,
}

/// 单条被批准的产物条目。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreEntry {
    pub pack_id: String,
    /// 该 release 产物的版本号，供声明版兼容门（Task 4）。
    pub artifact_version: String,
    /// GitHub Release 资产直链（受 allow-list 约束）。
    pub asset_url: String,
    /// 64 位十六进制 SHA-256（取自 release 附属摘要）。
    pub sha256: String,
    /// 可选：产物文件名（缺省从 URL 末段推断）。
    #[serde(default)]
    pub file_name: Option<String>,
}

/// 解析并轻校验元清单 JSON（schema 版本、条目必填）。
pub fn parse_manifest(json: &str) -> Result<StoreManifest, String> {
    let manifest: StoreManifest =
        serde_json::from_str(json).map_err(|e| format!("store manifest parse: {e}"))?;
    if manifest.schema != STORE_MANIFEST_SCHEMA {
        return Err(format!(
            "store manifest schema {} unsupported (expected {STORE_MANIFEST_SCHEMA})",
            manifest.schema
        ));
    }
    for entry in &manifest.entries {
        if entry.pack_id.trim().is_empty()
            || entry.asset_url.trim().is_empty()
            || entry.sha256.trim().is_empty()
            || entry.artifact_version.trim().is_empty()
        {
            return Err(format!(
                "store manifest entry '{}' missing required fields",
                entry.pack_id
            ));
        }
    }
    Ok(manifest)
}

/// 按 pack_id 取条目。
pub fn find_entry<'a>(manifest: &'a StoreManifest, pack_id: &str) -> Option<&'a StoreEntry> {
    manifest.entries.iter().find(|e| e.pack_id == pack_id)
}

/// 提取小写主机名（仅 http/https，剥离 userinfo 与端口）。
fn host_of(url: &str) -> Result<String, String> {
    let lower = url.trim().to_ascii_lowercase();
    let after = lower
        .strip_prefix("http://")
        .or_else(|| lower.strip_prefix("https://"))
        .ok_or_else(|| format!("store url not http(s): {url}"))?;
    let authority = after.split('/').next().unwrap_or("");
    let host = authority.rsplit('@').next().unwrap_or(authority);
    let host = host.split(':').next().unwrap_or(host);
    if host.is_empty() {
        return Err(format!("store url missing host: {url}"));
    }
    Ok(host.to_string())
}

/// 主机是否在 allow-list 内：完全相等，或为某允许域的（点分隔）子域。
pub fn is_host_allowed(url: &str, allow_hosts: &[String]) -> Result<bool, String> {
    let host = host_of(url)?;
    Ok(allow_hosts.iter().map(|a| a.trim().to_ascii_lowercase()).any(|allowed| {
        !allowed.is_empty() && (host == allowed || host.ends_with(&format!(".{allowed}")))
    }))
}

/// 计算并比对 SHA-256（大小写无关）。
pub fn verify_sha256(bytes: &[u8], expected_hex: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    hex::encode(digest).eq_ignore_ascii_case(expected_hex.trim())
}

/// 从 URL 末段推断文件名（剥离查询串）。
pub fn file_name_from_url(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    path.rsplit('/').next().filter(|s| !s.trim().is_empty()).unwrap_or("artifact").to_string()
}

/// 下载编排：先查 allow-list（不在则**不发起 fetch**），再 fetch，再校验 SHA-256。
/// `fetch` 注入真实网络或测试桩；`now` 参数化仅为测试可断言 fetch 是否被调用。
pub fn download_and_verify<F>(
    entry: &StoreEntry,
    allow_hosts: &[String],
    fetch: F,
) -> Result<Vec<u8>, String>
where
    F: FnOnce(&str) -> Result<Vec<u8>, String>,
{
    if !is_host_allowed(&entry.asset_url, allow_hosts)? {
        return Err(format!(
            "store asset host not allow-listed: {}",
            entry.asset_url
        ));
    }
    let bytes = fetch(&entry.asset_url)?;
    if !verify_sha256(&bytes, &entry.sha256) {
        return Err(format!(
            "store artifact checksum mismatch for '{}'",
            entry.pack_id
        ));
    }
    Ok(bytes)
}

/// 原子写产物到目标目录：先写 `.{name}.tmp` 再 rename。
pub fn materialize(dir: &std::path::Path, name: &str, bytes: &[u8]) -> Result<std::path::PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("store create dir: {e}"))?;
    let target = dir.join(name);
    let tmp = dir.join(format!(".{name}.tmp"));
    std::fs::write(&tmp, bytes).map_err(|e| format!("store write tmp: {e}"))?;
    std::fs::rename(&tmp, &target).map_err(|e| format!("store rename onto artifact: {e}"))?;
    Ok(target)
}

// ---- 真网络（不在单测调用；Task 5 IPC 消费）----

/// 拉取元清单文本。
#[allow(dead_code)] // 控制面接线于 Task 5。
pub fn fetch_manifest_text(url: &str) -> Result<String, String> {
    let mut response = ureq::get(url)
        .call()
        .map_err(|e| format!("store manifest fetch ({url}): {e}"))?;
    response
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("store manifest read ({url}): {e}"))
}

/// 下载字节（release 资产）。隔离产物可能 >10MB，放开到 1 GiB 上限（仍校验 SHA-256）。
#[allow(dead_code)] // download_and_verify 注入此函数于 Task 5。
pub fn http_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    const MAX_ARTIFACT_BYTES: u64 = 1024 * 1024 * 1024; // 1 GiB
    let mut response =
        ureq::get(url).call().map_err(|e| format!("store asset fetch ({url}): {e}"))?;
    response
        .body_mut()
        .with_config()
        .limit(MAX_ARTIFACT_BYTES)
        .read_to_vec()
        .map_err(|e| format!("store asset read ({url}): {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn manifest_json() -> &'static str {
        r#"{
            "schema": 1,
            "entries": [
                {
                    "packId": "awam.file-gateway",
                    "artifactVersion": "0.4.2",
                    "assetUrl": "https://github.com/awamwang/awam-file/releases/download/v0.4.2/awam-file-gateway.exe",
                    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "fileName": "awam-file-gateway.exe"
                }
            ]
        }"#
    }

    #[test]
    fn parse_ok_and_find_entry() {
        let m = parse_manifest(manifest_json()).unwrap();
        assert_eq!(m.entries.len(), 1);
        let e = find_entry(&m, "awam.file-gateway").unwrap();
        assert_eq!(e.artifact_version, "0.4.2");
        assert!(find_entry(&m, "nope").is_none());
    }

    #[test]
    fn parse_rejects_unsupported_schema() {
        let json = r#"{"schema":2,"entries":[]}"#;
        let err = parse_manifest(json).unwrap_err();
        assert!(err.contains("schema 2 unsupported"), "{err}");
    }

    #[test]
    fn parse_rejects_entry_missing_fields() {
        let json = r#"{"schema":1,"entries":[{"packId":"x","artifactVersion":"","assetUrl":"https://github.com/a/b","sha256":"deadbeef"}]}"#;
        let err = parse_manifest(json).unwrap_err();
        assert!(err.contains("missing required fields"), "{err}");
    }

    #[test]
    fn allow_list_accepts_exact_and_subdomain_rejects_lookalike() {
        let allow = vec!["github.com".to_string(), "objects.githubusercontent.com".to_string()];
        assert!(is_host_allowed("https://github.com/a/b", &allow).unwrap());
        assert!(is_host_allowed("https://codeload.github.com/x/y", &allow).unwrap());
        // 形似 github.com 的伪装域必须被拒。
        assert!(!is_host_allowed("https://github.com.attacker.tld/payload", &allow).unwrap());
        assert!(!is_host_allowed("https://notgithub.com/p", &allow).unwrap());
    }

    #[test]
    fn allow_list_rejects_non_http_and_localhost_unless_listed() {
        let allow = vec!["github.com".to_string()];
        assert!(is_host_allowed("ftp://github.com/x", &allow).is_err());
        assert!(!is_host_allowed("http://127.0.0.1:18787/a", &allow).unwrap());
        let with_local = vec!["127.0.0.1".to_string()];
        assert!(is_host_allowed("http://127.0.0.1:8766/a", &with_local).unwrap());
    }

    #[test]
    fn verify_sha256_known_empty_digest() {
        // sha256("") = e3b0c442...b855
        assert!(verify_sha256(
            b"",
            "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855"
        ));
        assert!(!verify_sha256(b"x", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"));
    }

    #[test]
    fn download_and_verify_passes_on_matching_checksum() {
        let m = parse_manifest(manifest_json()).unwrap();
        let entry = find_entry(&m, "awam.file-gateway").unwrap();
        let allow = vec!["github.com".to_string()];
        // 桩 fetch 返回空字节，与空串 sha 匹配。
        let bytes = download_and_verify(entry, &allow, |_url| Ok(Vec::new())).unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn download_and_verify_rejects_disallowed_host_without_fetch() {
        let m = parse_manifest(manifest_json()).unwrap();
        let mut entry = find_entry(&m, "awam.file-gateway").unwrap().clone();
        entry.asset_url = "https://evil.tld/awam-file-gateway.exe".to_string();
        let called = Cell::new(false);
        let allow = vec!["github.com".to_string()];
        let err = download_and_verify(&entry, &allow, |_url| {
            called.set(true);
            Ok(Vec::new())
        })
        .unwrap_err();
        assert!(err.contains("not allow-listed"), "{err}");
        assert!(!called.get(), "disallowed host must not even fetch");
    }

    #[test]
    fn download_and_verify_rejects_checksum_mismatch() {
        let m = parse_manifest(manifest_json()).unwrap();
        let entry = find_entry(&m, "awam.file-gateway").unwrap();
        let allow = vec!["github.com".to_string()];
        // fetch 返回非空字节，校验应失败（期望空串 sha）。
        let err = download_and_verify(entry, &allow, |_| Ok(vec![1, 2, 3])).unwrap_err();
        assert!(err.contains("checksum mismatch"), "{err}");
    }

    #[test]
    fn file_name_from_url_strips_query_and_falls_back() {
        assert_eq!(
            file_name_from_url("https://github.com/a/b/releases/v1/tool.exe?token=x"),
            "tool.exe"
        );
        assert_eq!(file_name_from_url("https://github.com/a/b/"), "artifact");
    }

    #[test]
    fn materialize_writes_atomically_and_overwrites() {
        let dir = std::env::temp_dir().join(format!("afm-store-mat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = materialize(&dir, "tool.exe", b"hello").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"hello");
        // 二次写入覆盖，不留 tmp。
        materialize(&dir, "tool.exe", b"world").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"world");
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "no tmp residue");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
