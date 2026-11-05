<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# AIA-90 OpenState 去掉显式保存

## 产品

分组 / 工作组 UI **不提供** OpenState「保存」按钮。切换时由 `work_groups_silent_switch` 静默落盘离开组；运行时仍由 `navigatorBrowse` 的 persist 写入；回到分组时静默加载。

## 改动

- `AppShell` WG 下拉：移除 `workGroups.saveOpenState` 项
- `WorkGroupsView`：移除 Save Open State 按钮
- `workGroupChrome.ts`：移除 `saveActiveOpenStateHard` 封装（成员「从标签保存」仍保留）

## 检查

`npm run test:navigator-wg-chrome` · `npm run test:open-state`
