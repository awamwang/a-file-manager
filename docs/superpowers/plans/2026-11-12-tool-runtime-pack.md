# 工具管理运行时 Pack Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Feature Pack 壳之上交付第一个 Pack——**工具管理运行时**：附着或拉起 awam-tool-manager 网关，统一列出工具与 capability，schema 简表单调用（预览→执行，破坏性确认）。

**Architecture:** AFM 为 MCP/HTTP 薄客户端。Rust 模块 `tool_runtime`（仅在 Pack 启用或开发嵌入时注册命令）用已有 `ureq` 调 `http://127.0.0.1:8766/mcp`（Streamable HTTP + Bearer）。连接偏好与是否自动拉起 sidecar 存 Pack 私有配置目录。前端 `src/packs/tool-runtime/` 提供视图；经 `builtin:tool-runtime` 挂到贡献路由。普通 `build:core` **不**静态挂该侧栏；安装并启用本 Pack 后由贡献系统注入。Open tools / AI 中央仓库不动。

**Tech Stack:** Tauri 2 · Rust `ureq` · Vue 3 · awam-tool-manager HTTP MCP · Feature Pack 壳（前置计划）

**Depends on:** `docs/superpowers/plans/2026-11-12-feature-pack-shell.md` 完成 Task 1–4（类型、IPC、FE 贡献合并）

**Spec:** `docs/glossary-feature-pack.md` · ADR 0013 · 0014

## Global Constraints

- 术语：工具管理运行时、工具管理网关、工具、Capability；勿与 MCP 目录条目混淆
- 网关权威；禁止用本地 JSON 缓存冒充主目录
- 非只读默认预览；破坏性 AFM 确认后 `allowDestructive=true`
- Open tools 长期分立
- 表单 v1：string/number/boolean/enum + JSON 回退
- 提交信息中文；注释中文

---

## File map

| Path | Responsibility |
|------|----------------|
| `src-tauri/src/tool_runtime/mod.rs` | 模块入口、命令注册门闩 |
| `src-tauri/src/tool_runtime/config.rs` | Pack 私有配置读写 |
| `src-tauri/src/tool_runtime/gateway.rs` | 健康检查、附着、MCP tools/call 封装 |
| `src-tauri/src/tool_runtime/sidecar.rs` | 按贡献/配置 spawn 网关进程 |
| `src-tauri/src/tool_runtime/semver_compat.rs` | gatewayCompat 范围检查 |
| `src/packs/tool-runtime/ToolRuntimeView.vue` | 主 UI |
| `src/packs/tool-runtime/CapabilityForm.vue` | 简表单 |
| `src/packs/tool-runtime/client.ts` | invoke 封装 |
| `src/state/featurePack/builtinEntries.ts` | 注册 `builtin:tool-runtime` |
| `packs/tool-runtime/pack.json` | 正式 Pack 清单 |
| `src-tauri/src/lib.rs` | 注册 commands（可始终编译；运行时检查 Pack 启用） |

**打包策略（v1 务实）：**  
源码可在仓库内；`build:core` 的 Vite 仍可包含 `builtinEntries` 映射，但 **无启用 Pack 时不贡献侧栏**——满足「普通版不出现功能」。若需「二进制也不含 UI 字符串」，后续再加 `VITE_EMBED_TOOL_PACK`；本计划不阻塞。

---

### Task 1: Pack 私有配置 + semver 兼容

**Files:**
- Create: `src-tauri/src/tool_runtime/config.rs`
- Create: `src-tauri/src/tool_runtime/semver_compat.rs`
- Create: `src-tauri/src/tool_runtime/mod.rs`

**Interfaces:**
- `ToolRuntimeConfig { baseUrl: String, preferAttach: bool, autoStartSidecar: bool, lastSidecarPid: Option<u32> }`
- 默认 `baseUrl = "http://127.0.0.1:8766"`
- 路径：`{app_data}/feature-packs/awam.tool-runtime/config.json`（id 与 pack.json 一致）
- `fn version_in_range(version: &str, min: &str, max: &str) -> bool` — 简易数字三段比较即可（勿强依赖 semver crate，除非已有）

- [ ] **Step 1: 单测 version_in_range**

```rust
#[test]
fn version_in_range_inclusive() {
    assert!(version_in_range("1.2.0", "1.0.0", "2.0.0"));
    assert!(!version_in_range("2.1.0", "1.0.0", "2.0.0"));
}
```

- [ ] **Step 2: 红 → 实现 → 绿**

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(tool-runtime): Pack 配置与网关版本范围校验"
```

---

### Task 2: 网关附着 / 健康检查

**Files:**
- Create: `src-tauri/src/tool_runtime/gateway.rs`
- Modify: `mod.rs`

**Interfaces:**
- `fn read_http_token(runtime_dir: &Path) -> Result<String, String>` — 读 `%LOCALAPPDATA%\.awam-tool-manager\http-token`（或 `AWAM_RUNTIME_DIR`）；与 awam-tool-manager 约定一致
- `fn health_check(base_url: &str) -> Result<HealthOk, String>` — `GET {base}/health`（无 token 亦可，以网关实现为准）
- `GatewayStatus { reachable: bool, detail: String, version: Option<String> }`

- [ ] **Step 1: 单测 token 文件读取（temp 文件）**

- [ ] **Step 2: 实现 ureq GET health；超时短（如 1–2s）**

- [ ] **Step 3: IPC `tool_runtime_gateway_status`**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(tool-runtime): 网关健康检查与 token 读取"
```

---

### Task 3: Sidecar 拉起

**Files:**
- Create: `src-tauri/src/tool_runtime/sidecar.rs`

**Interfaces:**
- `fn start_sidecar(app_data, pack_install_dir, contrib: &SidecarContribution) -> Result<u32, String>`
- 需要权限：已启用 Pack 含 `spawnSidecar`
- 命令来自 pack 贡献或配置覆盖；工作目录相对 pack 安装目录
- 先 `health_check`；已 reachable 则不重复拉起（附着优先）

- [ ] **Step 1: 单测 — reachable 时 start 返回 Ok 且不 spawn（mock 或注入 probe 闭包）**

将 `health_check` 做成可注入函数指针/`trait GatewayProbe` 便于测。

- [ ] **Step 2: 实现；IPC `tool_runtime_ensure_gateway`**

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(tool-runtime): 优先附着否则拉起网关 sidecar"
```

---

### Task 4: MCP 调用封装（list / capabilities / call）

**Files:**
- Modify: `gateway.rs`
- Modify: `mod.rs` / `lib.rs`

**Interfaces:**
- 对 Streamable HTTP MCP：实现最小子集——`initialize`（若需要）、`tools/call` 调网关的 `list_tools`、`list_capabilities`、`call_capability`、`get_tool_info`
- 具体 JSON-RPC/SSE 形状以 `G:\Projects\19AI\skills\awam-tool-manager\src\http_transport.py` 与现网抓包为准；**实现前先用子代理或本地 curl 确认一帧请求格式**，把样例请求写入本 Task 注释或 `docs/tool-runtime-mcp-notes.md`
- IPC：
  - `tool_runtime_list_tools(kind?: String)`
  - `tool_runtime_list_capabilities(toolId, refresh?: bool)`
  - `tool_runtime_call_capability({ toolId, capabilityId, arguments, execute, allowDestructive })`
- 调用前：`ensure_gateway`；`gatewayCompat` 不匹配则返回明确错误，不盲调
- Pack 未启用：所有 IPC 返回 `tool_runtime_disabled`

- [ ] **Step 1: 用本地网关或录制 fixture 写 `call_capability` 解析测试**

- [ ] **Step 2: 实现客户端**

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(tool-runtime): 经网关 list/call capability"
```

---

### Task 5: 前端主界面 + 表单

**Files:**
- Create: `src/packs/tool-runtime/client.ts`
- Create: `src/packs/tool-runtime/CapabilityForm.vue`
- Create: `src/packs/tool-runtime/ToolRuntimeView.vue`
- Modify: `builtinEntries.ts` → `'builtin:tool-runtime': () => import('../../packs/tool-runtime/ToolRuntimeView.vue')`
- Create: `packs/tool-runtime/pack.json`
- i18n keys：`nav.toolRuntime`、连接态、预览/执行、破坏性确认

**UI 流程：**
1. 顶部：网关状态；按钮「附着/启动」
2. 左：工具列表（search 可二期；v1 用 list + 本地 filter）
3. 右：capability 列表 → 点开表单
4. `CapabilityForm`：按 inputSchema.properties 渲染 string/number/boolean/enum；否则 JSON textarea
5. 只读：直接调用 `execute=true`；否则先 `execute=false` 展示预览，确认后执行
6. 若返回要求 destructive：Modal 确认后再 `allowDestructive=true`
7. 可选：从 navigator 选中路径写入 `path`/`paths`（读既有 selection store；无则跳过）

- [ ] **Step 1: pack.json 贡献 `/tools` 侧栏 + route entry `builtin:tool-runtime`**

```json
{
  "id": "awam.tool-runtime",
  "name": "工具管理运行时",
  "version": "0.1.0",
  "apiVersion": "feature-pack.1",
  "permissions": ["network", "spawnSidecar"],
  "gatewayCompat": { "minVersion": "0.1.0", "maxVersion": "1.0.0" },
  "contributions": {
    "sideNav": [{ "id": "tool-runtime", "labelKey": "nav.toolRuntime", "to": "/tools", "order": 45 }],
    "routes": [{ "path": "/tools", "name": "tool-runtime", "entry": "builtin:tool-runtime" }],
    "commands": [{ "id": "tool-runtime.open", "labelKey": "cmd.toolRuntimeOpen", "route": "/tools" }],
    "sidecars": [{
      "id": "awam-tool-manager",
      "command": "python",
      "args": ["-m", "awam_tool_manager_placeholder"],
      "workingDirRel": "sidecar"
    }]
  }
}
```

`sidecars` 的真实 command 在实现时改为 awam-tool-manager 实际启动行（`server.py --transport http`），以该仓库 README 为准。

- [ ] **Step 2: 实现 View + Form + client**

- [ ] **Step 3: 安装 Pack（Extensions）后手动验收连接与一次只读 call**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(tool-runtime): 工具管理运行时 UI 与 Pack 清单"
```

---

### Task 6: 门禁与回归

**Files:**
- Create: `scripts/check-tool-runtime-pack.mjs` — 断言 `pack.json` id、`builtinEntries` 含 `builtin:tool-runtime`、Core 路径不 import AI 术语混用
- Modify: `package.json` scripts 可选挂接
- Modify: `docs/a-file-manager-feature-map.md` 一行指向 `/tools`（Pack）

- [ ] **Step 1: 脚本 + `cargo test` + 相关 check**
- [ ] **Step 2: Commit**

```bash
git commit -m "test: 工具管理运行时 Pack 静态门禁"
```

---

## Self-review

| Spec 点 | Task |
|---------|------|
| 薄客户端 + call_capability | 4–5 |
| 附着否则 sidecar | 2–3 |
| 独立侧栏 /tools | 5 |
| Open tools 分立 | 全局 |
| 破坏性 AFM 确认 | 5 |
| 预览再执行 | 5 |
| 路径预填可选 | 5 |
| 无 JSON 假目录 | 2–4 |
| Pack 私有配置 | 1 |
| 版本范围 | 1、4 |
| 表单简单类型 | 5 |
| 本机 Pack 安装 | 依赖壳计划 + 5 的 pack.json |

**前置缺口：** 若壳计划未完成 zip，可用「从目录安装 `packs/tool-runtime`」验收。
