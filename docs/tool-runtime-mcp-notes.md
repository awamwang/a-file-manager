# 工具管理网关 Streamable HTTP MCP 请求形状

对 `awam-tool-manager`（`python server.py --transport http`）的最小客户端约定，来源：`src/http_transport.py`、MCP Python SDK `streamable_http`、README。本机 8766 现被其它服务占用，未抓到活会话；形状与 SDK / e2e（`initialize` 后 `tools/call`）一致。

- **URL**：`POST {baseUrl}/mcp/`（默认 `http://127.0.0.1:8766/mcp/`；须带尾斜杠，避免 Starlette Mount 对 `/mcp` 的 307 导致客户端 redirect failed）
- **鉴权**：`Authorization: Bearer <token>`（`AWAM_HTTP_TOKEN` 或 `%LOCALAPPDATA%\.awam-tool-manager\http-token`）
- **必填头**：`Content-Type: application/json`；`Accept: application/json, text/event-stream`（缺 SSE 会被 406）
- **会话**：`initialize` 响应头 `mcp-session-id` 须带回后续 POST；通知 `notifications/initialized` 返回 202
- **响应**：`application/json` 或 SSE `event: message` + `data: <jsonrpc>`

样例（`tools/call` → 网关 `call_capability`）：

```http
POST /mcp HTTP/1.1
Host: 127.0.0.1:8766
Authorization: Bearer <http-token>
Content-Type: application/json
Accept: application/json, text/event-stream
mcp-session-id: <uuid-from-initialize>

{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"call_capability","arguments":{"tool_id":"everything-search-mcp","capability_id":"search_files","arguments":{"query":"测试"},"execute":false,"allowDestructive":false}}}
```
