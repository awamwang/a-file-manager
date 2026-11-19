// SPDX-License-Identifier: GPL-3.0-or-later
//! managed 产物 feed 配置（ADR 0022）：AFM 自有发布通道的元清单 URL 与 allow-list。
//!
//! 「白名单商店」语义已收窄为 **AFM 自有发布通道**：产物来源由一份 feed 元清单集中声明，
//! 仅 AFM 受批准来源主机。本模块只负责「读配置 + 拉/校验元清单 + 解析某 pack 的 managed 绑定」，
//! 真下载（含 SHA-256 校验）在 [`super::provision`]。网络以注入闭包表达，单测全离线。
//!
//! 降级语义（ADR 0019/0020）：未配置 URL 或元清单不可达 → `Ok(None)`，控制面据此隐藏 managed
//! 入口、回退 config-mode 展示，**绝不阻断启用/拉起**。

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use super::store::{find_entry, is_host_allowed, parse_manifest, StoreManifest};

/// feed 配置文件名（位于 app_data 顶层，**不在** `managed-dependencies/` 内，
/// 避免被 `registry::list_records` 当作托管记录解析）。
pub const FEED_FILE_NAME: &str = "managed-dependency-feed.json";

/// AFM 自有发布通道配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedConfig {
    /// 元清单 JSON 的 http(s) URL；空表示未配置（所有 pack 走内置/config）。
    #[serde(default)]
    pub manifest_url: String,
    /// 受批准来源主机（精确或点分子域）；同时约束元清单与产物资产的下载主机。
    #[serde(default)]
    pub allow_hosts: Vec<String>,
}

impl Default for FeedConfig {
    fn default() -> Self {
        Self { manifest_url: String::new(), allow_hosts: Vec::new() }
    }
}

/// `{app_data_dir}/managed-dependency-feed.json`
pub fn feed_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(FEED_FILE_NAME)
}

/// 读取 feed 配置；不存在返回默认（未配置态）。
pub fn load_feed(app_data_dir: &Path) -> FeedConfig {
    let path = feed_path(app_data_dir);
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<FeedConfig>(&text).unwrap_or_default(),
        Err(_) => FeedConfig::default(),
    }
}

/// 原子写 feed 配置（先写 `.tmp` 再 rename）。
pub fn save_feed(app_data_dir: &Path, cfg: &FeedConfig) -> Result<(), String> {
    let path = feed_path(app_data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("managed_dependency feed create dir: {e}"))?;
    }
    let text = serde_json::to_string_pretty(cfg)
        .map_err(|e| format!("managed_dependency feed serialize: {e}"))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| format!("managed_dependency feed write tmp: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("managed_dependency feed rename: {e}"))?;
    Ok(())
}

/// 拉取并解析 feed 元清单。
///
/// - 未配置 `manifest_url` → `Ok(None)`（无 feed，所有 pack 走内置/config）。
/// - 配置了 URL 但其主机不在 allow-list → `Err`（绝不向未批准来源发起 fetch）。
/// - 拉取/解析失败 → `Err`（调用方据此降级；本函数不吞错，保持与 store 一致的「如实返回」）。
pub fn fetch_manifest<F>(cfg: &FeedConfig, fetch: F) -> Result<Option<StoreManifest>, String>
where
    F: FnOnce(&str) -> Result<String, String>,
{
    let url = cfg.manifest_url.trim();
    if url.is_empty() {
        return Ok(None);
    }
    if !is_host_allowed(url, &cfg.allow_hosts)? {
        return Err(format!("feed manifest host not allow-listed: {url}"));
    }
    let text = fetch(url)?;
    let manifest = parse_manifest(&text)?;
    Ok(Some(manifest))
}

/// 命中 feed entry 时解析出的 managed 绑定（启用时据此写 `record`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedBinding {
    pub declared_version: String,
    pub base_url: String,
}

/// 纯函数：按 pack_id 解析 managed 绑定。命中 feed entry 时返回其声明产物版本与本机探活地址
/// （供启用时写入 `record.declared_version` / `base_url`、并把 `mode` 切成 Managed）；未命中返回 `None`。
pub fn resolve_managed_binding(manifest: Option<&StoreManifest>, pack_id: &str) -> Option<ManagedBinding> {
    let entry = manifest.and_then(|m| find_entry(m, pack_id))?;
    Some(ManagedBinding {
        declared_version: entry.artifact_version.clone(),
        base_url: entry.base_url.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(url: &str, hosts: &[&str]) -> FeedConfig {
        FeedConfig {
            manifest_url: url.to_string(),
            allow_hosts: hosts.iter().map(|h| h.to_string()).collect(),
        }
    }

    const MANIFEST: &str = r#"{
        "schema": 1,
        "entries": [
            {
                "packId": "awam.tool-runtime",
                "artifactVersion": "0.1.0",
                "assetUrl": "https://feed.afm.local/tool-manager/awam-tool-manager.exe",
                "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "fileName": "awam-tool-manager.exe",
                "baseUrl": "http://127.0.0.1:8766"
            }
        ]
    }"#;

    #[test]
    fn empty_url_yields_no_manifest() {
        let m = fetch_manifest(&cfg("", &[]), |_| unreachable!("must not fetch")).unwrap();
        assert!(m.is_none());
    }

    #[test]
    fn disallowed_manifest_host_blocks_before_fetch() {
        use std::cell::Cell;
        let called = Cell::new(false);
        let err = fetch_manifest(&cfg("https://evil.tld/m.json", &["feed.afm.local".into()]), |_| {
            called.set(true);
            Ok(MANIFEST.to_string())
        })
        .unwrap_err();
        assert!(err.contains("not allow-listed"), "{err}");
        assert!(!called.get(), "disallowed manifest host must not fetch");
    }

    #[test]
    fn allowed_host_parses_and_binds() {
        let m = fetch_manifest(&cfg("https://feed.afm.local/m.json", &["feed.afm.local".into()]), |u| {
            assert_eq!(u, "https://feed.afm.local/m.json");
            Ok(MANIFEST.to_string())
        })
        .unwrap()
        .expect("manifest");
        let binding = resolve_managed_binding(Some(&m), "awam.tool-runtime").expect("binding");
        assert_eq!(binding.declared_version, "0.1.0");
        assert_eq!(binding.base_url, "http://127.0.0.1:8766");
        assert_eq!(resolve_managed_binding(Some(&m), "awam.knowledge"), None);
    }

    #[test]
    fn resolve_binding_none_manifest_is_none() {
        assert_eq!(resolve_managed_binding(None, "awam.tool-runtime"), None);
    }

    #[test]
    fn feed_round_trip_and_avoids_records_root() {
        let dir = std::env::temp_dir().join(format!("afm-feed-rt-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let c = cfg("https://feed.afm.local/m.json", &["feed.afm.local"]);
        save_feed(&dir, &c).unwrap();
        assert_eq!(load_feed(&dir), c);
        // 关键不变量：feed 文件不落 records_root，避免污染 list_records。
        assert_eq!(feed_path(&dir), dir.join(FEED_FILE_NAME));
        assert_ne!(feed_path(&dir).parent().unwrap().join("managed-dependencies"), feed_path(&dir).parent().unwrap());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_feed_missing_file_is_default() {
        let dir = std::env::temp_dir().join(format!("afm-feed-miss-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(load_feed(&dir), FeedConfig::default());
    }
}
