<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Microsoft Store 一览：素材与文案（定稿）

本目录是 **A File Manager** 在 Microsoft Store 上架时使用的商店素材与文案**最终定稿**。
文案以官网 <https://software.awam.wang/a-file-manager/> 为准（特性优先，基础文件管理一笔带过），
已于 2026-11-28 随 Submission 1 提交认证。

| 项 | 值 |
|---|---|
| Store ID | `9PFJSXVRKQFG` |
| Store URL | <https://apps.microsoft.com/detail/9PFJSXVRKQFG> |
| Submission 1 ID | `1152921505702527105` |
| 提交状态 | 正在认证（通过后立即发布） |
| 包 | `a-file-manager-0.4.0.0-x64.msix` |
| 账号类型 | 个人开发者（Publisher = `Awam M Wang`） |

> 界面改版后：按下方顺序重新截图覆盖 `screenshots/`，并同步复核文案里提到的功能是否仍成立。

---

## 一、素材

### 1. 截图（Store 一览按此顺序上传）

| 顺序 | 文件 | 尺寸 | 画面 |
|---|---|---|---|
| 1 | `screenshots/01-explorer.png` | 1920×1080 | 资源管理器（侧栏树 + 多标签 + 双栏 + 预览） |
| 2 | `screenshots/02-work-groups.png` | 1920×1080 | 工作组 |
| 3 | `screenshots/03-projects.png` | 1920×1080 | 项目 |
| 4 | `screenshots/04-ai.png` | 1920×1080 | AI（技能 / MCP） |

- 来源：`docs/screenshots/{explorer,work-groups,projects,ai}.png`（3377×1440）
- 处理：居中裁成 16:9 → 缩放到 1920×1080 PNG（Store 要求 16:9、建议 ≥1366×768）
- 中英文两个 Store 一览使用**同一组**截图与同一顺序
- `extensions.png`（扩展 / Feature Pack）**未采用**——与官网主张（工作组 / 项目 / 为 AI 而生）不贴合

### 2. 图标

- `store-icon-300.png`：300×300 透明底，取自 `src-tauri/icons/icon.png`

---

## 二、中文（中国）一览

### 说明（718 字符）

```
A File Manager 是一款为 AI 智能体打造的现代 Windows 桌面文件管理器。它把「你正在做的一件事」当成一等公民，不再是一堆散落的窗口，而是随时可以切回的工作现场。

工作组：把同一件事会用到的文件夹聚在一起
工作组是一组相关文件夹位置的集合。它不移动、也不复制磁盘上的任何文件，只是把这些目录收拢到一处：成员可拖动调整顺序，打开工作组即恢复它记住的标签页，删除工作组不会动磁盘文件，未分类的位置由默认工作组兜底。常用的一批位置一键切换，随时回到工作现场。

项目：从工作组一步长出面向 AI 的结构
项目从工作组创建（源工作组保留），会建立独立的工作目录，并搭好 AI 开发所需的基础结构：CONTEXT.md、tasks.json、.skills/ 与 mcp.json。项目成员只是指向文件夹的链接，不动原目录；删除项目同样不移除磁盘文件。

为 AI 而生
在 IDE 或终端中一键打开当前文件夹，打开方式可自定义；目录可登记为 Skill 供智能体调用；项目自带 Skill / MCP 配置，IDE 与 Agent 进入即感知。命令行同样可用：afm open --with vscode、afm skill add ./control-plane。

基础的文件管理
双栏对照与多标签页、列表/详情/图标视图、全局搜索、系统剪贴板互通、拖放与重命名；删除进回收站可还原，所有文件操作支持 Ctrl+Z 撤销。

界面采用 Fluent 设计，深/浅色主题，内置简中、繁中、英、日、德、法。提供主线版（含 AI 技能与智能体集成）与精简版 Lite（纯文件管理）。开源，GPL-3.0-or-later。
```

### 产品功能（4 条，每条 ≤200 字符）

1. 基础资源管理器功能，对齐 Windows 原生：双栏对照、多标签、列表/详情视图、全局搜索、拖放与系统剪贴板互通
2. 工作组：把一组相关文件夹存成工作现场，一键切换并恢复对应标签页，不移动磁盘文件
3. 项目：从工作组一步创建独立工作目录，自带 CONTEXT、tasks.json、Skill 与 MCP 配置
4. 为 AI 而生：目录可登记为 Skill，在 IDE / 终端一键打开，Agent 进入即感知项目上下文

### 简短描述（64 字符）

```
为 AI 智能体打造的现代 Windows 文件管理器：工作组、项目、为 AI 而生，把散落的文件夹收拢成随时可切换的工作现场。
```

---

## 三、英语（美国）一览

### Description（1934 字符）

```
A File Manager is a modern Windows file manager built for AI agents. It treats "the thing you are working on" as a first-class citizen — not a pile of scattered windows, but a work scene you can jump back into at any time.

Work Groups: gather the folders one task needs
A Work Group is a set of related folder locations. It does not move or copy anything on disk — it simply brings those directories together in one place: members can be reordered by drag and drop, opening a group restores the tabs it remembers, and deleting a group never touches your files. Uncategorized locations are covered by a default group, so a batch of frequently used places is always one click away.

Projects: grow an AI-ready structure out of a Work Group
A Project is created from a Work Group (the source group is kept). It creates an independent working directory and scaffolds what AI development needs: CONTEXT.md, tasks.json, .skills/ and mcp.json. Project members are links to folders — the original directories stay untouched, and deleting a project does not remove files either.

Built for AI
Open the current folder in your IDE or terminal with one click; openers are customizable. Register a directory as a Skill for agents to call. Projects ship with their own Skill / MCP configuration, so an IDE or agent understands the context the moment it enters. The command line works too: afm open --with vscode, afm skill add ./control-plane.

Basic file management
Dual-pane and multi-tab browsing, list/details/icon views, global search, system clipboard interop, drag and drop and rename; deletions go to the Recycle Bin and can be restored, and file operations support Ctrl+Z.

Fluent design with dark and light themes, built-in Simplified Chinese, Traditional Chinese, English, Japanese, German and French. Available as the mainline build (with AI skills and agent integration) and Lite (pure file management). Open source, GPL-3.0-or-later.
```

### Features（4 条，each ≤200 chars）

1. Baseline file management on par with Windows Explorer: dual panes, tabs, list and details views, global search, drag and drop, system clipboard interop
2. Work Groups: save a set of related folders as a work scene, switch in one click and restore its tabs, without moving anything on disk
3. Projects: create an independent working directory in one step, with CONTEXT, tasks.json, Skill and MCP configuration included
4. Built for AI: register a folder as a Skill, open it in your IDE or terminal in one click, and agents pick up project context immediately

### Short description（139 字符）

```
A modern Windows file manager built for AI agents: Work Groups, Projects, and a workspace that always comes back exactly where you left it.
```

---

## 四、提交选项

### 1. 受限功能 `runFullTrust` 理由（284/500 字符）

```
本应用是基于 Tauri（Rust + WebView2）构建的 Windows 桌面文件管理器，以 MSIX 分发，需要 runFullTrust 才能启动本地 WebView2 运行时与原生可执行文件。用途：(1) 浏览和管理用户本机任意位置的文件夹，读写、复制、移动、重命名并将删除送入回收站，UWP 容器无法覆盖任意路径、外部卷与网络位置；(2) 保存用户指定的“工作组/项目”目录配置，使用系统剪贴板与拖放；(3) 按用户选择调用本机 IDE 或终端打开当前文件夹。文件操作均在本机完成，不上传内容；仅用户主动启用可选扩展（rclone、AI 技能）时联网。
```

### 2. 认证说明（其他测试信息，1513 字符）

```
Test notes for A File Manager (Tauri desktop app, packaged as MSIX)

1. Launch: install the MSIX and start "A File Manager" from the Start menu. No sign-in, account or license key is needed; the core features work fully offline.

2. Network: the core flow needs no network access. Optional extensions (rclone remoting, AI skills) are off by default and are not required to evaluate the app.

3. Suggested flow: the left sidebar lists drives and quick locations; single-click expands, double-click opens a folder in the active pane. Use the second pane to compare two locations and drag a file between panes to copy or move it. Ctrl+Z undoes the last file operation; deleted items go to the Recycle Bin and can be restored from there.

4. Work Groups: the "Work Groups" entry in the sidebar groups related folder locations. Creating a group does not move or copy any file on disk.

5. Projects: a Project is created from a Work Group and creates a new folder that contains CONTEXT.md, tasks.json, .skills/ and mcp.json. Nothing outside that new folder is modified.

6. "Open with IDE / terminal" only lists editors detected on the machine. It is optional and not required for review.

7. UI language follows the system language (Simplified Chinese, Traditional Chinese, English, Japanese, German, French are included).

8. WebView2 runtime is required; if it is missing the installer prompts for it.

Contact: awam@awam.wang
Publisher: Tianfu New Area Chengdu Huayang Zhicun Software Development Studio (智存软件开发工作室)
```

### 3. 其余选项

- 发布：**通过认证后立即发布**（默认项）
- 定价：市场组 `Default` 基本价格 `0`（免费），市场与可见性沿用默认
- 属性：类别 `工具` → `文件管理器`（`UtilitiesAndTools` / `FileManager`）
- 隐私策略：<https://software.awam.wang/privacy/>
- 年龄分级：IARC 问卷全部回答「否」，预览后同意条款保存

---

## 五、账号与打包标识

| 字段 | 值 |
|---|---|
| Identity Name | `AwamMWang.AFileManager` |
| Publisher | `CN=205330CE-5E66-4AF5-B35C-25EC4109E6AB` |
| PublisherDisplayName | `Awam M Wang` |
| PFN | `AwamMWang.AFileManager_b9vksr1wb7frj` |

> 以上为**个人开发者账号**的字段。若公司账号审核通过，Publisher GUID 与 Identity Name 会完全不同，届时需按新字段重新打包并重新提交。

---

## 六、被驳回时的常见原因与处理

1. **10.1 功能不完整**：截图或功能条目与实际不符 → 按本目录顺序核对截图，必要时重截。
2. **`runFullTrust` 理由不充分**：补充说明为何不能改用 AppContainer，可引用具体 API（回收站、进程启动、剪贴板）。
3. **文件操作可能导致数据丢失**：确保「删除进回收站 + Ctrl+Z 撤销」在认证说明中显著写出。
4. 修改后创建 Submission 2 重新提交，**无需重新上传包**（可直接沿用已验证的 msix）。
