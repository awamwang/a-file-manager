// SPDX-License-Identifier: GPL-3.0-or-later
//! 对 awam-tool-manager 的最小 Streamable HTTP MCP 客户端（initialize + tools/call）。

use serde_json::{json, Value};
use std::time::Duration;
use ureq::Agent;

/// MCP Streamable HTTP 路径，与 awam-tool-manager `MCP_PATH` 一致。
pub const MCP_PATH: &str = "/mcp";

/// 与 mcp 1.29/1.30 默认协商版本一致。
const PROTOCOL_VERSION: &str = "2025-03-26";

const MCP_TIMEOUT: Duration = Duration::from_secs(30);

/// Streamable HTTP MCP 会话：先 `initialize`，再 `tools/call` 网关工具。
pub struct McpClient {
    agent: Agent,
    mcp_url: String,
    token: String,
    session_id: Option<String>,
    protocol_version: Option<String>,
    server_version: Option<String>,
    next_id: u64,
}

impl McpClient {
    /// 建立会话：`initialize` + `notifications/initialized`。
    pub fn connect(base_url: &str, token: &str) -> Result<Self, String> {
        let mut client = Self {
            agent: Agent::config_builder()
                .timeout_global(Some(MCP_TIMEOUT))
                .build()
                .into(),
            mcp_url: mcp_url(base_url)?,
            token: token.to_string(),
            session_id: None,
            protocol_version: None,
            server_version: None,
            next_id: 1,
        };
        client.initialize()?;
        client.notify_initialized()?;
        Ok(client)
    }

    /// `initialize` 结果里的 `serverInfo.version`（/health 无 version 时用于 gatewayCompat）。
    pub fn server_version(&self) -> Option<&str> {
        self.server_version.as_deref()
    }

    /// 调用网关 `list_tools`（可选 `kind`：app / mcp / launcher）。
    pub fn list_tools(&mut self, kind: Option<&str>) -> Result<Value, String> {
        let mut arguments = json!({});
        if let Some(kind) = kind.map(str::trim).filter(|value| !value.is_empty()) {
            arguments["kind"] = json!(kind);
        }
        self.call_gateway_tool("list_tools", arguments)
    }

    /// 调用网关 `list_capabilities`。
    pub fn list_capabilities(&mut self, tool_id: &str, refresh: Option<bool>) -> Result<Value, String> {
        let mut arguments = json!({ "tool_id": tool_id });
        if let Some(refresh) = refresh {
            arguments["refresh"] = json!(refresh);
        }
        self.call_gateway_tool("list_capabilities", arguments)
    }

    /// 调用网关 `call_capability`。`allowDestructive` 字段名与网关 schema 一致。
    pub fn call_capability(
        &mut self,
        tool_id: &str,
        capability_id: &str,
        arguments: Option<&Value>,
        execute: Option<bool>,
        allow_destructive: Option<bool>,
    ) -> Result<Value, String> {
        let mut params = json!({
            "tool_id": tool_id,
            "capability_id": capability_id,
            "arguments": arguments.cloned().unwrap_or_else(|| json!({})),
        });
        if let Some(execute) = execute {
            params["execute"] = json!(execute);
        }
        if let Some(allow_destructive) = allow_destructive {
            params["allowDestructive"] = json!(allow_destructive);
        }
        self.call_gateway_tool("call_capability", params)
    }

    /// 调用网关 `get_tool_info`（IPC 未暴露，供内部按需取详情）。
    #[allow(dead_code)]
    pub fn get_tool_info(&mut self, tool_id: &str) -> Result<Value, String> {
        self.call_gateway_tool("get_tool_info", json!({ "tool_id": tool_id }))
    }

    fn call_gateway_tool(&mut self, name: &str, arguments: Value) -> Result<Value, String> {
        let envelope = self.rpc("tools/call", json!({ "name": name, "arguments": arguments }))?;
        let result = jsonrpc_result(&envelope)?;
        extract_call_tool_payload(&result)
    }

    fn initialize(&mut self) -> Result<(), String> {
        let envelope = self.rpc(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "a-file-manager",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }),
        )?;
        let result = jsonrpc_result(&envelope)?;
        if let Some(version) = result
            .get("protocolVersion")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            self.protocol_version = Some(version.to_string());
        }
        if let Some(version) = result
            .pointer("/serverInfo/version")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            self.server_version = Some(version.to_string());
        }
        Ok(())
    }

    fn notify_initialized(&mut self) -> Result<(), String> {
        let body = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        let _ = self.post_rpc(&body)?;
        Ok(())
    }

    fn rpc(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        let http = self.post_rpc(&body)?;
        if method == "initialize" {
            if let Some(session_id) = http.session_id {
                self.session_id = Some(session_id);
            }
        }
        parse_mcp_http_body(&http.content_type, &http.body)
    }

    fn post_rpc(&self, body: &Value) -> Result<RpcHttp, String> {
        let mut request = self
            .agent
            .post(&self.mcp_url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/json, text/event-stream")
            .header("Content-Type", "application/json");
        if let Some(session_id) = self.session_id.as_deref() {
            request = request.header("mcp-session-id", session_id);
        }
        if let Some(protocol_version) = self.protocol_version.as_deref() {
            request = request.header("mcp-protocol-version", protocol_version);
        }
        let mut response = request.send_json(body).map_err(map_ureq_error)?;
        let status = response.status().as_u16();
        let session_id = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let body = if status == 202 {
            String::new()
        } else {
            response.body_mut().read_to_string().map_err(|err| {
                format!("tool-runtime MCP read body failed ({}): {err}", self.mcp_url)
            })?
        };
        Ok(RpcHttp {
            session_id,
            content_type,
            body,
        })
    }
}

struct RpcHttp {
    session_id: Option<String>,
    content_type: String,
    body: String,
}

fn mcp_url(base_url: &str) -> Result<String, String> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("tool-runtime gateway base URL is empty".to_string());
    }
    Ok(format!("{trimmed}{MCP_PATH}"))
}

fn map_ureq_error(err: ureq::Error) -> String {
    match err {
        ureq::Error::StatusCode(401) => {
            "tool-runtime gateway unauthorized (Bearer token)".to_string()
        }
        ureq::Error::StatusCode(code) => format!("tool-runtime MCP HTTP {code}"),
        other => format!("tool-runtime MCP request failed: {other}"),
    }
}

/// 解析 JSON 或 SSE（`event: message` / `data:`）中的 JSON-RPC 对象。
pub(crate) fn parse_mcp_http_body(content_type: &str, body: &str) -> Result<Value, String> {
    let ct = content_type.to_ascii_lowercase();
    let trimmed = body.trim_start();
    if ct.contains("text/event-stream")
        || trimmed.starts_with("event:")
        || trimmed.starts_with("data:")
    {
        return parse_sse_jsonrpc(body);
    }
    serde_json::from_str(body.trim()).map_err(|err| {
        format!("tool-runtime MCP JSON decode failed: {err}")
    })
}

fn parse_sse_jsonrpc(body: &str) -> Result<Value, String> {
    let mut event = String::new();
    let mut data = String::new();
    let mut frames = Vec::new();
    for line in body.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            if !data.is_empty() {
                frames.push((std::mem::take(&mut event), std::mem::take(&mut data)));
            }
            event.clear();
            data.clear();
            continue;
        }
        if let Some(rest) = line.strip_prefix("event:") {
            event = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("data:") {
            let payload = rest.strip_prefix(' ').unwrap_or(rest);
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(payload);
        }
    }
    if !data.is_empty() {
        frames.push((event, data));
    }
    for (event, payload) in frames {
        if !(event.is_empty() || event == "message") {
            continue;
        }
        if payload.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(&payload)
            .map_err(|err| format!("tool-runtime MCP SSE JSON decode failed: {err}"))?;
        if value.get("result").is_some() || value.get("error").is_some() {
            return Ok(value);
        }
    }
    Err("tool-runtime MCP SSE stream has no JSON-RPC result".to_string())
}

fn jsonrpc_result(message: &Value) -> Result<Value, String> {
    if let Some(error) = message.get("error") {
        let msg = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("JSON-RPC error");
        return Err(format!("mcp jsonrpc error: {msg}"));
    }
    message
        .get("result")
        .cloned()
        .ok_or_else(|| "mcp response missing result".to_string())
}

/// 从 `tools/call` 结果取网关载荷：优先 `structuredContent`，否则解析 text JSON。
pub(crate) fn extract_call_tool_payload(result: &Value) -> Result<Value, String> {
    let is_error = result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let payload = if let Some(structured) = result.get("structuredContent") {
        structured.clone()
    } else if let Some(text) = result.get("content").and_then(Value::as_array).and_then(|items| {
        items.iter().find_map(|item| {
            item.get("text")
                .and_then(Value::as_str)
                .and_then(|text| serde_json::from_str::<Value>(text).ok())
        })
    }) {
        text
    } else {
        result.clone()
    };
    if is_error {
        let msg = payload
            .get("error")
            .and_then(Value::as_str)
            .or_else(|| payload.get("message").and_then(Value::as_str))
            .unwrap_or("tool call failed");
        return Err(format!("mcp tool error: {msg}"));
    }
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::{Duration, Instant};

    const PREVIEW_JSONRPC: &str = include_str!("fixtures/call_capability_preview.json");
    const PREVIEW_SSE: &str = include_str!("fixtures/call_capability_sse.txt");

    #[test]
    fn parse_call_capability_structured_content_fixture() {
        let envelope: Value = serde_json::from_str(PREVIEW_JSONRPC).unwrap();
        let result = jsonrpc_result(&envelope).unwrap();
        let payload = extract_call_tool_payload(&result).unwrap();
        assert_eq!(payload["success"], json!(true));
        assert_eq!(payload["toolId"], json!("everything-search-mcp"));
        assert_eq!(payload["capabilityId"], json!("search_files"));
        assert_eq!(payload["executed"], json!(false));
        assert_eq!(payload["arguments"]["query"], json!("测试"));
    }

    #[test]
    fn parse_call_capability_sse_fixture() {
        let envelope = parse_mcp_http_body("text/event-stream", PREVIEW_SSE).unwrap();
        let payload = extract_call_tool_payload(&jsonrpc_result(&envelope).unwrap()).unwrap();
        assert_eq!(payload["executed"], json!(false));
        assert_eq!(payload["capabilityId"], json!("search_files"));
    }

    #[test]
    fn extract_call_tool_payload_prefers_text_when_structured_missing() {
        let result = json!({
            "content": [{ "type": "text", "text": "{\"success\":true,\"executed\":false}" }],
            "isError": false
        });
        let payload = extract_call_tool_payload(&result).unwrap();
        assert_eq!(payload["executed"], json!(false));
    }

    #[test]
    fn extract_call_tool_payload_is_error() {
        let result = json!({
            "content": [{ "type": "text", "text": "{\"success\":false,\"error\":\"能力被标记为 destructive\"}" }],
            "structuredContent": { "success": false, "error": "能力被标记为 destructive" },
            "isError": true
        });
        let err = extract_call_tool_payload(&result).unwrap_err();
        assert!(err.contains("destructive"), "{err}");
    }

    fn spawn_fixture_gateway() -> (String, String) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        // 阻塞 accept，避免非阻塞轮询下客户端连入被对端中止（Windows 10053）。
        listener.set_nonblocking(false).unwrap();
        let addr = listener.local_addr().unwrap();
        let token = "fixture-token";
        thread::spawn(move || {
            // 短生命周期 fixture：处理若干连接后随测试进程结束。
            for _ in 0..16 {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = stream.set_nodelay(true);
                        let _ = handle_fixture_conn(&mut stream, token);
                    }
                    Err(_) => break,
                }
            }
        });
        // 给监听线程一点启动时间，降低首连竞态。
        thread::sleep(Duration::from_millis(20));
        (format!("http://{addr}"), token.to_string())
    }

    fn handle_fixture_conn(stream: &mut std::net::TcpStream, token: &str) -> std::io::Result<()> {
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        let (request_line, headers, body) = read_http_request(stream)?;
        if request_line.starts_with("GET ") && request_line.contains("/health") {
            write_http(
                stream,
                "200 OK",
                &[("Content-Type", "application/json")],
                r#"{"status":"ok","transport":"http","version":"0.1.0"}"#,
            )?;
            return Ok(());
        }
        let auth = headers
            .iter()
            .find(|(k, _)| k == "authorization")
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if auth != format!("Bearer {token}") {
            write_http(
                stream,
                "401 Unauthorized",
                &[("Content-Type", "application/json")],
                r#"{"error":"unauthorized"}"#,
            )?;
            return Ok(());
        }
        let accept = headers
            .iter()
            .find(|(k, _)| k == "accept")
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if !(accept.contains("application/json") && accept.contains("text/event-stream")) {
            write_http(stream, "406 Not Acceptable", &[], "")?;
            return Ok(());
        }
        let rpc: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
        let method = rpc.get("method").and_then(Value::as_str).unwrap_or("");
        match method {
            "initialize" => {
                write_http(
                    stream,
                    "200 OK",
                    &[
                        ("Content-Type", "application/json"),
                        ("mcp-session-id", "fixture-session"),
                    ],
                    r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-03-26","capabilities":{"tools":{}},"serverInfo":{"name":"awam-tool-manager","version":"0.1.0"}}}"#,
                )?;
            }
            "notifications/initialized" => {
                write_http(stream, "202 Accepted", &[], "")?;
            }
            "tools/call" => {
                let name = rpc
                    .pointer("/params/name")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let session = headers
                    .iter()
                    .find(|(k, _)| k == "mcp-session-id")
                    .map(|(_, v)| v.as_str())
                    .unwrap_or("");
                assert_eq!(session, "fixture-session", "tools/call must send mcp-session-id");
                if name == "call_capability" {
                    write_http(
                        stream,
                        "200 OK",
                        &[("Content-Type", "text/event-stream")],
                        PREVIEW_SSE,
                    )?;
                } else if name == "list_tools" {
                    write_http(
                        stream,
                        "200 OK",
                        &[("Content-Type", "application/json")],
                        r#"{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"{\"success\":true,\"count\":1,\"tools\":[{\"id\":\"everything-search-mcp\",\"kind\":\"mcp\"}]}"}],"structuredContent":{"success":true,"count":1,"tools":[{"id":"everything-search-mcp","kind":"mcp"}]},"isError":false}}"#,
                    )?;
                } else {
                    write_http(
                        stream,
                        "200 OK",
                        &[("Content-Type", "application/json")],
                        r#"{"jsonrpc":"2.0","id":2,"result":{"structuredContent":{"success":true},"isError":false}}"#,
                    )?;
                }
            }
            _ => {
                write_http(stream, "400 Bad Request", &[], "")?;
            }
        }
        Ok(())
    }

    fn read_http_request(
        stream: &mut std::net::TcpStream,
    ) -> std::io::Result<(String, Vec<(String, String)>, Vec<u8>)> {
        let mut buf = Vec::new();
        let mut tmp = [0u8; 2048];
        loop {
            let n = stream.read(&mut tmp)?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                let header = String::from_utf8_lossy(&buf[..pos]);
                let mut lines = header.split("\r\n");
                let request_line = lines.next().unwrap_or("").to_string();
                let mut headers = Vec::new();
                let mut content_length = 0usize;
                for line in lines {
                    if let Some((k, v)) = line.split_once(':') {
                        let key = k.trim().to_ascii_lowercase();
                        let val = v.trim().to_string();
                        if key == "content-length" {
                            content_length = val.parse().unwrap_or(0);
                        }
                        headers.push((key, val));
                    }
                }
                let mut body = buf[pos + 4..].to_vec();
                while body.len() < content_length {
                    let n = stream.read(&mut tmp)?;
                    if n == 0 {
                        break;
                    }
                    body.extend_from_slice(&tmp[..n]);
                }
                body.truncate(content_length);
                return Ok((request_line, headers, body));
            }
            if buf.len() > 64 * 1024 {
                break;
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "incomplete HTTP request",
        ))
    }

    fn write_http(
        stream: &mut std::net::TcpStream,
        status: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> std::io::Result<()> {
        let mut extra = String::new();
        for (k, v) in headers {
            extra.push_str(&format!("{k}: {v}\r\n"));
        }
        let resp = format!(
            "HTTP/1.0 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n{body}",
            body.len()
        );
        stream.write_all(resp.as_bytes())
    }

    #[test]
    fn call_capability_roundtrip_against_fixture_gateway() {
        let (base, token) = spawn_fixture_gateway();
        let mut client = McpClient::connect(&base, &token).unwrap();
        assert_eq!(client.server_version(), Some("0.1.0"));
        let payload = client
            .call_capability(
                "everything-search-mcp",
                "search_files",
                Some(&json!({ "query": "测试" })),
                Some(false),
                Some(false),
            )
            .unwrap();
        assert_eq!(payload["success"], json!(true));
        assert_eq!(payload["executed"], json!(false));
        assert_eq!(payload["toolId"], json!("everything-search-mcp"));
    }

    #[test]
    fn list_tools_roundtrip_against_fixture_gateway() {
        let (base, token) = spawn_fixture_gateway();
        let mut client = McpClient::connect(&base, &token).unwrap();
        let payload = client.list_tools(Some("mcp")).unwrap();
        assert_eq!(payload["count"], json!(1));
        assert_eq!(payload["tools"][0]["id"], json!("everything-search-mcp"));
        let info = client.get_tool_info("everything-search-mcp").unwrap();
        assert_eq!(info["success"], json!(true));
    }

    #[test]
    fn mcp_url_appends_path() {
        assert_eq!(mcp_url("http://127.0.0.1:8766/").unwrap(), "http://127.0.0.1:8766/mcp");
    }
}
