# 搜索面板工具 Enter 配置（`command-palette.json`）

统一搜索面板（Ctrl+Shift+P）中「工具」分组的主动作默认是**跳转工具运行时并预选**。用户可在 Core 配置目录下的 `command-palette.json` 中按工具 id 覆盖该动作，让 Enter 直接调用一个非破坏性能力。

- 文件位置：`<appConfigDir>/command-palette.json`（Core 持有，非 Pack 私有配置；Windows 默认 `%APPDATA%\<bundleId>\`）。
- 文件缺失、JSON 损坏或动作非法时回落 `navigate`，不阻断面板。
- v1 无界面编辑；能力级配置字段仅预留、不实现。

## Schema

```json
{
  "schemaVersion": 1,
  "toolActions": {
    "some-tool": { "enter": "navigate" },
    "another-tool": {
      "enter": "callCapability",
      "capabilityId": "run",
      "arguments": { "flag": true }
    }
  }
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `schemaVersion` | `number` | 当前为 `1`。 |
| `toolActions` | `object` | key 为工具 id（`tool_runtime_list_tools` 返回的 `id`），value 为该工具的 Enter 动作。 |
| `toolActions[*].enter` | `"navigate" \| "callCapability"` | 缺省 `navigate`；非法值按 `navigate` 处理。 |
| `toolActions[*].capabilityId` | `string` | `callCapability` 时要调用的能力 id（`tool_runtime_list_capabilities` 返回的 `capabilityId`）。 |
| `toolActions[*].arguments` | `object` | 调用参数（会作为能力 `inputSchema` 的输入），缺省 `{}`。 |

## 行为

- `navigate`（缺省）：跳转 `/tools` 并通过 `?tool=<id>` 预选工具（能力行会附加 `&capability=<id>`）。
- `callCapability`：面板直接调用该能力，固定 `execute: true`、`allowDestructive: false`；破坏性能力只在面板内提示，并引导进入工具运行时人工确认，**不会**在面板内执行。
- `callCapability` 缺少 `capabilityId`、能力不存在或调用失败时，回落跳转工具运行时并预选。

## 示例：让 Enter 直接运行某工具的 `run` 能力

```json
{
  "schemaVersion": 1,
  "toolActions": {
    "quick-launcher": {
      "enter": "callCapability",
      "capabilityId": "run",
      "arguments": { "verbose": false }
    }
  }
}
```
