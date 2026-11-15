<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# FIL-74 右键「在第二窗口打开」：双栏副栏定位交接

> 状态：**已实现 / 待完整验证**。本文是该功能与 FIL-73 取消决定的权威记录。

## 前情：FIL-73 镜像窗口已取消

用户 2026-11-15 先要求「镜像窗口拥有全量独立能力」（独立 Tauri `WebviewWindow`、
工作组 / 标签 / 导航 / 搜索各自一份），随后在讨论「工作组项目打开状态是否也要
每窗口独立」后决定**取消该功能**。原因口径：状态拆分面过大，性价比不足。

- 已落地的骨架（`mirror_window.rs`、`windowScope.ts`、`lib.rs` 注册、
  `capabilities/default.json`、FIL-73 设计文档）整体存进 `git stash@{0}`
  （message：`FIL-73 mirror window skeleton (cancelled)`），**未删除、可恢复**：
  `git stash pop stash@{0}`。
- 取消后工作区回到 HEAD；`.workbuddy/` 仍为未跟踪，按惯例不入库。
- 原 FIL-73 设计文档随骨架一起进 stash，仓库内不再有该主题的活动文档。

## 需求与口径

- 在**标签页右键**与**文件夹右键**都新增「在第二窗口打开」。
- 当前**不是双窗口状态**：打开双窗口，并把该指定目录显示到第二窗口。
- 当前**已是双窗口状态**：只替换第二窗口所展示的路径。

> 关键澄清（2026-11-15 与用户确认）：这里的「第二窗口 / 双窗口状态」指的是
> **现有的双栏副栏**（`dualPaneEnabled` + `secondaryPath`），
> **不是**独立操作系统窗口，也不是应用内浮动面板。
> 副栏只有一份路径与 listing，没有自己的标签页 / 工作组 / 搜索。

## 行为契约

| 场景 | 行为 |
|---|---|
| `dualPaneEnabled === false` | 置 `true` 打开双栏 → `navigateSecondary(path)` 把目标目录载入副栏 |
| `dualPaneEnabled === true` | 只调 `navigateSecondary(path)` 替换副栏路径 |
| 主栏 | 不变；当前路径、标签页、导航历史、Open State 均不改动 |
| 空路径 | 直接返回，不做任何事 |
| 重复点选已开双栏 | **只开不关**，不会误把双栏切回单栏（区别于 `toggleDualPane()`） |

- 开启双栏会触发 `AppShell.vue` 的 `watch(dualPaneEnabled)`，由
  `applyDualPaneWindowMinWidth` 抬高 OS 窗口最小宽度 / 补足当前宽度（AIA-252 既有守卫）。
- `navigateSecondary` 只写副栏 listing / `secondaryPath`，不写主 Open State 与历史栈。
- 菜单项**只对目录可见**：文件、空白区 New-only 菜单、回收站 restore 菜单都不出现。
- 标签右键沿用 `tabContextHasPath` 守卫（标签必须有真实路径），并先
  `ensureNavigatorRoute()` 切回 `/navigator` 再执行。

## 改动清单

| 文件 | 内容 |
|---|---|
| `src/state/dualPane/index.ts` | 新增共享动作 `openInSecondPane(path)`（本功能的唯一行为实现） |
| `src/components/navigator/contextMenuModel.ts` | `ContextMenuActionId` 加 `openInSecondPane`；`secondPaneItems` 顶层项（icon `dualPane`，紧随「打开方式…」）；新增 `canOpenInSecondPane()` |
| `src/components/navigator/NavigatorContextMenu.vue` | 导入共享动作与守卫，`run()` 加 `case "openInSecondPane"` |
| `src/components/shell/AppShell.vue` | 导入共享动作；`tabContextCanDuplicate` 更名 `tabContextHasPath`（复制标签与副栏打开共用）；新增 `onOpenTabInSecondPane()`、菜单按钮；弹出高度 220 → 260 |
| `src/i18n/locales/{en,zh-CN,ja,de,fr}.json` | 两处各补 `chrome.openInSecondPane` / `contextMenu.openInSecondPane` |
| `package.json` | 新增 `test:open-in-second-pane` 并挂入 `test` 链 |
| `scripts/check-open-in-second-pane.mjs`（新） | 全链路静态契约：共享动作、两处菜单 wiring、5 语言文案、测试链 |
| `scripts/open-in-second-pane-checks.ts`（新） | 纯菜单模型运行时断言（目录可见 / 文件与空白区与回收站不可见 / 位置与图标） |
| `scripts/context-menu-model-checks.ts`、`check-context-menu.mjs`、`check-tab-copy-path.mjs`、`check-dual-pane.mjs` | 各自领域补断言 |

文案：中文统一「在第二窗口打开」；en `Open in second pane`；ja `第 2 ペインで開く`；
de `Im zweiten Fenster öffnen`；fr `Ouvrir dans le deuxième volet`。

## 非目标 / 已知边界

- 不新建 Tauri `WebviewWindow`，不改 Rust，不动 capabilities。
- 不改托盘、启动 reveal、关闭确认语义（这些原本就只认 `main`）。
- 副栏路径仍是会话态，不落盘；重启后由既有 Open State 恢复逻辑决定。
- `WorkGroupsView.vue` 里路径卡片的独立右键菜单（`openPathMenu`）**未加**该入口：
  用户明确点名的是「标签的右键」与「文件夹的右键」。若后续要覆盖路径卡片，
  复用 `openInSecondPane(path)` 即可。
- 面包屑 / 列表空白区的文件夹菜单走同一个 `NavigatorContextMenu`，
  因此也自动获得该入口（它们是 `dirEntryFromPath` 构造的目录 entry）。

## 检查

`npm run build` · `npm run test:i18n` · `test:context-menu` · `test:tab-copy-path` ·
`test:dual-pane` · `test:open-in-second-pane` · 完整 `npm test`（含 Rust 测试）。

手工实测：单栏时文件夹右键 → 双栏自动打开且副栏为目标目录；已双栏时再点另一目录
→ 只有副栏换路径、主栏与标签不动；标签右键在 `/work-groups` 等页面也能正确跳回
`/navigator` 并定位副栏；窄窗打开双栏时窗口最小宽度守卫生效。
