# Third-Party Notices

A File Manager (AFM) 以 `GPL-3.0-or-later` 授权（见 `LICENSE`）。本文件记录 AFM 作为**集成方**（ADR 0022）为能力后端自构建 / 随发行提供的第三方 / 自有上游组件的许可证与义务。

## 集成（managed）能力后端源码

以下三个后端作为 AFM 的 Feature Pack 后端产物分发（`runtime: self` 的 PyInstaller onefile 等），各自为独立可执行进程、**不与 AFM 静态链接**；其上游源码均带本 notice，AFM 发行中保留对应版权声明并提供对应源码。三者的 `MIT License` 与 AFM 的 `GPL-3.0-or-later` **兼容**（MIT 可被 GPL 接受并合并）。

### awam-tool-manager

- 许可证：MIT
- 版权：Copyright (c) 2026 awamwang
- 用途：工具管理运行时后端（Feature Pack `awam.tool-runtime`，监听 `127.0.0.1:18778`）
- 对应源码：AFM 自有上游仓 `awam-tool-manager`（MIT 全文见该仓 `LICENSE`）

MIT License 原文要件（保留于上游仓 `LICENSE`）：

> Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software...

### awam-file（awam-resource-gateway）

- 许可证：MIT
- 版权：Copyright (c) 2026 awamwang
- 用途：统一 `awam://` 资源网关后端（MCP + localhost HTTP，默认端口 `18787`）
- 对应源码：AFM 自有上游仓 `awam-file`（MIT 全文见该仓 `LICENSE`）

### awam-knowledge

- 许可证：MIT
- 版权：Copyright (c) 2026 awamwang
- 用途：本地 Markdown 知识库召回 / 导入 / 评测后端（零依赖 Node，`engines.node >= 18`）
- 对应源码：AFM 自有上游仓 `awam-knowledge`（MIT 全文见该仓 `LICENSE`）

## AFM 构建/分发使用的工具链（非随发行二进制）

- **PyInstaller**：构建期把上述 Python 后端冻结为 onefile 产物；其产物为独立后端进程，PyInstaller 本体不随 AFM 桌面应用分发。
- **Node.js**：`awam-knowledge` 后端运行时；由系统 / 共享运行时提供，非 AFM 二进制的一部分。
- 构建脚本 `scripts/build_pack_feed.py` 由 AFM 维护（GPL-3.0-or-later）。

## 义务摘要（GPL-3.0-or-later 组合作品）

1. 上述 MIT 组件的版权声明与本许可要件在 AFM 发行中**予以保留**。
2. 对 AFM 分发的、含这些组件的产物，**提供对应源码**——由 AFM 自有上游仓源码满足。
3. 任一 MIT 组件不施加超出 GPL 的额外限制（MIT 允许）。

如新增 / 变更集成组件，请同步更新本文件并在对应 Feature Pack 产物内保留其许可声明。
