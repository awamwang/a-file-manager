<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# AIA-98 应用图标候选

协调端已生成候选图到 Project store `media/`：

- `afm-icon-candidate-a-folder-blue.jpg` — Windows 资源管理器式蓝文件夹
- `afm-icon-candidate-b-dual-pane.jpg` — Fluent / 双栏意象
- （若有）`afm-icon-candidate-c*` — 第三风格

选定后：将 PNG 源放入仓库，执行 `npx tauri icon <source.png>` 覆盖 `src-tauri/icons/`（当前仍为脚手架默认图标）。
