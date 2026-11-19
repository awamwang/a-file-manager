# Feature Pack 清单格式（`pack.json`）

v1 安装入口为本机 zip 或目录；包根目录需包含 `pack.json`，`apiVersion` 固定为 `feature-pack.1`。JSON 键使用 camelCase，与 Rust `FeaturePackManifest` 序列化一致。样例：`packs/_template/pack.json`。

## 根字段

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 包标识，`^[A-Za-z0-9._-]+$` |
| `name` | string | 是 | 显示名称 |
| `version` | string | 是 | 包版本（semver 字符串） |
| `apiVersion` | string | 是 | 清单 API，当前为 `feature-pack.1` |
| `description` | string | 否 | 简介，默认空 |
| `permissions` | string[] | 否 | 权限：`uiCommands`、`fsProvider`、`network`、`spawnSidecar` |
| `contributions` | object | 是 | 见下表 |
| `gatewayCompat` | object | 否 | `minVersion` / `maxVersion`（semver，被连接方的版本兼容区间）。这是 versionCompat 的**唯一实现**：每次网关 MCP 调用前由 `tool_runtime`（`semver_compat::check_gateway_compat`）用实例上报版本比对；版本未知/非 `x.y.z` 三元组时**降级跳过**（不阻断）。managed 自构建产物版本构建即锁定，天然在区间内。 |

## `contributions`

| 字段 | 类型 | 说明 |
|------|------|------|
| `sideNav` | array | 侧栏项：`id`、`labelKey`、`to`、`order` |
| `routes` | array | 路由：`path`、`name`、`entry`（如 `builtin:tool-runtime`） |
| `commands` | array | 命令：`id`、`labelKey`、可选 `route` |
| `sidecars` | array | sidecar：`id`、`command`、`args`、可选 `workingDirRel` |
| `search` | array | 搜索面板静态条目：`id`、`labelKey`、`keywords`（string[]，可选）、`route` |

未列出的权限字符串在安装时会被拒绝。`entry` 仅允许宿主内置路由别名，不热加载任意 Rust 或远程模块。

## `contributions.search`

声明搜索面板「扩展」分组的静态条目（声明 `search` 必须申请 `uiCommands` 权限）：

```json
"search": [
  {
    "id": "open-example",
    "labelKey": "packs.example.openEntry",
    "keywords": ["example", "demo"],
    "route": "/example"
  }
]
```

- `labelKey` 为 5 locale 均需提供的 i18n key；面板内显示为条目标题，匹配字段为 翻译后标题 + `keywords`。
- `route` 必须可解析（核心壳路由或本 Pack `routes` 贡献的路径），否则条目被过滤。
- 聚合时宿主自动写入 `packId` / `packName`（条目副标题显示 Pack 名），清单内声明无效。
- Pack 禁用后条目随聚合一起消失。
