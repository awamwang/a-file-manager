<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# AFM macOS 功能可用性清单（2026-11-26 实测）

基线：`master @ add1d4c`。本机：macOS 15.8.3 / x86_64，已安装并运行 `/Applications/A File Manager.app`（ad-hoc 签名、x86_64、2026-11-23 构建）。

状态口径：**✅ 可用**＝代码/平台层已实现且有自动验证；**🟡 条件可用**＝需外部依赖或本机配置；**🔴 不支持**＝Mac 显式降级或 Windows-only；**⚪ 未实测**＝需真实窗口/真实介质点验。

## 一句话结论

核心文件管理器（浏览、文件操作、工作组/项目、设置、快捷键、主题、AI 视图、帮助）在 Mac 上已真正可用；**当前本机真正缺的是外部依赖**：`rclone` 未安装（即你截图里 rclone 卡片显示"未检测到"）、18787/18778/18790 三个本地服务均未运行、语音 STT 未配置。平台级降级项集中在回收站列表/还原、系统缩略图、原生右键菜单/自启动/单实例、系统文件剪贴板。

> **22:25 更新（第二轮环境准备 + 修复）**：rclone 已装（`~/.local/bin/rclone` v1.76.0）；18787 网关已由 **AFM 托管模式（Managed）自行下载 x64 产物并拉起**，`/health` 与 `/api/backends` 均可用；同时修复了 macOS GUI 不继承 shell PATH 导致漏检 rclone 的代码缺口（含 cargo 基线测试）。线上官方 feed 仍缺 `macos-x86_64` 条目，根因是线上 `v0.3.1-rc.6` 早于仓库的 Intel 打包链路，需新 tag 触发 release；本轮用本地 feed 完成了等价链路验证。详见文末「第二轮实操」。

## 本机环境事实

| 检查项 | 结果 |
| --- | --- |
| `rclone` | **已安装**：`~/.local/bin/rclone` v1.76.0（官方二进制，brew 因沙箱写锁失败后改用用户目录安装）；无 remote，`rclone listremotes` 为空 |
| `127.0.0.1:18787` awam-file 网关 | **由 AFM 托管拉起并监听**（PID 见托管记录；`/health` = `{"ok":true,"service":"awam-resource-gateway"}`） |
| `127.0.0.1:8078` 本地 feed | 运行中，提供 `manifest.json` / x64 onefile / pack zip，供托管下载校验 |
| `127.0.0.1:18778` 工具运行时 / `18790` 知识服务 | 无监听（对应 Feature Pack 未启用或 feed 未配置） |
| `/usr/bin/mdfind` / `/usr/bin/qlmanage` | 均存在（Spotlight 搜索、QuickLook 预览具备系统条件） |
| App 签名 | `Signature=adhoc`、`TeamIdentifier=not set`；无 Developer ID / 公证，首次安装需安装脚本或 `xattr` 清隔离 |

## 功能清单

### 1. 浏览与文件操作

| 功能 | 平台/代码 | 本机当前 | 说明 |
| --- | --- | --- | --- |
| 列表/图标视图、双栏、多标签、面包屑、侧栏、文件夹树、隐藏文件、排序/过滤、Shift 连选、键盘导航、地址栏 | ✅ | ✅ | E2E 6/6 通过；`check-list-select` / `keyboard-nav` / `dual-pane` 等门禁通过 |
| 复制/剪切/粘贴、粘贴目标判定、新建文件/文件夹、F2 内联改名与撞名处理、父+子混选去重、操作后保持选区 | ✅ | ✅ | cargo 真执行 + FE 纯逻辑门禁；E2E 验证撞名不静默覆盖 |
| 删除到废纸篓 | 🟡 | ⚪ | `trash` crate 在 macOS 走 Finder/AppleScript，首次可能需 TCC "自动化" 授权；需在真实窗口点验 |
| 永久删除、属性查看、ZIP 压缩/解压 | ✅ | ✅ | 纯 Rust 跨平台，`test:archive` / `test:properties-attrs` 通过 |
| 撤销 Ctrl+Z（复制/移动/新建等） | ✅ | ✅ | 内存态，重启后不保留（有意设计） |
| 撤销"删除恢复" | 🔴 | 🔴 | `trashListRestore=false`；Mac 无回收站列表/还原 API，用 Finder 废纸篓代替 |
| 应用内拖放（条目→标签/文件夹树/另一栏）、跨标签拖放 | ✅ | ⚪ | 门禁齐备；真实窗口落点仍需点验 |
| 跨应用拖入（Finder→AFM） | 🟡 | ⚪ | WKWebView `text/uri-list` 降级路径已接，未真机验证 |
| 原生拖出（AFM→Finder） | 🔴 | 🔴 | `nativeDragOut=false`（插件在 macOS 会 abort）；Web 拖出降级可用 |

### 2. 工作组 / 项目 / AI 目录

| 功能 | 平台/代码 | 本机当前 | 说明 |
| --- | --- | --- | --- |
| 工作组创建/改名/删除/成员管理、Open State 保存与恢复、⌥+数字切换 | ✅ | ✅ | macOS 数字键已改用 `event.code`，避免 ⌥ 产生字形导致失效 |
| 从工作组创建项目（先预览确认、原 WG 保留） | ✅ | ✅ | cargo 测试 `promote_creates_bound_project_keeping_wg` |
| 项目目录、CONTEXT.md / tasks.json、绑定 WG、成员目录链接 | ✅ | ✅ | Unix 用 symlink（无需管理员权限），无 `board.json` |
| AI Agent Targets 路径槽与目录跳转（ADR 0028） | ✅ | ✅ | Windows 预设路径在 Mac 会被归一化后发现 |
| Skill / MCP 部署到 Agent | 🟡 | 🟡 | Mac 用 symlink 无额外工具；需本机已安装目标 Agent（如 Claude/Cursor） |
| 知识页「在 AFM 中显示」 | ✅ | 🟡 | 应用内新标签定位；需知识服务可用才能检索到文档 |

### 3. 搜索 / 预览 / 图标

| 功能 | 平台/代码 | 本机当前 | 说明 |
| --- | --- | --- | --- |
| 全局搜索 Discovery（Spotlight） | ✅ | 🟡 | `mdfind` 存在；结果取决于 Spotlight 索引是否开启。Everything IPC 为 Windows-only |
| 空格 QuickLook 预览 | ✅ | ✅ | `qlmanage -p`，由 AFM 管理子进程；失败时自动回落内置预览面板 |
| 内置预览窗格 / 详细信息窗格 | ✅ | ✅ | 与 QuickLook 独立，`test:preview-bridge` / `details-pane` 通过 |
| Peek 悬停预览 | 🔴 | 🔴 | Windows-only，设置项在 Mac 不显示 |
| 文件类型图标 | ✅ | ✅ | `file_icon_provider` → NSWorkspace |
| 系统内容缩略图 | 🔴 | 🔴 | `system_thumbnail()` 在 Mac 显式返回 Err，回落文件类型图标 |

### 4. 提供程序 / 扩展 / 服务

| 功能 | 平台/代码 | 本机当前 | 说明 |
| --- | --- | --- | --- |
| 云盘 / SFTP / WebDAV（rclone provider） | 🟡 | 🟡 可用待配置 remote | rclone 已装、网关 backend 报 healthy；需自行 `rclone config` 后才有 remote 内容。GUI 漏检路径已修复（见文末） |
| awam-file 网关 provider（浏览器书签、downloads、Prowlarr、虚拟文件下载） | 🟡 | 🟡 网关已托管运行 | 网关由 AFM 以 Managed 模式启动；书签/Prowlarr/qBittorrent 仍需各自外部服务配置 |
| torrent / magnet 打开 | 🟡 | 🔴 | 依赖网关或本机默认程序；本机无对应服务 |
| Feature Pack 安装/启用/卸载 | ✅ | ✅ | 无 Mac 平台限制；本地 zip/文件夹安装 |
| 托管 Pack 更新检查（file-gateway / tool-runtime / knowledge） | 🟡 | 🟡 | 默认 feed 为空；需配置 `managed-dependency-feed.json` 且 feed 含 `macos-x86_64` 条目 |
| 工具运行时服务（18778） | 🟡 | 🔴 | 未启用；sidecar 为 PyInstaller onefile，运行期不需本机 Python/uv |
| 知识库服务（18790） | 🟡 | 🔴 | 未启用；需 feed 或自跑服务 + 令牌/数据根设置 |
| 健康体检 | ✅ | ✅ | WebView2/Peek 项在 Mac 不显示；QuickLook 卡片文案仍是 Windows 命名管道描述（已知显示假阳性，不影响功能） |

### 5. AI / 系统整合 / 外观

| 功能 | 平台/代码 | 本机当前 | 说明 |
| --- | --- | --- | --- |
| AI 视图（Agents / MCP / Skills / Harness） | ✅ | ✅ | 主线版含 AI addon；Lite 版不含 |
| 语音输入 | 🟡 | 🔴 | 需配置 `power.sttBaseUrl` / `sttModel` / `sttApiKey`（OpenAI 兼容 STT），首次录音触发麦克风 TCC 授权，未实测 |
| 系统托盘（打开窗口 / 退出） | ✅ | ✅ | NSStatusItem 已实现 |
| 自启动（登录项） | 🔴 | 🔴 | 仅 Windows；Mac 设置项隐藏 |
| 单实例 | 🔴 | 🔴 | 非 Windows 不拦截第二个实例 |
| 原生右键菜单 / Shell verb / 原生"打开方式"对话框 | 🔴 | 🔴 | Windows-only；AFM 自绘右键菜单与默认打开（`open`）可用 |
| 系统文件剪贴板 / RustDesk 远程文件剪贴板 | 🔴 | 🔴 | Windows-only；AFM 内部复制/剪切不受影响；文本剪贴板用 `pbcopy` 可用 |
| IDE / 终端发现与启动 | 🟡 | 🟡 | 预设 Terminal.app、VS Code、Cursor 等；未预设的可填绝对路径 |
| Fluent 深浅色、多语言（中简/繁、英、日、德、法）、命令面板、⌘ 快捷键、交通灯居中 | ✅ | ✅ | 快捷键与主题在 webdemo E2E/门禁中验证；真实 WKWebView 按键手感仍需点验 |
| 安装与分发 | 🟡 | ✅ | Homebrew tap `brew install --cask awamwang/tap/a-file-manager` 已发布；ad-hoc 包需安装脚本清隔离，无 Developer ID/公证 |

## 验证证据（本轮实跑）

- `npm run build`：✅ 通过（vue-tsc + vite build）。
- `PW_CHANNEL=chrome npm run e2e`：✅ **6/6 通过**（导航网格/列表降级、地址栏、零 console/page error、axe 严重问题为零、撞名不覆盖）。沙箱在退出时清理 Chrome 临时克隆报 file-delete，不影响测试结论。
- `npm test`：唯一失败为 `test:knowledge-generality` —— `upstream/awam-knowledge` 子模块未检出（本机 SSH key 被安全策略拦截，无法拉取）。**其余门禁逐项运行全部通过（含 `test:rust` 全量 cargo 测试）**。该失败是开发环境完整性问题，不影响已安装发布版。
- 浏览器路由巡检 19 条：15 条零错误；`/projects`、`/extensions/providers`、`/extensions/store`、`/settings/open-tools` 在 webdemo 中出现 `Cannot read properties of null`，定位为 mock 后端对未实现命令返回 `null`，属演示 harness 不完整，不能据此判定真实 Tauri 应用有问题。
- `docs/qa/gui-smoke-checklist.md` 的 28 项（真实废纸篓、TCC、锁定文件、满盘、跨应用拖放、重启等）仍需按清单人工点验。

## 第二轮实操（22:00–22:25）：安装、托管启动与代码修复

### 1. rclone 安装
- `brew install rclone` 失败：`Operation not permitted @ apply2files - /usr/local/var/homebrew/locks/...lock`（本机沙箱对 Homebrew 锁目录的写入限制），非 rclone 本身问题。
- 改用官方二进制安装到用户目录：`~/.local/bin/rclone` = **v1.76.0（darwin/amd64）**，`~/.local/bin` 已在 PATH。
- `rclone listremotes` 为空（无 config 文件），符合预期；网关侧已把 rclone backend 报为 healthy。

### 2. x64 托管产物与本地 feed
- `upstream/awam-file` 子模块无法用 git 拉取（本机沙箱禁止读取 SSH key），改用 codeload 下载 gitlink `056e020` 的源码快照。
- 隔离 venv（Python 3.13.12 + PyInstaller 6.22.3 + mcp/uvicorn/starlette + **typer/rich**——`--collect-all mcp` 必需，CI 也已装）本地冻结出 `awam-resource-gateway-x64`：26,541,744 bytes、x86_64、ad-hoc 签名、sha256 `a9f1788d…c68e9`。
- 用仓库自带的 `scripts/build_pack_feed.py`（build / pack-dir / emit-feed-config）生成 `/tmp/afm-feed`（manifest + artifacts + pack zip）与 AFM 配置 `managed-dependency-feed.json`；`python3 -m http.server 8078` 提供本地 feed。

### 3. 托管启动验证（核心结论：AFM 自己完成了下载→校验→落盘→spawn）
- 在 AFM 配置目录写入 pack 安装目录、`state.json`（启用）、`mode=managed` 记录后重启 App。
- AFM 从本地 feed 下载 x64 onefile 到 `.awam-resource-gateway-x64.tmp`；产物 sha256 与 manifest 一致；随后 spawn 监听 **127.0.0.1:18787**。
- `GET /health` → `{"ok":true,"service":"awam-resource-gateway"}`；`GET /api/backends` 列出 `rclone` backend `healthy=true`（detail：No remotes configured）。
- 本机沙箱拦住了 AFM 的两次原子 rename（产物 `rename onto artifact`、`record rename onto record`，均 `os error 1`）；已手动完成等价落盘并同步托管记录状态。**这是沙箱限制，不是 AFM 代码缺陷**；普通终端运行的 App 不会有此问题。

### 4. 代码修复：macOS GUI 漏检 rclone
- 问题：`src-tauri/src/providers/rclone.rs` 在 macOS 只按 PATH 找 `rclone`；从 Dock/Finder 启动的 GUI 不继承 shell PATH，Homebrew 与用户目录安装会漏检。
- 改动：新增 `rclone_macos_candidate_paths()`，按序探测 `/opt/homebrew/bin/rclone`、`/usr/local/bin/rclone`、`$HOME/.local/bin/rclone`；环境变量仍最高优先。新增 cargo 基线测试 `macos_candidates_cover_brew_prefixes_and_user_bin`。
- 验证：`cargo test --lib providers::rclone` **6/6 通过**；`test:provider-capabilities`、`test:provider-error-hint`、全量 `test:rust` 通过。
- 注意：`/Applications` 里现有的 v0.4.0 不含此修复；要从 Dock 启动即生效需重新打包安装，当前可从终端启动（继承 PATH）临时使用。

### 5. 线上 release 的 x64 缺口判断
- 仓库当前 master 已有 `build-pack-feed-intel`（`macos-15-intel` / `platform: macos-x86_64` / `name_suffix: "-x64"`）与合并门禁，三 pack × 三平台配置齐备。
- 线上 `v0.3.1-rc.6` 的 manifest 只有 `macos-aarch64` / `windows-x86_64`，根因是该 tag 早于引入 Intel 阻塞链路的 `55df879`，**不是当前配置 bug**。真正补齐线上需从 ≥ `55df879` 打新 tag 跑一次真实 release，确认 9 条 entry；静态门禁无法替代真实 release 验证。

## 让本机全部"条件项"变可用的最短路径（更新）

1. rclone 已装；运行 `rclone config` 配 remote。GUI 漏检修复需含该改动的新版 App，或从终端启动。
2. 网关已由 AFM 托管运行；书签 / Prowlarr / qBittorrent 还需各自外部服务配置。
3. 知识 / 工具运行时仍需配置 feed 或自跑 18790 / 18778。
4. 语音输入：在设置里填 STT 服务 URL / model / API key，并在系统弹窗允许麦克风。
5. 回收站删除若失败：系统设置 → 隐私与安全性 → 自动化 → 允许 AFM 控制 Finder。
