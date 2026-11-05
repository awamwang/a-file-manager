<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# A File Manager — 架构与功能地图

> 权威副本亦在 Project store：`docs/a-file-manager-feature-map.md`。本文件为仓内同步副本。

基线：`master` · 栈：Tauri 2 + Vue 3 + TypeScript · Fluent（非 Sigma UI kit）。

## 架构一览

`App.vue` → Router → `AppShell`（标题栏 / 侧栏 / Navigator chrome / Toast / 命令面板 / 快捷键）→ Views；经 `invoke` 到 Rust 域：`navigator` · `file_ops` · `work_groups` · `projects` · `discovery` · `providers` · `favorites` · `settings` · `extension_host` 等。

## 路由

| 路由 | 视图 |
|---|---|
| `/navigator` | 目录浏览、Open State 标签、双栏、文件操作 |
| `/work-groups` | Hard members ↔ Open State |
| `/projects` | 看板 / 链接 / 绑定 WG |
| `/discovery` | 全局搜索 |
| `/providers` | 虚拟 FS 后端 |
| `/extensions` | 扩展宿主 |
| `/settings*` | 设置 / 健康 / 快捷键 / 收藏 hook |

## 差异化与缺口

- **强项**：WG Open State 会话、Projects 绑定、Discovery、Fluent shell。
- **基线缺口**：F5 刷新、新建空文件、WG 移除成员/改名/删除、预览窗格、网格视图、压缩、属性。
- **架构债**：AppShell 体量、路径卡片重复、FE/Rust 路径规范化对齐、双栏窄窗。

详见 store 完整版（含对照表与数据流）。
