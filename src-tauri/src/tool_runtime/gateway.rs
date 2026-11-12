// SPDX-License-Identifier: GPL-3.0-or-later

use super::config::{load_from_path, DEFAULT_BASE_URL};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use ureq::Agent;

/// 与 awam-tool-manager 运行时目录中的 token 文件名一致。
pub const TOKEN_FILE_NAME: &str = "http-token";

/// 覆盖运行时目录的环境变量（与 awam-tool-manager `AWAM_RUNTIME_DIR` 一致）。
pub const RUNTIME_DIR_ENV: &str = "AWAM_RUNTIME_DIR";

/// Bearer token 环境变量（与 awam-tool-manager `AWAM_HTTP_TOKEN` 一致，优先于 token 文件）。
pub const TOKEN_ENV_VAR: &str = "AWAM_HTTP_TOKEN";

const RUNTIME_DIR_NAME: &str = ".awam-tool-manager";

/// 探活超时：网关未启动时应尽快失败。
const HEALTH_TIMEOUT: Duration = Duration::from_secs(2);

/// 工具管理网关 `/health` 成功响应。
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct HealthOk {
    pub status: String,
    #[serde(default)]
    pub transport: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

/// 工具管理网关附着状态（IPC 返回）。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GatewayStatus {
    pub reachable: bool,
    pub detail: String,
    pub version: Option<String>,
}

/// 解析 awam-tool-manager 运行时目录：`AWAM_RUNTIME_DIR` 优先，否则 `%LOCALAPPDATA%\.awam-tool-manager`。
pub fn resolve_runtime_dir() -> PathBuf {
    if let Ok(override_dir) = std::env::var(RUNTIME_DIR_ENV) {
        let trimmed = override_dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let trimmed = local.trim();
        if !trimmed.is_empty() {
            return Path::new(trimmed).join(RUNTIME_DIR_NAME);
        }
    }
    home_dir().join(RUNTIME_DIR_NAME)
}

fn home_dir() -> PathBuf {
    for key in ["USERPROFILE", "HOME"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed);
            }
        }
    }
    PathBuf::from(".")
}

/// 解析 HTTP Bearer token：`AWAM_HTTP_TOKEN` 优先，否则读运行时目录 `http-token`。
pub fn resolve_http_token(runtime_dir: &Path) -> Result<String, String> {
    resolve_http_token_with_env(
        runtime_dir,
        std::env::var(TOKEN_ENV_VAR).ok().as_deref(),
    )
}

pub(crate) fn resolve_http_token_with_env(
    runtime_dir: &Path,
    env_token: Option<&str>,
) -> Result<String, String> {
    if let Some(token) = env_token.map(str::trim).filter(|value| !value.is_empty()) {
        return Ok(token.to_string());
    }
    read_http_token(runtime_dir)
}

/// 读取运行时目录下的 `http-token`（去除首尾空白）。
pub fn read_http_token(runtime_dir: &Path) -> Result<String, String> {
    let path = runtime_dir.join(TOKEN_FILE_NAME);
    let text = fs::read_to_string(&path).map_err(|err| {
        format!(
            "Failed to read HTTP token ({path}): {err}",
            path = path.display()
        )
    })?;
    let token = text.trim().to_string();
    if token.is_empty() {
        return Err(format!(
            "HTTP token file is empty ({path})",
            path = path.display()
        ));
    }
    Ok(token)
}

fn health_url(base_url: &str) -> Result<String, String> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("tool-runtime gateway base URL is empty".to_string());
    }
    Ok(format!("{trimmed}/health"))
}

fn http_agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(HEALTH_TIMEOUT))
        .build()
        .into()
}

/// `GET {base}/health`；`/health` 免鉴权，与 awam-tool-manager 约定一致。
pub fn health_check(base_url: &str) -> Result<HealthOk, String> {
    let url = health_url(base_url)?;
    let mut response = http_agent()
        .get(&url)
        .call()
        .map_err(|err| format!("tool-runtime gateway request failed ({url}): {err}"))?;
    let ok: HealthOk = response
        .body_mut()
        .read_json()
        .map_err(|err| format!("tool-runtime gateway JSON decode failed ({url}): {err}"))?;
    if !ok.status.eq_ignore_ascii_case("ok") {
        return Err(format!(
            "tool-runtime gateway /health unexpected status: {}",
            ok.status
        ));
    }
    Ok(ok)
}

/// 将探活结果转为附着状态（失败时仍返回结构体，不走 IPC Err）。
pub fn query_gateway_status(base_url: &str) -> GatewayStatus {
    match health_check(base_url) {
        Ok(ok) => GatewayStatus {
            reachable: true,
            detail: ok
                .transport
                .as_deref()
                .map(|transport| format!("{} ({transport})", ok.status))
                .unwrap_or_else(|| ok.status.clone()),
            version: ok.version,
        },
        Err(detail) => GatewayStatus {
            reachable: false,
            detail,
            version: None,
        },
    }
}

pub(crate) fn load_base_url(app_data_dir: &Path) -> String {
    let path = super::config::config_file_path(app_data_dir);
    if !path.exists() {
        return DEFAULT_BASE_URL.to_string();
    }
    load_from_path(app_data_dir)
        .map(|cfg| cfg.base_url)
        .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn write_token(dir: &Path, contents: &str) {
        fs::write(dir.join(TOKEN_FILE_NAME), contents).unwrap();
    }

    #[test]
    fn read_http_token_from_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        write_token(dir.path(), "secret-token\r\n");
        assert_eq!(read_http_token(dir.path()).unwrap(), "secret-token");
    }

    #[test]
    fn resolve_http_token_prefers_env_over_file() {
        let dir = tempfile::tempdir().unwrap();
        write_token(dir.path(), "file-token\n");
        assert_eq!(
            resolve_http_token_with_env(dir.path(), Some(" env-token \n")).unwrap(),
            "env-token"
        );
        assert_eq!(
            resolve_http_token_with_env(dir.path(), Some("  ")).unwrap(),
            "file-token"
        );
    }

    #[test]
    fn read_http_token_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let err = read_http_token(dir.path()).unwrap_err();
        assert!(err.contains("Failed to read HTTP token"), "{err}");
    }

    #[test]
    fn read_http_token_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        write_token(dir.path(), "  \n");
        let err = read_http_token(dir.path()).unwrap_err();
        assert!(err.contains("empty"), "{err}");
    }

    fn serve_once(status_line: &'static str, body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 2048];
            let _ = stream.read(&mut buf);
            let resp = format!(
                "HTTP/1.0 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes());
        });
        format!("http://{addr}")
    }

    #[test]
    fn health_check_ok_payload() {
        let base = serve_once("200 OK", r#"{"status":"ok","transport":"http"}"#);
        let ok = health_check(&base).unwrap();
        assert_eq!(ok.status, "ok");
        assert_eq!(ok.transport.as_deref(), Some("http"));
        assert!(ok.version.is_none());
    }

    #[test]
    fn query_gateway_status_reachable() {
        let base = serve_once(
            "200 OK",
            r#"{"status":"ok","transport":"http","version":"0.1.0"}"#,
        );
        let status = query_gateway_status(&base);
        assert!(status.reachable);
        assert!(status.detail.contains("ok"));
        assert_eq!(status.version.as_deref(), Some("0.1.0"));
    }

    #[test]
    fn health_check_rejects_non_ok_status() {
        let base = serve_once("200 OK", r#"{"status":"degraded"}"#);
        let err = health_check(&base).unwrap_err();
        assert!(err.contains("unexpected status"), "{err}");
    }

    #[test]
    fn health_check_unreachable() {
        let err = health_check("http://127.0.0.1:1").unwrap_err();
        assert!(err.contains("request failed"), "{err}");
        let status = query_gateway_status("http://127.0.0.1:1");
        assert!(!status.reachable);
        assert!(status.version.is_none());
    }

    #[test]
    fn health_url_strips_trailing_slash() {
        assert_eq!(
            health_url("http://127.0.0.1:8766/").unwrap(),
            "http://127.0.0.1:8766/health"
        );
    }

    #[test]
    fn load_base_url_default_when_config_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_base_url(dir.path()), DEFAULT_BASE_URL);
        assert!(!dir.path().join("feature-packs").exists());
    }
}
