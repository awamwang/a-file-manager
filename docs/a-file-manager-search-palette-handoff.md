# 统一搜索面板（Search Palette）交接文档

对应实施计划：`docs/a-file-manager-search-palette-implementation-plan.md`。
本文档面向后续维护者，汇总已交付内容、架构地图、行为契约与验证清单。

## 1. 交付概览

| 阶段 | 内容 | 提交 |
| --- | --- | --- |
| Phase 1 | 契约与编排（types/scope/match/useSearchPalette）、命令源接入、范围 chip、i18n 骨架 | `aa950d3` |
| Phase 2 | 文件源（discovery_search limit 8 + 查看全部深链 `?q=`）、项目源、工作组源 | `186f176` |
| Phase 3 | 工具/能力源（toolRuntimeIndex 预取 + callCapability 配置）、Core 配置 `command-palette.json`、工具运行时预选深链 | `119d794` |
| Phase 4 | 扩展源：Pack `contributions.search` 静态条目（uiCommands 权限门控、路由过滤、packId/packName 聚合） | `7b69b34` |
| Phase 5 | 测试门禁（`test:search-palette`）、check-global-search 的 Discovery `q` 参数断言、本文档 | 见 git log |

## 2. 架构地图

```
src/search/
  types.ts                 契约：SearchSourceId / SEARCH_SOURCE_ORDER（chip 顺序）/ SEARCH_RESULT_GROUP_ORDER（结果分组顺序，文件殿后）/ SEARCH_GROUP_LIMIT=8
                           / SearchResultItem / SearchSource（enabled、unavailableKey、viewAll）
  scope.ts                 范围组合：localStorage afm.searchPaletteScopes + afm.searchPaletteFuzzy（模糊勾选），归一化/持久化/切换
  match.ts                 仅名称匹配（项目/工作组/工具）：fuzzyScore 子序列，描述不参与
  useSearchPalette.ts      编排：范围交集、跨源检索（seq 过期丢弃）、固定分组 + 组内上限 8
  registry.ts              buildSearchSources(deps)：六源注册（命令→文件→项目→工作组→工具→扩展）
  searchPaletteConfig.ts   Core 配置 <appConfigDir>/command-palette.json 加载/归一化（palette_get_config）
  sources/commands.ts      命令源：空 query 返回全量命令
  sources/files.ts         文件源：discovery_search limit 8；组尾「查看全部」→ /discovery?q=…
  sources/projects.ts      项目源：openLocalEntry 定位；副标题为项目路径
  sources/workgroups.ts    工作组源：REQUIRE_SAVE_PROMPT → toast 提示后跳转
  sources/tools.ts         工具/能力源：toolRuntimeIndex 平铺；Enter 可按配置 callCapability
  sources/extensions.ts    扩展源：Pack 静态条目，标题 = t(labelKey)，副标题 = Pack 名
src/state/toolRuntimeIndex.ts   网关探活 + listTools 预取（TTL 5 分钟、并发 4、上限 50、离线返回 null）
src/state/featurePack/contributions.ts
                           toolRuntimeRouteContribution / searchExtras 响应式；
                           getSearchExtras = toFeaturePackSearchEntries(过滤非法路由)
src/components/command/CommandPanel.vue   面板 UI：chip、分组、行尾类型标签、Alt+数字、Ctrl+Enter
src-tauri/src/palette/mod.rs             PaletteConfig + palette_get_config 命令
src-tauri/src/extension_host/            Pack search 贡献聚合 + uiCommands 权限校验
```

## 3. 行为契约（改动前先读）

1. **顺序解耦**：范围 chip / Alt+数字按 `SEARCH_SOURCE_ORDER`（命令→文件→项目→工作组→工具→扩展，
   文件=Alt+2）；结果分组按 `SEARCH_RESULT_GROUP_ORDER`（命令→项目→工作组→工具→扩展→文件，
   **文件组排最后**）；每组上限 8 条（`SEARCH_GROUP_LIMIT`）。
2. **范围组合**：`afm.searchPaletteScopes` 持久化；空/损坏/含未知 id 一律按「全部」（全选）处理；
   含暂不可用源时保留选择并显示状态行，不自动回退；关掉最后一个源回落「全部」。
3. **过期结果丢弃**：查询变化即递增 `searchSeq`，慢源返回的旧结果不落状态（`useSearchPalette.ts`）。
4. **仅名称匹配**：项目/工作组/工具源只匹配名称（`match.ts`），描述与路径不参与。
5. **文件源查看全部**：组尾伪行跳 `/discovery?q=<query>`；Discovery 一次性消费后 `router.replace` 清除。
6. **工具源 Enter**：默认跳工具运行时并预选（`?tool=…&capability=…`）；
   `command-palette.json` 可覆盖为 `callCapability`（固定 `execute: true`、`allowDestructive: false`）；
   破坏性能力只在面板内提示并引导进运行时，绝不直接执行。见 `docs/command-palette-config.md`。
7. **扩展源**：Pack 声明 `contributions.search` 必须申请 `uiCommands` 权限（Rust 侧校验）；
   缺 id/labelKey/route 或路由不可解析的条目被过滤；禁用 Pack 后条目与 chip 一起消失。
8. **网关离线**：工具源显示状态行 + 「打开工具运行时」引导；tool-runtime Pack 未启用时源与 chip 一起隐藏。
9. **模糊搜索勾选**：面板底部勾选框（默认不勾选），状态经 localStorage `afm.searchPaletteFuzzy` 持久化。勾选后
   tools/extensions 改子序列匹配（`matchText`），文件源发 Everything `*a*b*c*` 通配符模式
   （limit 32）并按名称 `fuzzyScore` 重排取前 8；commands/projects/workgroups 本就是模糊匹配，不受影响。
10. **命名不变量**：内部文件名 `CommandPanel.vue` / `state/commandPanel.ts`、快捷键 id
   `commandPanel.toggle` 不变；仅用户可见文案为「搜索面板」。

## 4. 测试门禁

- `npm run test:search-palette`：范围归一化/持久化/切换、仅名称匹配、分组顺序与每组上限、
  类型标签齐全、异步源过期丢弃、工具动作配置解析与非法回落、Pack 条目过滤。
- `npm run test:global-search`：Ctrl+Shift+F 焦点链 + Discovery `q` 参数深链。
- 相关回归：`test:command-panel`、`test:i18n`（1077 keys × 5 locale）、`test:feature-pack-shell`、
  `npm run test:rust`（含 palette 与 extension_host search 测试）。

## 5. 真机验证清单（`npm run tauri:dev`）

- [ ] Ctrl+Shift+P 打开「搜索面板」；六类搜索各自出结果且分组顺序正确。
- [ ] 组合范围（如仅 命令+工具）关闭重开面板后保持；Alt+数字切换范围；
      Alt+数字不再触发工作组切换（面板内拦截）。
- [ ] 文件命中 Enter 打开；Ctrl+Enter 在资源管理器中定位。
- [ ] 工具网关离线时工具组显示状态行；恢复后重开面板可检索。
- [ ] 工具运行时 Pack 禁用时工具组与 chip 消失；启用 Pack 的 search 条目出现在「扩展」组，
      禁用后消失。
- [ ] `command-palette.json` 配置 callCapability 后 Enter 直接执行非破坏性能力并出 info toast。

## 6. 已知边界

- 工具索引预取有 5 分钟 TTL 与 50 工具上限；网关不可达不缓存，下次打开面板重试。
- Discovery `q` 深链是一次性的：消费后清除，刷新/分享不残留旧查询。
- `searchPalette.status.extensionsUnavailable` 为兜底文案：当前实现里扩展源无条目即隐藏，
  不会真正进入 unavailable 状态。
