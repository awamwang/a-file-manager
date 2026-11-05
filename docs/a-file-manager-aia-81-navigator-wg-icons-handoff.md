<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# AIA-81 / 82 / 84 交接

## 范围

| 键 | 内容 | 状态 |
| --- | --- | --- |
| **AIA-81** | Navigator chrome Work Group 下拉：silent switch + Save Open State / Save members from tabs（对齐 Sigma store 语义，Fluent UI） | 已做 |
| **AIA-82** | Home（主目录）/ 星标 Places（QA+收藏）/ 虚线双栏图标拆分 | 已做 |
| **AIA-84** | `docs/a-file-manager-search-status.md` 搜索盘点（无大 UI 改动） | 已做 |
| **AIA-83** | 多窗口 | **待定，未做** |

## 实现要点

### AIA-81

- 新模块 `src/state/workGroupChrome.ts`：`work_groups_get` / `silent_switch` / `save_open_state` / `save_members_from_open_state`。
- `AppShell` Navigator 工具栏：`#navigator-work-group-menu` 显示当前 WG 名；切换走 silent switch + `restoreWorkGroupOpenState`；soft sync 横幅可 dismiss。
- 「管理工作组…」跳转既有 `/work-groups` 页（完整 CRUD 仍在该页）。
- Sigma rewrite 侧仅有 store/sync，无 Navigator WG Vue chrome；本轮按 AFM IPC + Sigma 语义落地 Fluent 下拉。

### AIA-82

- **Home** `#navigator-home-btn`：优先 `core.defaultLocation`，否则 `navigator_default_path`（USERPROFILE）。
- **Places** 触发图标改为 `star`（仍为快速访问 + 收藏下拉）。
- **Dual Pane** 图标改为 `dualPane`（双栏 + 虚线分隔）；`view` 四宫格不再充当分栏。
- 侧栏仍是主导航主体；上述控件仅 Navigator chrome。

### AIA-84

- 文档：`docs/a-file-manager-search-status.md`（引擎/黑名单/入口/UI 讨论点）。

## 检查

```text
npm run test:navigator-wg-chrome
npm run test:navigator-icon-split
npm run test:search-status-doc
npm run test:navigator-chrome-layout
npm run test:dual-pane
npm run test:i18n
npm run test:fluent-icons
```

## 未做 / 后续

- AIA-83 多窗口：待定。
- 搜索页大改版、chip i18n、黑名单单一入口：见盘点文档讨论节。
- 未 push / 未开 PR（按本批约定本地 master 提交）。
