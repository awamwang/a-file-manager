<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# 搜索实现盘点（AIA-84）

盘点日期：对照当前 `master` Discovery / Everything / 黑名单实现。本文档**只做能力与样式核对**，不做大范围 UI 改造。

## 1. 产品入口

| 入口 | 行为 |
| --- | --- |
| 侧栏 **Discovery** | `/discovery` 页面；状态卡 + 黑名单 + 搜索框 + 命中列表 |
| 快捷键 **Ctrl+Shift+F** | `discovery.globalSearch` → 跳转 Discovery 并聚焦 `#discovery-global-search` |
| Command Panel | 「打开 Discovery」类命令（导航组） |
| Navigator 本页过滤 | **无**；地址栏是路径跳转，不是全文搜索 |

左侧 sidenav 仍是主导航；搜索页不是 chrome 内嵌条。

## 2. 引擎与索引范围

实现位于 `src-tauri/src/discovery/`（ADR 0002：Everything IPC 一等公民，**无** `es.exe` 产品路径）。

| 状态 | 条件 | UI `activeEngine` / `indexState` |
| --- | --- | --- |
| Everything 可用 | IPC probe 成功 | `everything` / `ready` |
| Fallback stub | Everything 不可用 **且** `fallbackEnabled` | `fallback` / `degraded` |
| 不可用 | Everything 不可用 **且** fallback 关 | `unavailable` / `unavailable`；搜索直接报错 |

要点：

- **默认 fallback 关闭**（`DiscoveryService::new(..., false)`），避免与自建索引双默认。
- Everything 索引范围 = 本机 Everything 服务已索引的路径；AFM **不自建**全盘索引。
- `estimatedItemCount`：Everything 可用时对空查询做 best-effort 总量探测；否则为 `null`（UI 显示「未知」）。
- Fallback 是 stub，不是完整自建索引；开启后仅降级可用。

IPC / 命令：

- `discovery_status`
- `discovery_search`（`offset` / `limit`，默认 limit 100，上限 1000）
- `discovery_set_fallback_enabled`
- `discovery_set_blacklist`（同时写入 `core.discoveryBlacklist`）

## 3. 目录黑名单

- 设置键：`core.discoveryBlacklist`（字符串数组）。
- 启动时 `sync_blacklist_from_settings` 灌入 live service。
- Discovery 页可编辑草稿并保存；命中路径按前缀规则过滤（`blacklist.rs`）。
- Everything 查询会 **over-fetch**（约 3× limit）再过滤，尽量填满一页。

## 4. 命中交互（与 Navigator 对齐，AIA-69）

Discovery 命中行已复用 Navigator 交互面：

- 单击选中；双击目录 → `navigateTo` 进 Navigator。
- 右键 → `NavigatorContextMenu`（收藏等）。
- Shell 图标 / `entryKindIcon` 回退。

**未做**：搜索框内嵌到 Navigator chrome、结果分栏预览、增量输入即搜、分页控件（目前单次 limit）。

## 5. UI / 状态显示核对（讨论点）

当前页面结构（自上而下）：

1. 标题 + meta  
2. **状态卡**：引擎 chip、degraded chip、indexState、估计条目数、detail 原文、fallback 开关  
3. **黑名单卡**：多行文本 + 保存  
4. 搜索表单  
5. 横幅（成功 / 错误 / 无命中）  
6. 命中列表

观察到的样式 / 信息问题（供讨论，本轮不改大 UI）：

| 问题 | 说明 |
| --- | --- |
| 引擎 chip 显示原始枚举 | `everything` / `fallback` / `unavailable` 英文小写，未走 i18n 展示名 |
| `status.detail` 偏工程化 | 直接展示 Rust 探测字符串，终端用户可读性一般 |
| 状态卡 + 黑名单卡偏「运维面板」 | 与 Fluent 内容页「一职一节」相比信息密度高；若产品定位是用户搜索，可下沉高级项到 Settings |
| Settings 已有 blacklist 字段 | Discovery 页与 Settings 双入口，易重复；需约定「权威编辑面」 |
| 无分页 / 加载更多 | `totalCount` 可能远大于 shown；仅 meta 文案提示 |
| Fallback stub 语义弱 | 勾选后用户可能以为「已启用完整本地索引」 |
| Navigator 无搜索入口 | 仅靠快捷键 / 侧栏；新用户可发现性一般 |

建议讨论顺序：① 用户搜索 vs 运维面板信息架构 → ② chip/detail 文案 → ③ Settings 与 Discovery 黑名单单一入口 → ④ 是否在 Navigator chrome 加轻量跳转（非本轮）。

## 6. 相关检查脚本

- `npm run test:discovery-search` — 索引状态 / 黑名单契约  
- `npm run test:discovery-unify` — 命中与 Navigator 交互对齐  
- `npm run test:global-search` — Ctrl+Shift+F 焦点链路  
- `npm run test:search-status-doc` — 本盘点文档存在性（AIA-84）

## 7. 明确不做（本轮）

- 不重做搜索页大布局 / 设计系统换肤  
- 不引入第二搜索引擎产品路径  
- 不做多窗口搜索会话（见 AIA-83，待定）
