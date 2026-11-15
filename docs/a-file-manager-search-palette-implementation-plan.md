<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# 统一搜索面板实施计划（Ctrl+Shift+P）

状态：设计与分阶段规划已确认，尚未开始实现。本文是实现入口；术语见 [搜索面板术语](./glossary-search-palette.md)，整体决策见 [ADR 0015](./adr/0015-unified-search-palette.md)。

## 1. 目标

把 Ctrl+Shift+P 从“只列命令”的命令面板升级为统一搜索面板：一次输入即可检索命令、文件、项目、工作组、工具/能力与 Pack 扩展条目；结果分类展示、每行显式标注类型、范围可组合并持久化。面板只负责“找到并触发下一跳”，不承担项目激活、布局修复等状态变更。

## 2. 不变量

- 内部文件名 `CommandPanel.vue`、`state/commandPanel.ts` 与快捷键 id `commandPanel.toggle` 保持不变；只改用户可见文案为“搜索面板”。
- `Ctrl+Shift+F` 与 Discovery 页面保留为完整文件结果入口。
- 破坏性工具能力不在面板内执行，必须进入工具运行时人工确认。
- 现有 `test:command-panel`、`test:shortcuts`、`test:fluent-p3-icons`、`test:dead-code` 必须继续通过。

## 3. 架构

```
AppShell (panelCommands / commandPanelOpen)
   └─ CommandPanel.vue
        └─ useSearchPalette()
             ├─ sources/commands.ts
             ├─ sources/files.ts
             ├─ sources/projects.ts
             ├─ sources/workgroups.ts
             ├─ sources/tools.ts
             └─ sources/extensions.ts
        ├─ scope.ts                范围组合 + localStorage
        ├─ searchPaletteConfig.ts  → Rust palette_get_config
        └─ toolRuntimeIndex.ts     能力预拉取 / 5 分钟缓存
```

结果行：行首图标 + 标题 + 副标题 + 行尾类型文字 chip。类型为：命令 / 文件 / 文件夹 / 项目 / 工作组 / 工具 / 能力 / 扩展。

固定分组顺序：命令 → 文件 → 项目 → 工作组 → 工具 → 扩展；每组上限 8 条。

## 4. 搜索源契约

| 源 | 数据 | 匹配字段 | Enter | 不可用表现 |
|---|---|---|---|---|
| 命令 | `buildAppCommands` | 标题 + 关键词（group 不参与） | 执行 | — |
| 文件 | `discovery_search`，limit 8 | Everything 自身规则，面板不二次过滤 | 遵循 `core.fileDoubleClickAction`；Ctrl+Enter 资源管理器定位 | 状态行 + 打开 Discovery |
| 项目 | `projects_workspace_list` | `displayName`（仅名称） | `/projects?slug=<slug>` 仅聚焦，不激活 | 状态行 + 打开项目页 |
| 工作组 | `hydrateWorkGroups` | 名称（仅名称） | 静默切换；`REQUIRE_SAVE_PROMPT` 时关面板 + toast | 状态行 |
| 工具 | `tool_runtime_list_tools` + `list_capabilities` | 工具名 / 能力名（描述不参与） | 跳工具运行时并预选；可按配置改为 `callCapability` | 状态行 + 打开工具运行时 |
| 扩展 | Pack `contributions.search` | 标题 + 关键词 | 跳声明路由 | 无条目时隐藏 chip |

文件结果打开必须走本地路径专用动作，不能复用当前路径含 Provider 时的远端分支。

## 5. 范围组合

- `localStorage` 键：`afm.searchPaletteScopes`，值为搜索源 id 数组。
- 空集、含未知 id、或等于全集时按「全部」处理；「全部」= 全选，不是独立模式。
- chip 点击或面板内 `Alt+1…6` 切换该源；关掉最后一个源回落「全部」。
- 面板打开时在遮罩层 `keydown` 拦截并 `stopPropagation`，优先于全局 Alt+数字（工作组切换）。
- 持久化的组合包含暂不可用源时保留选择，显示状态行，不自动回退。

## 6. 工具 Enter 配置

文件：`<appConfigDir>/command-palette.json`（Core 持有，非 Pack 私有配置）。

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

- 缺省 `navigate`：跳工具运行时并预选工具/能力。
- `callCapability`：固定 `execute: true`、`allowDestructive: false`；破坏性能力只提示并引导进工具运行时。
- 文件缺失、JSON 损坏或动作非法时回落 `navigate`，不阻断面板。
- v1 无界面，能力级配置字段仅预留、不实现。

## 7. 分阶段任务

### Phase 1 — 面板骨架 + 本地源（命令 / 项目 / 工作组）

- 新增 `src/search/`：`types.ts`、`match.ts`、`scope.ts`、`useSearchPalette.ts`、`registry.ts`、`sources/{commands,projects,workgroups}.ts`。
- 重写 `src/components/command/CommandPanel.vue`：范围 chips、分组、类型 chip、副标题、加载/离线/空状态、`Alt+数字`、`Ctrl+Enter`。
- 项目/工作组沿用现有加载器；不新增数据源。
- 5 个 locale 文件新增 `searchPalette.*`。
- 验收：命令/项目/工作组可搜；组合范围关闭重开保持；`Alt+数字` 只切范围；类型标签完整。

### Phase 2 — 文件源

- 新增 `src/search/sources/files.ts`：180ms 防抖、序列号丢弃过期结果、`discovery_search` limit 8、行图标 `requestShellIcons`。
- 在 `src/state/navigatorBrowse.ts` 抽出 `openLocalEntry()`，跳过 Provider 分支；`openEntry` 复用它保持现状。
- 目录 Enter：`openOrFocusNavigatorTab` + 跳 `/navigator`；文件 Enter：本地文件动作；Ctrl+Enter：`revealItemInDir`。
- `src/views/DiscoveryView.vue` 支持 `/discovery?q=...` 自动填入并搜索；“查看全部文件结果”带查询跳转。
- 验收：输入即出 Top 8；打开/定位正确；离线出状态行；查看全部带查询。

### Phase 3 — 工具 / 能力源 + Enter 配置

- 新增 `src/state/toolRuntimeIndex.ts`：先探活，再 `list_tools`，并发 4 拉取能力；5 分钟 TTL、最多 50 个工具、单工具失败忽略。
- 新增 `src/search/sources/tools.ts`：工具与能力同组平铺，能力副标题显示所属工具。
- 工具运行时路由从 Pack 贡献读取（`builtin:tool-runtime`）；Pack 未启用时隐藏工具源。
- `src/packs/tool-runtime/ToolRuntimeView.vue` 读取 `?tool=...&capability=...` 并预选，应用后清 query。
- 新增 Rust `src-tauri/src/palette/mod.rs`：读取 `command-palette.json`，`palette_get_config` 注册进 `src-tauri/src/lib.rs`；文件缺失/损坏回落空配置。
- 新增 `docs/command-palette-config.md` 记录 schema 与示例。
- 验收：工具/能力可搜；Enter 预选正确；配置覆盖生效；网关离线只显示状态行。

### Phase 4 — Pack 静态条目

- Rust：`FeaturePackContributions` / `AggregatedContributions` 增加 `search: Vec<SearchEntryContribution>`（`id`、`labelKey`、`keywords`、`route`，聚合时宿主写入 `packId` / `packName`）；声明 `search` 必须申请 `uiCommands`；只收可解析路由。
- 前端：`src/state/featurePack/contributions.ts` 新增 `getSearchExtras()`；`src/search/sources/extensions.ts` 映射为「扩展」类型，副标题显示 Pack 名。
- 更新 `docs/feature-pack-format.md` 与 `packs/_template/pack.json`。
- 验收：启用 Pack 条目出现在「扩展」；禁用后消失；非法/未知路由被过滤。

### Phase 5 — 测试、文档与收尾

- 新增 `scripts/search-palette-checks.ts` + `scripts/check-search-palette.mjs`，加入 `package.json` 的 `test:search-palette` 与 `test` 聚合。覆盖：范围归一化/持久化/切换；仅名称匹配；分组顺序与每组上限；类型标签；异步源过期结果丢弃；工具动作配置解析与非法回落；Pack 条目过滤。
- 更新 `scripts/check-global-search.mjs`（Discovery `q` 参数）。
- 运行：`npm run test:search-palette`、`npm run test:i18n`、`npm run test:global-search`、`npm run test:rust`、`npm run build`。
- 真机验证（`npm run tauri:dev`）：六类搜索、组合范围重开保持、Alt+数字不切工作组、Ctrl+Enter 定位、网关离线、Pack 条目。
- 新增 `docs/a-file-manager-search-palette-handoff.md`。

## 8. 风险与处理

1. `openEntry` 的 Provider 分支：当前路径是 Provider 时会把本地文件误当远端打开 → 新增 `openLocalEntry`，不改 `openEntry` 语义。
2. Alt+数字冲突：不修改快捷键注册表；面板内拦截并 `stopPropagation`，保留工作组快捷键在其他场景的行为。
3. 能力预拉取拖慢网关：探活优先、TTL 5 分钟、并发 4、单工具失败忽略、上限 50；离线不发请求。
4. 持久化范围包含不可用源：保留选择，显示状态行，不自动回退。
5. Discovery 双入口：`q` 参数优先于焦点 token；`Ctrl+Shift+F` 行为不变。
6. 工具运行时路由不存在：源与 chip 一起隐藏，不显示死入口。

## 9. 交付方式

- 从 `master` 切功能分支，按 Phase 分次提交。
- 每次提交后列出全部改动路径并汇报进度。
- 不推送远端、不开 PR，除非明确要求。

## 10. 开始位置

从 **Phase 1** 开始，第一个落点是 `src/search/types.ts` 与 `src/search/scope.ts`，随后改造 `src/components/command/CommandPanel.vue`。