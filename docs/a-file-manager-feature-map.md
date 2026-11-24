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
| `/projects` | 工作区链接 / 绑定 WG / `tasks.json`（D5 已移除任务看板 / 列表 UI，board 数据不迁移） |
| `/discovery` | 全局搜索 |
| `/providers` | 虚拟 FS 后端 |
| `/extensions` | 扩展宿主 |
| `/tools` | 工具管理运行时（Feature Pack `awam.tool-runtime`，启用 Pack 后注入侧栏） |
| `/settings*` | 设置 / 健康 / 快捷键 / 收藏 hook |

## 差异化与缺口

- **强项**：WG Open State 会话、Projects 绑定、Discovery、Fluent shell、三层回归基线门禁（ADR 0025：STATIC-GATE + FE-UNIT + CARGO，150 门禁全接入 `npm test`）。
- **资源管理器基本盘已齐**（2026-11-24 核对）：F5 刷新（`navigator.refresh`）、新建文件/文件夹（`new_templates.rs`）、WG 改名/删除/移除成员（`work_groups_*`）、预览窗格（`PreviewPane.vue` + preview bridge）、视图模式（`navigatorViewMode`）、ZIP 压缩/解压（`archive/`）、属性（`navigator/stat.rs` + `test:properties-attrs`）。
- **已拍板未实现的场景项**：以 `file-ops-scenario-baseline.md` 的 🔵/🟡 行为准（如 TR-203 右键「复制为链接」、CLIP-008 剪切源消失跳过、DEL-007 还原重建父目录、SEL-010 Esc 第 2/3 层）。
- **架构债**：AppShell 体量、路径卡片重复、FE/Rust 路径规范化对齐、双栏窄窗。

覆盖事实以 `external-behavior-baseline-coverage.md`（coverage ledger）为准，场景以 `file-ops-scenario-baseline.md` 为准。
