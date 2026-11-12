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
| `gatewayCompat` | object | 否 | `minVersion` / `maxVersion`（semver，本阶段仅存储） |

## `contributions`

| 字段 | 类型 | 说明 |
|------|------|------|
| `sideNav` | array | 侧栏项：`id`、`labelKey`、`to`、`order` |
| `routes` | array | 路由：`path`、`name`、`entry`（如 `builtin:tool-runtime`） |
| `commands` | array | 命令：`id`、`labelKey`、可选 `route` |
| `sidecars` | array | sidecar：`id`、`command`、`args`、可选 `workingDirRel` |

未列出的权限字符串在安装时会被拒绝。`entry` 仅允许宿主内置路由别名，不热加载任意 Rust 或远程模块。
