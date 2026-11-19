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

/// 一份元清单：固定远端拉回，列出被批准的 pack 产物与可安装 pack 包。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreManifest {
    pub schema: u32,
    #[serde(default)]
    pub entries: Vec<StoreEntry>,
    /// 可安装 pack 包目录（ADR 0022「可安装 Store 接缝」）；旧清单缺此段=空，向后兼容。
    #[serde(default)]
    pub packs: Vec<PackEntry>,
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
    /// managed 拉起后 AFM 探活/附着该实例的本机地址（如 `http://127.0.0.1:18778`）。
    /// 缺省空表示该 pack 无固定本机探活地址（仅 config-mode 用户提供）。
    #[serde(default)]
    pub base_url: String,
    /// 可选：对象存储镜像回落地址。主源（GitHub Release）传输失败/不可达时按同 SHA 校验下载。
    /// 缺省 None=不回落（仅本地/单源 feed 不变）。回落主机同样须命中 allow-list。
    #[serde(default)]
    pub asset_fallback_url: Option<String>,
}

/// 单条可安装 pack 包条目（pack 安装包 = zip，含 pack.json + UI 资源；sidecar 产物另由 entries 描述）。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackEntry {
    pub pack_id: String,
    pub name: String,
    pub version: String,
    /// pack 安装包（zip）直链，受 allow-list 约束。
    pub install_url: String,
    /// zip 安装包 64 位十六进制 SHA-256。
    pub sha256: String,
    /// 可选：对象存储镜像回落地址（主源不可达时按同 SHA 下载）。同样受 allow-list 约束。
    #[serde(default)]
    pub install_fallback_url: Option<String>,
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
    for pack in &manifest.packs {
        if pack.pack_id.trim().is_empty()
            || pack.name.trim().is_empty()
            || pack.version.trim().is_empty()
            || pack.install_url.trim().is_empty()
            || pack.sha256.trim().is_empty()
        {
            return Err(format!(
                "store manifest pack '{}' missing required fields",
                pack.pack_id
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

/// 计算磁盘文件 SHA-256（小写十六进制）。用于与 feed entry 的 `sha256` 比对，判断已落盘产物
/// 是否即 feed 声明的那一版（一致=最新；不一致=有可用升级，见 provision::ensure_artifact）。
pub fn file_sha256(path: &std::path::Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("store read file for sha ({path:?}): {e}"))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

/// 从 URL 末段推断文件名（剥离查询串）。
pub fn file_name_from_url(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    path.rsplit('/').next().filter(|s| !s.trim().is_empty()).unwrap_or("artifact").to_string()
}

/// 通用「先查 allow-list（不在则**绝不发起 fetch**）→ fetch → SHA-256 校验」。
/// 产物下载（`download_and_verify`）与可安装 pack 包下载共用此原语。`fetch` 注入网络/桩。
/// 无回落 URL 的便捷包装；带回落链时调 [`download_verify_url_with_fallback`]。
pub fn download_verify_url<F>(
    url: &str,
    expected_sha: &str,
    allow_hosts: &[String],
    mut fetch: F,
) -> Result<Vec<u8>, String>
where
    F: FnMut(&str) -> Result<Vec<u8>, String>,
{
    download_verify_url_with_fallback(url, None, expected_sha, allow_hosts, &mut fetch)
}

/// 主源优先、失败回落的下载链（ADR 0022 / P2-5：GitHub Release 主源，对象存储镜像回落）。
/// 规则（安全不变量逐源成立）：
/// - 主源主机不在 allow-list → **绝不 fetch 主源**，记录原因；
/// - 主源 fetch/校验出错（传输失败 / 不可达 / 摘要不符）→ 若配了回落 URL，则对**回落源**同样先查 allow-list 再 fetch，并按同一 SHA 校验；
/// - 回落命中即成功；两路都失败则返回主源原始错误。
/// `fetch` 需 `FnMut`（可能调两次）。
pub fn download_verify_url_with_fallback<F>(
    url: &str,
    fallback_url: Option<&str>,
    expected_sha: &str,
    allow_hosts: &[String],
    mut fetch: F,
) -> Result<Vec<u8>, String>
where
    F: FnMut(&str) -> Result<Vec<u8>, String>,
{
    let primary_err: String = match is_host_allowed(url, allow_hosts) {
        Ok(true) => match fetch(url) {
            Ok(bytes) if verify_sha256(&bytes, expected_sha) => return Ok(bytes),
            Ok(_) => format!("store checksum mismatch for {url}"),
            Err(err) => err,
        },
        Ok(false) => format!("store asset host not allow-listed: {url}"),
        Err(err) => err,
    };
    let Some(fallback) = fallback_url.map(str::trim).filter(|u| !u.is_empty()) else {
        return Err(primary_err);
    };
    if !is_host_allowed(fallback, allow_hosts)? {
        return Err(format!("store fallback host not allow-listed: {fallback}"));
    }
    let bytes = fetch(fallback)
        .map_err(|err| format!("{primary_err}; fallback fetch failed: {err}"))?;
    if !verify_sha256(&bytes, expected_sha) {
        return Err(format!("store checksum mismatch for fallback {fallback}"));
    }
    Ok(bytes)
}

/// 下载编排（产物条目）：主源 `asset_url` + 可选 `asset_fallback_url` 回落链。
pub fn download_and_verify<F>(
    entry: &StoreEntry,
    allow_hosts: &[String],
    mut fetch: F,
) -> Result<Vec<u8>, String>
where
    F: FnMut(&str) -> Result<Vec<u8>, String>,
{
    download_verify_url_with_fallback(
        &entry.asset_url,
        entry.asset_fallback_url.as_deref(),
        &entry.sha256,
        allow_hosts,
        &mut fetch,
    )
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

// ---- 真网络（被 managed 产物 feed 拉取与 provision 下载生产调用；单测注入桩离线）----

/// 拉取元清单文本。
pub fn fetch_manifest_text(url: &str) -> Result<String, String> {
    let mut response = ureq::get(url)
        .call()
        .map_err(|e| format!("store manifest fetch ({url}): {e}"))?;
    response
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("store manifest read ({url}): {e}"))
}

/// 下载字节（feed 产物资产）。隔离产物可能 >10MB，放开到 1 GiB 上限（仍校验 SHA-256）。
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
        assert!(is_host_allowed("http://127.0.0.1:18778/a", &with_local).unwrap());
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
    fn file_sha256_matches_written_bytes() {
        let dir = std::env::temp_dir().join(format!("afm-store-fsha-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = materialize(&dir, "empty.exe", b"").unwrap();
        assert_eq!(
            file_sha256(&p).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        materialize(&dir, "empty.exe", b"changed").unwrap();
        assert_ne!(
            file_sha256(&p).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let _ = std::fs::remove_dir_all(&dir);
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
    fn download_and_verify_falls_back_to_mirror_when_primary_unreachable() {
        let json = r#"{"schema":1,"entries":[{"packId":"p","artifactVersion":"1.0.0","assetUrl":"https://primary.example/a.exe","assetFallbackUrl":"https://mirror.example/a.exe","sha256":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","fileName":"a.exe"}]}"#;
        let m = parse_manifest(json).unwrap();
        let e = find_entry(&m, "p").unwrap();
        let allow = vec!["primary.example".to_string(), "mirror.example".to_string()];
        let mut urls = Vec::new();
        let bytes = download_and_verify(e, &allow, |u| {
            urls.push(u.to_string());
            if u.contains("primary") {
                Err("down".to_string())
            } else {
                Ok(Vec::new())
            }
        })
        .unwrap();
        assert!(bytes.is_empty(), "mirror bytes accepted");
        assert_eq!(urls.len(), 2, "primary then mirror");
        assert!(urls[1].contains("mirror"), "{urls:?}");
    }

    #[test]
    fn download_verify_never_fetches_disallowed_fallback() {
        let json = r#"{"schema":1,"entries":[{"packId":"p","artifactVersion":"1.0.0","assetUrl":"https://evil.example/a.exe","assetFallbackUrl":"https://also-evil.example/a.exe","sha256":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855","fileName":"a.exe"}]}"#;
        let m = parse_manifest(json).unwrap();
        let e = find_entry(&m, "p").unwrap();
        let called = std::cell::Cell::new(false);
        let err = download_and_verify(e, &["github.com".to_string()], |_| {
            called.set(true);
            Ok(Vec::new())
        })
        .unwrap_err();
        assert!(err.contains("not allow-listed"), "{err}");
        assert!(!called.get(), "neither primary nor fallback may be fetched when disallowed");
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

    #[test]
    fn manifest_with_packs_parses_and_legacy_defaults_empty() {
        let json = r#"{
            "schema": 1,
            "entries": [],
            "packs": [
                {
                    "packId": "awam.tool-runtime",
                    "name": "工具管理运行时",
                    "version": "0.1.0",
                    "installUrl": "http://127.0.0.1:8078/packs/awam.tool-runtime.zip",
                    "sha256": "da29c005a885328bc1c231d789a61ad0c58b4c067b4d00f8f32c5efc306f821e"
                }
            ]
        }"#;
        let m = parse_manifest(json).unwrap();
        assert_eq!(m.packs.len(), 1);
        assert_eq!(m.packs[0].install_url, "http://127.0.0.1:8078/packs/awam.tool-runtime.zip");
        // 旧清单（无 packs 段）默认空，向后兼容。
        assert!(parse_manifest(manifest_json()).unwrap().packs.is_empty());
    }

    #[test]
    fn manifest_pack_missing_field_rejected() {
        let json = r#"{"schema":1,"packs":[{"packId":"x","name":"X","version":"","installUrl":"https://a/b.zip","sha256":"deadbeef"}]}"#;
        let err = parse_manifest(json).unwrap_err();
        assert!(err.contains("pack 'x' missing required fields"), "{err}");
    }

    #[test]
    fn download_verify_url_disallowed_host_never_fetches() {
        let called = std::cell::Cell::new(false);
        let err = download_verify_url(
            "https://evil.tld/p.zip",
            "ab",
            &["feed.afm.local".to_string()],
            |_| { called.set(true); Ok(vec![1]) },
        )
        .unwrap_err();
        assert!(err.contains("not allow-listed"), "{err}");
        assert!(!called.get());
    }
}
