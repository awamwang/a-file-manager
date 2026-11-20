// SPDX-License-Identifier: GPL-3.0-or-later
//! awam-knowledge HTTP 服务客户端（ADR 0016）：内容检索 + vault 人机通道代理。
//!
//! 明文边界：`knowledge_service_reveal` 的返回体是本模块唯一明文出口（密钥行短显 5s）；
//! `knowledge_service_copy` 取到明文后直写系统剪贴板（`clipboard::set_text`），只返回
//! 掩码元数据——明文不进日志、不进持久状态、不进前端 store。

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Duration;
use ureq::Agent;

pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:18790";
/// 设置项键：Power 层可配置服务地址与鉴权 token（ADR 0016）。
pub const URL_SETTING_KEY: &str = "power.awamKnowledgeServiceUrl";
pub const TOKEN_SETTING_KEY: &str = "power.awamKnowledgeServiceToken";

const HEALTH_TIMEOUT: Duration = Duration::from_secs(2);
const SEARCH_TIMEOUT: Duration = Duration::from_secs(8);
/// reveal 经服务端代理 agent_secret.py（子进程 10s 上限），客户端留足余量。
const VAULT_TIMEOUT: Duration = Duration::from_secs(15);
const DEFAULT_TOPK: u32 = 12;
const MAX_TOPK: u32 = 50;

static CONFIGURED_URL: Mutex<Option<String>> = Mutex::new(None);
static CONFIGURED_TOKEN: Mutex<Option<String>> = Mutex::new(None);

/// 规范化服务 base URL：去空白、去尾部 `/`；空串视为未配置。
pub fn normalize_service_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_end_matches('/').to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn normalize_token(raw: &str) -> Option<String> {
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// 应用设置中的服务地址与 token（启动同步 + settings_set patch 同步共用）。
pub fn set_configured(url: Option<&str>, token: Option<&str>) {
    if let Ok(mut guard) = CONFIGURED_URL.lock() {
        *guard = url.and_then(normalize_service_url);
    }
    if let Ok(mut guard) = CONFIGURED_TOKEN.lock() {
        *guard = token.and_then(normalize_token);
    }
}

/// 仅更新生效服务地址（managed 端口回退后由命令层推送实际绑定端口），**不触碰 token**。
/// 与 `set_configured` 分开，避免传 `url=Some、token=None` 误清已配置的鉴权 token。
pub fn set_effective_base_url(url: &str) {
    if let Ok(mut guard) = CONFIGURED_URL.lock() {
        *guard = normalize_service_url(url);
    }
}

/// 解析顺序：环境变量 → 设置 → 默认 `http://127.0.0.1:18790`。
fn base_url() -> String {
    if let Ok(env) = std::env::var("AFM_KNOWLEDGE_SERVICE_URL") {
        if let Some(url) = normalize_service_url(&env) {
            return url;
        }
    }
    if let Ok(guard) = CONFIGURED_URL.lock() {
        if let Some(url) = guard.as_ref() {
            return url.clone();
        }
    }
    DEFAULT_BASE_URL.to_string()
}

/// token 解析：环境变量 → 设置；`None` = 未配置（服务端将 401）。
fn token() -> Option<String> {
    if let Ok(env) = std::env::var("AFM_KNOWLEDGE_SERVICE_TOKEN") {
        if let Some(token) = normalize_token(&env) {
            return Some(token);
        }
    }
    if let Ok(guard) = CONFIGURED_TOKEN.lock() {
        if let Some(token) = guard.as_ref() {
            return Some(token.clone());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// 错误：面向前端的统一结构（camelCase），服务端 {code, reason} 优先。
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeError {
    pub code: String,
    pub reason: String,
    pub status: u16,
}

impl std::fmt::Display for KnowledgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "knowledge service error [{}]: {}",
            self.code, self.reason
        )
    }
}

fn err(code: &str, reason: impl Into<String>, status: u16) -> KnowledgeError {
    KnowledgeError {
        code: code.to_string(),
        reason: reason.into(),
        status,
    }
}

/// 非成功状态 → 错误：body 里的 `{code, reason}` 优先，缺省退回 `http_<status>`。
fn error_from_response(status: u16, value: &serde_json::Value) -> KnowledgeError {
    let code = value
        .get("code")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| format!("http_{status}"));
    let reason = value
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    KnowledgeError {
        code,
        reason,
        status,
    }
}

fn agent(timeout: Duration) -> Agent {
    Agent::config_builder()
        .timeout_global(Some(timeout))
        // 非 2xx 不作为传输错误：必须由 post_json 读响应体里的 {code, reason} 统一映射。
        // ureq 3 默认 http_status_as_error=true，423/404 等会变成 Err(StatusCode(u16))
        // 且丢失响应体——服务端 unlock_required 等错误码全被吞成 "offline"，
        // 解锁确认框永远不弹（ADR 0017 解锁链路回归修复）。
        .http_status_as_error(false)
        .build()
        .into()
}

/// POST JSON（带 Bearer token），成功返回 body；网络失败 → `offline`。
fn post_json(
    path: &str,
    body: serde_json::Value,
    timeout: Duration,
) -> Result<serde_json::Value, KnowledgeError> {
    let url = format!("{}{}", base_url(), path);
    let mut request = agent(timeout).post(&url);
    if let Some(token) = token() {
        request = request.header("Authorization", format!("Bearer {token}").as_str());
    }
    let mut response = request.send_json(body).map_err(|e| {
        err(
            "offline",
            format!("knowledge service request failed ({url}): {e}"),
            0,
        )
    })?;
    let status = response.status().as_u16();
    let value: serde_json::Value = response.body_mut().read_json().map_err(|e| {
        err(
            "bad_response",
            format!("knowledge service JSON decode failed ({url}): {e}"),
            status,
        )
    })?;
    if status >= 400 {
        return Err(error_from_response(status, &value));
    }
    Ok(value)
}

// ---------------------------------------------------------------------------
// Wire 类型（与服务契约一致，camelCase）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeRef {
    pub kind: String,
    pub key: String,
    pub display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeHit {
    pub path: String,
    pub title: String,
    pub line: u32,
    pub lines: String,
    pub score: f64,
    pub snippet: String,
    #[serde(default)]
    pub refs: Vec<KnowledgeRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeIndexInfo {
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub docs: Option<u64>,
    #[serde(default)]
    pub chunks: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSearchResponse {
    #[serde(default)]
    pub hits: Vec<KnowledgeHit>,
    #[serde(default)]
    pub refs: Vec<KnowledgeRef>,
    #[serde(default)]
    pub total: u32,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub index: Option<KnowledgeIndexInfo>,
    /// 索引根绝对路径（ADR 0017：供前端拼绝对路径做资源管理器定位）。
    #[serde(default)]
    pub root: Option<String>,
}

/// /doc 响应：索引根下文档全文（脱敏工作副本，掩码 token 原样保留）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeDoc {
    #[serde(default)]
    pub ok: bool,
    pub path: String,
    #[serde(default)]
    pub root: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeServiceStatus {
    pub online: bool,
    pub base_url: String,
    pub detail: Option<String>,
    pub vault_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VaultWindow {
    pub active: bool,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub scope: Option<Vec<String>>,
    #[serde(default)]
    pub by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub window: Option<VaultWindow>,
    #[serde(default)]
    pub map_exists: Option<bool>,
}

/// reveal 响应：`value` 是本模块唯一明文字段（仅密钥行短显用）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VaultReveal {
    pub name: String,
    pub label: String,
    #[serde(default)]
    pub sensitivity: Option<String>,
    #[serde(default)]
    pub masked: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
}

/// copy 结果：明文已写剪贴板，响应只带掩码元数据。
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VaultCopyResult {
    pub name: String,
    pub label: String,
    pub sensitivity: Option<String>,
    pub masked: Option<String>,
    pub copied: bool,
}

/// 条目名严格匹配 `[REDACTED:<label>:<hash8>]`（label [A-Za-z0-9_-]+，hash8 小写十六进制），
/// 杜绝任意字符串进入 vault 链路。
fn is_valid_entry_name(entry: &str) -> bool {
    let Some(inner) = entry
        .strip_prefix("[REDACTED:")
        .and_then(|s| s.strip_suffix(']'))
    else {
        return false;
    };
    let Some((label, hash)) = inner.split_once(':') else {
        return false;
    };
    let label_ok = !label.is_empty()
        && label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    let hash_ok = hash.len() == 8
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    label_ok && hash_ok
}

fn clamp_topk(topk: Option<u32>) -> u32 {
    topk.unwrap_or(DEFAULT_TOPK).clamp(1, MAX_TOPK)
}

/// /doc 路径客户端预校验：必须是索引根内相对路径（拒 `..` 段 / 绝对路径 / 盘符 / NUL）；
/// 服务端仍做权威边界校验，这里只拦明显非法输入。
fn is_valid_doc_path(path: &str) -> bool {
    let p = path.trim();
    if p.is_empty() || p.len() > 1024 || p.contains('\0') {
        return false;
    }
    if p.contains(':') || p.starts_with('/') || p.starts_with('\\') {
        return false;
    }
    !p.split(['/', '\\']).any(|seg| seg == "..")
}

// ---------------------------------------------------------------------------
// Tauri 命令
// ---------------------------------------------------------------------------

/// 服务探活：`/health`（无需鉴权）；失败不抛错，由源状态机消费。
#[tauri::command]
pub fn knowledge_service_status() -> KnowledgeServiceStatus {
    let url = format!("{}/health", base_url());
    let request = || {
        let response = agent(HEALTH_TIMEOUT)
            .get(&url)
            .call()
            .map_err(|e| format!("knowledge service health failed ({url}): {e}"))?;
        let status = response.status().as_u16();
        let mut response = response;
        let value: serde_json::Value = response
            .body_mut()
            .read_json()
            .map_err(|e| format!("knowledge service health decode failed: {e}"))?;
        if status >= 400 {
            return Err(format!("knowledge service /health status {status}"));
        }
        Ok(value)
    };
    match request() {
        Ok(value) => KnowledgeServiceStatus {
            online: true,
            base_url: base_url(),
            detail: None,
            vault_configured: value
                .get("vault")
                .and_then(|v| v.get("configured"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        },
        Err(detail) => KnowledgeServiceStatus {
            online: false,
            base_url: base_url(),
            detail: Some(detail),
            vault_configured: false,
        },
    }
}

/// BM25 内容检索（ADR 0017：`dedup = "file"` 时服务端按文件去重出单行）。
#[tauri::command]
pub fn knowledge_service_search(
    query: String,
    topk: Option<u32>,
    dedup: Option<String>,
) -> Result<KnowledgeSearchResponse, KnowledgeError> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(KnowledgeSearchResponse {
            hits: Vec::new(),
            refs: Vec::new(),
            total: 0,
            mode: None,
            index: None,
            root: None,
        });
    }
    let mut body = serde_json::json!({ "query": query, "topk": clamp_topk(topk) });
    if dedup.as_deref() == Some("file") {
        body["dedup"] = serde_json::json!("file");
    }
    let value = post_json("/search", body, SEARCH_TIMEOUT)?;
    serde_json::from_value::<KnowledgeSearchResponse>(value).map_err(|e| {
        err(
            "bad_response",
            format!("unexpected /search payload: {e}"),
            0,
        )
    })
}

/// 读取索引根下文档全文（知识页渲染用）。path 为相对索引根的路径（正斜杠）；
/// 服务端二次校验边界（拒穿越、2MiB 上限）。
#[tauri::command]
pub fn knowledge_service_read_doc(path: String) -> Result<KnowledgeDoc, KnowledgeError> {
    if !is_valid_doc_path(&path) {
        return Err(err(
            "invalid_path",
            "path 必须是索引根内的相对路径（不含 .. / 盘符 / 绝对路径）",
            400,
        ));
    }
    let body = serde_json::json!({ "path": path.trim() });
    let value = post_json("/doc", body, SEARCH_TIMEOUT)?;
    serde_json::from_value::<KnowledgeDoc>(value)
        .map_err(|e| err("bad_response", format!("unexpected /doc payload: {e}"), 0))
}

/// vault 窗口/通道状态（无值）。
#[tauri::command]
pub fn knowledge_service_vault_status() -> Result<VaultStatus, KnowledgeError> {
    let value = post_json("/vault/status", serde_json::json!({}), VAULT_TIMEOUT)?;
    serde_json::from_value::<VaultStatus>(value).map_err(|e| {
        err(
            "bad_response",
            format!("unexpected /vault/status payload: {e}"),
            0,
        )
    })
}

/// 取明文（短显 5s 用）：响应含 value，仅密钥行组件短暂持有，不入 store。
#[tauri::command]
pub fn knowledge_service_reveal(entry: String) -> Result<VaultReveal, KnowledgeError> {
    if !is_valid_entry_name(&entry) {
        return Err(err(
            "invalid_entry",
            "entry 必须形如 [REDACTED:<label>:<hash8>]",
            400,
        ));
    }
    let value = post_json(
        "/vault/reveal",
        serde_json::json!({ "entry": entry }),
        VAULT_TIMEOUT,
    )?;
    serde_json::from_value::<VaultReveal>(value).map_err(|e| {
        err(
            "bad_response",
            format!("unexpected /vault/reveal payload: {e}"),
            0,
        )
    })
}

/// 复制密钥：明文由本命令直写系统剪贴板，响应只带掩码元数据（Enter 主路径）。
#[tauri::command]
pub fn knowledge_service_copy(entry: String) -> Result<VaultCopyResult, KnowledgeError> {
    let reveal = knowledge_service_reveal(entry)?;
    let Some(value) = reveal.value.as_deref() else {
        return Err(err("backend", "reveal response missing value", 0));
    };
    crate::clipboard::set_text(value).map_err(|e| err("clipboard_failed", e, 0))?;
    Ok(VaultCopyResult {
        name: reveal.name,
        label: reveal.label,
        sensitivity: reveal.sensitivity,
        masked: reveal.masked,
        copied: true,
    })
}

/// 开启解锁窗口：必须 `confirm == true`（UI 确认对话框后才允许调用）。
#[tauri::command]
pub fn knowledge_service_unlock(
    ttl: Option<u32>,
    confirm: bool,
) -> Result<VaultStatus, KnowledgeError> {
    if !confirm {
        return Err(err(
            "confirm_required",
            "unlock 必须经用户确认（confirm=true）",
            400,
        ));
    }
    let ttl = ttl.unwrap_or(600).clamp(1, 3600);
    let value = post_json(
        "/vault/unlock",
        serde_json::json!({ "confirm": true, "ttl": ttl }),
        VAULT_TIMEOUT,
    )?;
    serde_json::from_value::<VaultStatus>(value).map_err(|e| {
        err(
            "bad_response",
            format!("unexpected /vault/unlock payload: {e}"),
            0,
        )
    })
}

/// 提前关闭解锁窗口。
#[tauri::command]
pub fn knowledge_service_lock() -> Result<VaultStatus, KnowledgeError> {
    let value = post_json("/vault/lock", serde_json::json!({}), VAULT_TIMEOUT)?;
    serde_json::from_value::<VaultStatus>(value).map_err(|e| {
        err(
            "bad_response",
            format!("unexpected /vault/lock payload: {e}"),
            0,
        )
    })
}

// ---------------------------------------------------------------------------
// 纯函数单测（无网络）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 串行化会改全局（env / CONFIGURED_*）的测试，避免并行互踩。
    static GLOBALS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn normalize_service_url_trims_and_strips_slash() {
        assert_eq!(
            normalize_service_url("  http://127.0.0.1:18790/  ").as_deref(),
            Some("http://127.0.0.1:18790")
        );
        assert_eq!(normalize_service_url("   "), None);
        assert_eq!(normalize_service_url(""), None);
    }

    #[test]
    fn entry_name_validation() {
        assert!(is_valid_entry_name("[REDACTED:llm-key:1a2b3c4d]"));
        assert!(is_valid_entry_name("[REDACTED:pw_1:99887766]"));
        assert!(
            !is_valid_entry_name("[REDACTED:llm-key:1A2B3C4D]"),
            "hash 大写拒绝"
        );
        assert!(
            !is_valid_entry_name("[REDACTED:llm-key:1a2b3c4]"),
            "hash 7 位拒绝"
        );
        assert!(!is_valid_entry_name("见保险库：x [vault:a:1a2b3c4d]"));
        assert!(!is_valid_entry_name("[REDACTED:llm-key]"));
        assert!(!is_valid_entry_name(""));
        assert!(
            !is_valid_entry_name("[REDACTED:a b:1a2b3c4d]"),
            "label 带空格拒绝"
        );
    }

    #[test]
    fn topk_clamped_to_bounds() {
        assert_eq!(clamp_topk(None), 12);
        assert_eq!(clamp_topk(Some(0)), 1);
        assert_eq!(clamp_topk(Some(999)), 50);
        assert_eq!(clamp_topk(Some(20)), 20);
    }

    #[test]
    fn doc_path_validation() {
        assert!(is_valid_doc_path("pages/notes.md"));
        assert!(
            is_valid_doc_path("a\\b.md"),
            "反斜杠相对路径放行（服务端 resolve）"
        );
        assert!(!is_valid_doc_path(""), "空路径拒绝");
        assert!(!is_valid_doc_path("  "), "纯空白拒绝");
        assert!(!is_valid_doc_path("../escape.md"), ".. 段拒绝");
        assert!(!is_valid_doc_path("a/../../escape.md"), "混合 .. 段拒绝");
        assert!(!is_valid_doc_path("a/.."), "尾部 .. 段拒绝");
        assert!(!is_valid_doc_path("C:\\temp\\x.md"), "盘符拒绝");
        assert!(!is_valid_doc_path("C:/temp/x.md"), "POSIX 盘符拒绝");
        assert!(!is_valid_doc_path("\\\\srv\\share\\x.md"), "UNC 拒绝");
        assert!(!is_valid_doc_path("/etc/passwd"), "绝对路径拒绝");
        assert!(!is_valid_doc_path("x.md\0"), "NUL 拒绝");
        assert!(!is_valid_doc_path(&"a".repeat(1025)), "超长拒绝");
    }

    #[test]
    fn parses_doc_response() {
        let value = json!({
            "ok": true, "path": "pages/a.md", "root": "G:/idx",
            "content": "# t\n[vault:pw:1a2b3c4d]"
        });
        let doc: KnowledgeDoc = serde_json::from_value(value).unwrap();
        assert!(doc.ok);
        assert_eq!(doc.path, "pages/a.md");
        assert_eq!(doc.root.as_deref(), Some("G:/idx"));
        assert!(doc.content.contains("[vault:pw:1a2b3c4d]"));
        let minimal: KnowledgeDoc =
            serde_json::from_value(json!({"path": "a.md", "content": "x"})).unwrap();
        assert!(!minimal.ok);
        assert_eq!(minimal.root, None);
    }

    #[test]
    fn error_from_response_prefers_body_code() {
        let e = error_from_response(423, &json!({"code": "unlock_required", "reason": "locked"}));
        assert_eq!(e.code, "unlock_required");
        assert_eq!(e.status, 423);
        let e = error_from_response(500, &json!({}));
        assert_eq!(e.code, "http_500");
    }

    #[test]
    fn parses_search_response_with_refs() {
        let value = json!({
            "hits": [{
                "path": "notes/a.md", "title": "a", "line": 1, "lines": "1-3",
                "score": 0.5, "snippet": "s",
                "refs": [{"kind": "vault", "key": "[REDACTED:k:1a2b3c4d]", "display": "n"}]
            }],
            "refs": [{"kind": "vault", "key": "[REDACTED:k:1a2b3c4d]", "display": "n"}],
            "mode": "bm25-title4",
            "total": 1,
            "index": {"root": "r", "docs": 1, "chunks": 1}
        });
        let parsed: KnowledgeSearchResponse = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.hits.len(), 1);
        assert_eq!(parsed.hits[0].refs[0].kind, "vault");
        assert_eq!(parsed.total, 1);
        assert_eq!(parsed.index.as_ref().unwrap().docs, Some(1));
    }

    #[test]
    fn parses_vault_status_window_or_null() {
        let with_window: VaultStatus =
            serde_json::from_value(json!({"ok": true, "window": {"active": true, "expires_at": "t", "scope": ["*"], "by": "u"}, "map_exists": true}))
                .unwrap();
        assert!(with_window.window.as_ref().unwrap().active);
        let locked: VaultStatus =
            serde_json::from_value(json!({"ok": true, "window": null, "map_exists": true}))
                .unwrap();
        assert!(locked.window.is_none());
    }

    #[test]
    fn base_url_env_overrides_configured() {
        let _g = GLOBALS_LOCK.lock().unwrap();
        let prev = std::env::var_os("AFM_KNOWLEDGE_SERVICE_URL");
        std::env::set_var("AFM_KNOWLEDGE_SERVICE_URL", "http://10.0.0.9:19999/");
        assert_eq!(base_url(), "http://10.0.0.9:19999");
        std::env::remove_var("AFM_KNOWLEDGE_SERVICE_URL");
        set_configured(Some("http://127.0.0.1:18800"), Some("tok"));
        assert_eq!(base_url(), "http://127.0.0.1:18800");
        assert_eq!(token().as_deref(), Some("tok"));
        set_configured(Some(""), Some("  "));
        assert_eq!(base_url(), DEFAULT_BASE_URL);
        assert_eq!(token(), None);
        match prev {
            Some(v) => std::env::set_var("AFM_KNOWLEDGE_SERVICE_URL", v),
            None => std::env::remove_var("AFM_KNOWLEDGE_SERVICE_URL"),
        }
    }

    /// 回归（ADR 0017 解锁链路）：非 2xx 响应体里的 {code} 必须透传到 KnowledgeError——
    /// ureq 3 默认 http_status_as_error=true 会把 423 吞成 Err(StatusCode(u16)) 丢掉响应体，
    /// 导致 unlock_required 变 "offline"、解锁确认框永远不弹。
    #[test]
    fn post_json_carries_non_2xx_body_code() {
        use std::io::{Read, Write};
        let _g = GLOBALS_LOCK.lock().unwrap();
        let prev_env = std::env::var_os("AFM_KNOWLEDGE_SERVICE_URL");
        std::env::remove_var("AFM_KNOWLEDGE_SERVICE_URL");

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            // 读完整请求（头 + Content-Length body）再响应：带未读数据关连接会触发
            // RST（os error 10053），高并发测试下时序敏感。
            let mut req: Vec<u8> = Vec::new();
            let mut chunk = [0u8; 2048];
            let mut header_end: Option<usize> = None;
            let mut content_length = 0usize;
            while req.len() < 64 * 1024 {
                let n = stream.read(&mut chunk).expect("mock read request");
                if n == 0 {
                    break;
                }
                req.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&req);
                if header_end.is_none() {
                    if let Some(pos) = text.find("\r\n\r\n") {
                        header_end = Some(pos + 4);
                        content_length = text
                            .lines()
                            .find_map(|l| {
                                l.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|v| v.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                    }
                }
                if let Some(start) = header_end {
                    if req.len() >= start + content_length {
                        break;
                    }
                }
            }
            let body = r#"{"code":"unlock_required","reason":"vault locked"}"#;
            let resp = format!(
                "HTTP/1.1 423 Locked\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(resp.as_bytes()).unwrap();
            stream.flush().unwrap();
        });
        set_configured(Some(&format!("http://{addr}")), Some("t"));
        let error = post_json(
            "/vault/reveal",
            json!({"entry": "[REDACTED:k:1a2b3c4d]"}),
            Duration::from_secs(5),
        )
        .err()
        .expect("423 必须映射为 KnowledgeError 而非传输错误");
        assert_eq!(error.code, "unlock_required");
        assert_eq!(error.status, 423);
        handle.join().unwrap();
        set_configured(None, None);
        match prev_env {
            Some(v) => std::env::set_var("AFM_KNOWLEDGE_SERVICE_URL", v),
            None => std::env::remove_var("AFM_KNOWLEDGE_SERVICE_URL"),
        }
    }
}
