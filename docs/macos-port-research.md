<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# macOS 兼容缺口与跨平台抽象层调研

调研日期：2026-11-14。初版为只读调研；当晚已追加 **PoC 落地与 macOS 实机验证**（第 7 节）。第 1–6 节保留调研时点的原始结论与估算，第 7 节记录实际改了什么、验证了什么、还剩什么。

本文合并三轮调研：① macOS 兼容缺口（Rust 后端 / 前端 / 构建发布三路并行排查）；② 跨平台浏览器实现底层能力的通用机制；③ 生态里可复用的开源中间层与协议层。**第 2、3 节是缺口与证据，第 4、5 节是解法与现成供给，第 6 节是建议的落地顺序。**

相关文档：`docs/explorer-baseline.md`（Windows 基线）、`docs/afm-explorer-dnd-interop-design.md`（OLE 拖放设计）、`docs/adr/0011-skill-deploy-junction-or-copy.md`（Junction 与 copy 回退）、`docs/adr/0014-feature-pack-local-install-only-v1.md`（Feature Pack 安装）。

---

## 0. 一页结论

**这不是「加一个 macOS bundle target」，而是移植第二个平台。** 项目自述 Windows-first（README.md:3、package.json description），`src-tauri/src` 约 3.1 万行 Rust 中 `#[cfg(target_os = "macos")]` **仅 1 处**（`providers/rclone.rs:125`，只是一句 brew 安装提示）；绝大多数平台差异是 `#[cfg(windows)]` / `#[cfg(not(windows))]` 二元分支，而后者**几乎全是「返回 Err / 空值 / no-op」的占位**，不是 Linux/macOS 实现。前端唯一的平台判断是 `FluentIcon.vue:23` 用 UA 决定字体，没有 OS 抽象层。

| 阶段 | 目标 | 粗估 |
|---|---|---|
| P0 | 能编译 + 能启动 + 能浏览 | 5–8 人日 |
| P1 | 主要功能可用（打开/图标/终端/IDE/Workspace/废纸篓/搜索/隐藏/卷信息） | 12–20 人日 |
| P2 | 体验对齐 macOS（剪贴板/右键/自启动/Quick Look/交通灯/桌面集成） | 15–25 人日 |
| P3 | 发布就绪（文案、双架构、签名公证、CI/CD、⌘ 键） | 5–10 人日 |
| **合计** | | **约 37–63 人日**（不含证书申请与公证流程） |

**核心策略**：P0+P1 先做最小闭环；P2 中凡是 Windows 专属集成（Explorer shell verb、Everything、PowerToys Peek / QL-Win、UAC、HKCU Run）**一律先降级禁用并在 UI 隐藏**，不追求原生对等。跨平台浏览器与成熟框架的做法也是如此——能力协商，而非逐平台硬补齐（见第 4 节）。

**两个最大杠杆**：① 把约 30 处 `replace('/', "\\")` / 大小写折叠收敛到单一 `crate::paths` 平台函数（一次改动修掉大半「行为不正确」）；② `shell_icons` 只替换 Win32 取图路径，保留 `file_icon_provider` 与现有缓存队列、批处理、命令。

---

## 1. 现状：硬事实与可复用资产

### 1.1 为什么现在编不过

`windows 0.62.2` crate 源码首行是 `#![cfg(windows)]`，即在 macOS 上整个 crate 被编译为空；`std::os::windows` 模块在非 Windows 根本不存在。因此**任何没有 cfg 保护的 `use windows::...` / `use std::os::windows::...` 都是编译错误（unresolved import），不是链接错误**。全库逐一核对的 17 处其余引用点均已包在 `#[cfg(windows)]` 内，不会再报。

### 1.2 已经具备、无需重做的东西

| 资产 | 位置 / 状态 |
|---|---|
| macOS 应用图标 | `src-tauri/icons/icon.icns` 已存在，且已列入 `tauri.conf.json:42` bundle.icon |
| 废纸篓删除 | `trash 5.2`（Cargo.toml:52）含 `objc2` 依赖，`file_ops/mod.rs:121-129` 的 `trash::delete` 可直接复用 |
| 文件图标 | `file_icon_provider 1.0`（Cargo.toml:53）含 `objc2-app-kit` / UTI，macOS 走 `NSWorkspace` |
| 打开外部资源 | `tauri-plugin-opener` 已在 `lib.rs:101` 注册、权限已配，但 `open_with` 没用它 |
| 原生拖出 | `patches/drag` 自带完整 macOS AppKit 实现（`platform_impl/macos/mod.rs:1-245`），依赖已正确用 `[target.'cfg(target_os="macos")'.dependencies]` 门控（Cargo.toml:61-104） |
| Feature Pack | `extension_host/pack_fs.rs` 的 zip + `copy_pack_tree()` + staging/rename 全部平台中立；ADR 0011 已预留 copy 回退 |
| 权限 / 托盘 | `capabilities/default.json` 全是跨平台权限；`TrayIconBuilder` 在 macOS 映射 NSStatusItem |
| 前端机制 | 快捷键 registry/chord 架构、虚拟 URI（`afm://` / `awam://`）、自绘对话框、图标 LRU 缓存队列、预览会话状态机、主题 token、列表虚拟滚动 |
| 已隔离模块 | `window_icon.rs`、`drag_focus.rs` 整文件 `#![cfg(windows)]`，macOS 直接跳过即可 |

### 1.3 全链路 Windows 硬编码（发布侧）

`ci.yml:16` `windows-latest` + PowerShell 断言 WebView2；`release.yml:35,217,298` 全 `windows-latest`，`flavor_matrix` 无 os/arch/target 维度（`:110-111`），产物 `x64-setup.exe` / `bundle/nsis`（`:263-284`），release notes 硬编码 "Windows NSIS"（`:187-195`）；`build-flavor.mjs:51` `--bundles nsis`；`release-assets.mjs:20-34` 的 `nsisAssetName()`；`check-deps-surface.mjs:18-22` 强制断言 `embedBootstrapper`；`check-arch-optimize.mjs:33` 甚至把 `pathKey.ts` 的 `/`→`\` 归一化当契约断言。**签名基础设施为零**（`.cursor/skills/afm-multi-pack-release/SKILL.md:86` 明确「当前无代码签名证书」）。

文档侧只有隐性的单平台约束，**没有任何 macOS 计划，也没有一句字面「只支持 Windows」**——即尚未决策排除，而非已决策不做。

---

## 2. 缺口清单（按优先级）

标记说明：**【编译】** 不修则 macOS 编译失败；**【运行】** 编译通过但核心功能不可用；**【行为】** 能跑但结果错；**【降级】** 建议显式禁用并在 UI 隐藏。

### P0 · 让它编译并启动（约 5–8 人日）

| # | 事项 | 证据 | 类型 |
|---|---|---|---|
| P0-1 | 3 处编译阻塞：`autostart.rs:6`、`favorites/shell_verb.rs:7` 顶层无条件 `use std::os::windows::ffi::OsStrExt;`（`to_wide()` :21-26 也无 cfg）；`shell_icons/mod.rs:18-28` 无条件 `use windows::{core::HSTRING, Win32::...}` | 已核实代码 | 【编译】 |
| P0-2 | `windows = "0.62"` 移入 `[target.'cfg(windows)'.dependencies]`；`shell_icons` 的 Win32 段（`:124-227`，COM/GDI/IShellItemImageFactory）整体 cfg 化，macOS 用 `file_icon_provider` 兜底 | Cargo.toml:33-50 | 【编译】 |
| P0-3 | 统一路径层：`paths.rs:5-51` 无条件 `replace('/', "\\")` + `:53-55` 比较时 lowercase；`navigator/list.rs:148-155` 同样；后端约 30 处各自 `replace`；前端 `pathKey.ts`、`navHistory.ts`、`fileClipboard.ts:108`、`dnd/paths.ts` 全按盘符 + 反斜杠 | 最大杠杆 | 【运行】 |
| P0-4 | `default_path()` 只读 `USERPROFILE` 再兜底 `C:\` → 启动即 `Err("Could not resolve a default browse path")`；改 `HOME` / `dirs::home_dir()` | list.rs:127-137 | 【运行】 |
| P0-5 | 双击打开：`open_with/mod.rs:42-56` 的非 Windows 分支一律 `success:false`；改用已注册的 `tauri-plugin-opener`（`open_path` / `reveal_item_in_dir`），`open_recycle_bin_system` → `open ~/.Trash` | `lib.rs:101`、`capabilities/default.json` | 【运行】 |
| P0-6 | `tauri.conf.json`：`targets: ["nsis"]` → `["app","dmg"]`，补 `bundle.macOS`、`bundle.resources`（否则 `dragOut.ts:21` 的 `resolveResource("icons/32x32.png")` 打包后缺拖影）、`titleBarStyle: Overlay` + `hiddenTitle`；`bundle.windows.webviewInstallMode` 对 macOS 无意义 | :24-25, :35-50 | 【运行】 |
| P0-7 | `deps_probe.rs:64-90` 把 WebView2 当 required，非 Windows 恒 `false` → 健康页永远报红并给出「安装 WebView2 / NSIS bootstrapper」的错误指引 | :471-490 | 【行为】 |
| P0-8 | CI 加 `macos-latest` matrix、去掉 WebView2/NSIS 断言、release 产 `.dmg`/`.app`；`rust-toolchain.toml` 补两个 apple targets | ci.yml / release.yml | 【运行】 |

### P1 · 主要功能（约 12–20 人日）

| 功能 | 现状 → macOS 需要做什么 | 类型 |
|---|---|---|
| 终端 / IDE | `TERMINAL_DEFS` 只有 `wt/powershell/pwsh/cmd/gitbash` + `C:\Program Files`（`terminal/mod.rs:27-33`）；`IDE_DEFS` 全是 `.cmd` + `C:\Users\...\AppData`（`ide_launch/mod.rs:36-84`）→ `open -a` / `/Applications/*.app` / PATH CLI，`USERNAME`→`USER`/`HOME` | 【运行】 |
| Workspace / AI Skills | `cmd /C mklink /J`（`projects/members.rs:169-195`、`ai/deploy.rs:51-89`）→ Unix symlink；`remove_dir` 还要改 `remove_file`。否则成员创建与 Skills 部署必然失败 | 【运行】 |
| 废纸篓 | `trash::delete` 已可用；但 `recycle.rs` 的 list/restore 是 PowerShell + `Shell.Application` COM + `InvokeVerb('restore')` → 用 `trash` 的 `restore_all` / `~/.Trash` 元数据，或**降级**为「只打开废纸篓」 | 【运行】/【降级】 |
| 全局搜索 | `everything_ipc.rs` 非 Windows 只有 stub，`fallback.rs` 恒空结果 → Spotlight `mdfind` / `NSMetadataQuery`，或**降级**到目录遍历 | 【运行】/【降级】 |
| 隐藏文件 / 属性 | `entry_is_hidden` 非 Windows 恒 false（`list.rs:34-46`）、`attrs.rs:67-88` 忽略 hidden 位 → dot-file + `UF_HIDDEN` / `chflags` | 【行为】 |
| 卷 / 磁盘空间 / 网络位置 | `places.rs:275-295` 非 Windows 仍遍历 `A:\`…`Z:\`；`disk_space.rs:61-65` 恒返回 Err；`list_network_places` 跑 `net use` → `/` + `/Volumes/*` + `statvfs`；`DriveInfo.letter` / `DriveKind` 数据模型需重设计 | 【运行】 |
| 图标 / 缩略图 | Win32 `IShellItemImageFactory` → `NSWorkspace` / Quick Look Thumbnailing；缓存队列 `shellIcons.ts:34-348` 完整复用 | 【行为】 |
| 浏览器书签 / 环境变量 | `LOCALAPPDATA` + `Microsoft\Edge\User Data`（`browser.rs:46-54`、`deps_probe.rs:363-413`）→ `~/Library/Application Support`；`%VAR%` → `$VAR` / `~`（`env_path.rs`） | 【行为】 |
| 进程树清理 | `win_process.rs:28-41` 的 `kill_process_tree` 非 Windows 是空实现 → rclone/sidecar 超时后子进程泄漏；sidecar `python` → `python3` | 【行为】 |

### P2 · 体验对齐（约 15–25 人日）

- **快捷键（性价比最高）**：`shortcuts/registry.ts:10-283` 约 45 条全是 `ctrl:true`，`chord.ts:26-33` 严格等值匹配 → Cmd+C/V/X/A/Z/W 全无响应。给 `ShortcutChord` 加「主修饰键」概念、改 `chord.ts` 的匹配/格式化/显示 3 处 + `registry.ts` 生成时折叠，即可覆盖绝大部分，无需逐个改 handler。另需 Cmd+Backspace 移废纸篓、Cmd+I 属性、Return 重命名、Cmd+R 刷新、Option 键冲突（Option+字母会输入特殊字符）、F 键被媒体键占用、Cmd+[ / ] 导航。
- **标题栏 / 窗口生命周期**：`decorations:false` + 自绘 Win11 三键（`AppShell.vue:1768-1854`），而 `tauri-plugin-frame` 在 macOS 是 no-op（`lib.rs:104-117`）→ 交通灯、原生菜单栏、Cmd+Q / Cmd+W、双击 zoom、窗口圆角与阴影、Electron 的 `-webkit-app-region` CSS 无效。`revealMainWindow.ts` 的 HWND/DWM settle、`closeBehavior.ts` 的「关闭→托盘」语义都要改。
- **拖放**：WKWebView 的 `File` 没有 `.path`，入站解析 `dnd/paths.ts:23-43` 只认 `file:///C:` 与 UNC → 跨应用拖入完全失效；`dnd/surface.ts:30-40` 的 Alt/Shift/Ctrl 语义与 macOS Option/Cmd 相反；出站可退回 JS `startDrag`（`dragOut.ts:26-29` 已留降级分支）。
- **右键 / 属性 / 打开方式**：`TrackPopupMenu` → `NSMenu`/Services 或降级为自绘菜单；`open_native_open_with_dialog` 无等价 API → `open -a` 或降级为默认程序打开；ShellNew 模板、`.lnk`、`- Copy` 命名、UAC 提权项、非法字符表（`file_ops/mod.rs:62-73` 的 Windows 保留字符）统一平台化。
- **空格预览**：`preview_bridge/mod.rs` 的「QuickLook」其实是 Windows 移植版 QL-Win 命名管道，Peek 是 PowerToys；macOS 用 `qlmanage -p` / `QLPreviewPanel`；好消息是 `auto` 模式已能优雅降级到内置预览面板（`:426-456`）。
- **剪贴板 / 自启动 / 单实例**：CF_HDROP → NSPasteboard；HKCU Run → `SMAppService` / LaunchAgent / `tauri-plugin-autostart`；`CreateMutexW` → `tauri-plugin-single-instance`。建议先降级禁用并在设置页隐藏。
- **CSS / 字体 / i18n**：Segoe UI / 微软雅黑 / Consolas 全缺 → SF / PingFang / Menlo；约 50 处 `color-mix()` 在旧版 WKWebView 整条声明失效；5 语言写死 `Ctrl+Shift+P`、`F5`、`Alt+Enter`、`此电脑`、`回收站`、`资源管理器`、`%APPDATA%`、`C:\Windows`、`\\server\share`、`WebView2 Runtime`。
- **无文件变更刷新**：`drag_focus.rs` 的轮询计时器是 Windows-only，Cargo.lock 无 `notify` → macOS 无实时刷新，需引入 `notify` + FSEvents 或前端定时重列。

### P3 · 发布就绪（约 5–10 人日）

双架构 targets（`aarch64-apple-darwin` / `x86_64-apple-darwin`，或 universal binary）、`Info.plist`、`*.entitlements`、`minimumSystemVersion`、hardened runtime、Developer ID 签名 → `xcrun notarytool` → stapler、`APPLE_*` CI secrets、`macOSPrivateApi` 评估、`.app`/`.dmg` 资产命名与 release notes、CI 缓存键加 OS/target 维度、README/SKILL.md 文档平台化。顺序见第 3.3 节的 16 步清单。

---

## 3. 分模块证据详表

### 3.1 Rust 后端（14 个模块，按影响面排序）

| 模块 | 关键文件:行 | 现状 | macOS 处理 | 类型 |
|---|---|---|---|---|
| 1 构建配置 | `Cargo.toml:33-50`、`tauri.conf.json:24-50`、`rust-toolchain.toml:1-4` | windows 依赖无门控；bundle 只有 nsis；无 apple targets | target 门控 + bundle.macOS + resources + Overlay 标题栏 | 【编译】/【运行】 |
| 2 路径（影响面最大） | `paths.rs:5-55`、`navigator/list.rs:127-155`、`navigator/stat.rs:92`、`file_ops/safe_io.rs:18-51`、`favorites/mod.rs:63-121`、`ai/*` 约 15 处 | 分隔符、大小写、盘符、`\\?\` 前缀、非法字符表、UNC 判定 | 收敛到单一平台函数；`statvfs`/`st_dev` 判同卷；持久化过的收藏需迁移 | 【运行】 |
| 3 Shell 图标 | `shell_icons/mod.rs:18-28,124-227` | 无条件 Win32 import（**编译阻塞**）；COM STA、HBITMAP→RGBA、IShellItemImageFactory | 整段 cfg 化；macOS 用 `file_icon_provider` + Quick Look Thumbnailing；缓存/批处理/命令复用 | 【编译】 |
| 4 右键菜单 / shell verb | `favorites/shell_verb.rs:7`（**编译阻塞**）、`shell_menu/mod.rs:96-119` | HKCU 注册表 verb；`IContextMenu` + `TrackPopupMenuEx` | cfg 化 + Finder Services/Quick Action，或降级隐藏 | 【编译】/【降级】 |
| 5 自启动 / 单实例 / 托盘 | `autostart.rs:6`（**编译阻塞**）、`system_tray.rs:171-202`、`lib.rs:104-117` | HKCU Run；`CreateMutexW` 非 Windows 恒 true；frame 插件 no-op | 官方插件或 `SMAppService`；补 `.icon_as_template(true)`；交通灯 | 【编译】/【行为】 |
| 6 回收站 | `file_ops/recycle.rs`（整文件无 cfg）、`open_with/mod.rs:59-73` | PowerShell + COM `NameSpace(10)` | `trash` crate + `~/.Trash`，或只提供打开废纸篓 | 【运行】 |
| 7 外部进程 | `open_with/mod.rs:42-107`、`terminal/mod.rs`、`ide_launch/mod.rs`、`win_process.rs:28-41`、`tool_runtime/sidecar.rs:20` | ShellExecute / `.cmd` / `C:\Program Files`；kill tree 空实现 | 复用 opener；`open -a`；`killpg`；`python3` | 【运行】 |
| 8 预览桥 | `preview_bridge/mod.rs:28-30,146-286,426-456`、`deps_probe.rs:226-312` | QL-Win 命名管道 + PowerToys Peek | 系统 Quick Look；内置面板兜底 | 【行为】 |
| 9 全局搜索 | `discovery/everything_ipc.rs:392-415`、`fallback.rs:5-23` | `WM_COPYDATA`；非 Windows stub；fallback 恒空 | Spotlight 或目录遍历 | 【运行】/【降级】 |
| 10 隐藏/卷/磁盘/环境 | `navigator/list.rs:34-46`、`attrs.rs:67-88`、`places.rs:134-296`、`disk_space.rs:14-65`、`env_path.rs:4-35` | 非 Windows 恒 false / 遍历 A:…Z: / 返回 Err / 原样返回 | dot-file + chflags；`/Volumes`；statvfs；`$HOME`/`~` | 【行为】 |
| 11 Junction / 快捷方式 / 剪贴板 | `projects/members.rs:169-206`、`ai/deploy.rs:51-89`、`shortcut.rs:10-65`、`clipboard.rs:121-284` | `mklink /J`；`"X - Shortcut.lnk"`；CF_HDROP；ShellNew | Unix symlink + `remove_file`；alias/symlink；NSPasteboard | 【运行】/【降级】 |
| 12 拖放补丁 | `patches/drag/Cargo.toml:61-131`、`platform_impl/macos/mod.rs:1-245` | macOS 实现完整且依赖已门控 | **无需重写**；只需验证 `bundle.resources` 与 JS 回退路径 | ✅ 复用 |
| 13 依赖体检 / 书签 / 监听 | `deps_probe.rs:64-90,363-490`、`providers/browser.rs:46-54` | WebView2 required 恒 false；LOCALAPPDATA；无 notify | 跳过 WebView2 项；`~/Library/Application Support`；FSEvents | 【行为】 |
| 14 明确未发现 | 全库 | ADS / `Zone.Identifier`、`SHFileOperation`、`IFileOperation`、`SHGetFileInfo` 零命中；除 4 处无 cfg 的 PowerShell/cmd 调用外其余都在 cfg 内 | 对称；macOS 的 `com.apple.quarantine` / xattr 同样未处理 | — |

### 3.2 前端（按用户可感知程度排序）

| # | 感知 | 问题 | 关键位置 |
|---|---|---|---|
| 1 | 立刻、全面失效 | 所有快捷键 Ctrl 而非 Cmd；删除键、属性键、功能键、Option 冲突、硬件后退键 | `shortcuts/registry.ts:10-283`、`chord.ts:6-60`、`AppShell.vue:1811`、`DiscoveryView.vue:332`、`SettingsView.vue:142-147` |
| 2 | 立刻、功能硬坏 | `fileClipboard.ts:108` 粘贴目标直接拼 `\`；home 兜底 `C:\`；面包屑按盘符切分；`file:///Users/...` 拖入返回 null | `pathKey.ts:3-67`、`navHistory.ts:124-165`、`dnd/paths.ts:11-65`、`guide/index.ts:142-156` |
| 3 | 一开窗口就刺眼 | 右上角 Win11 三键、无 traffic lights、双击最大化、无原生菜单栏、关闭语义、无圆角阴影 | `index.html:10-22`、`AppShell.vue:1768-1854,2648-2733`、`fluent-tokens.css:19-20`、`revealMainWindow.ts:3-36`、`closeBehavior.ts:56-64` |
| 4 | 核心功能无数据（后端更阻塞） | 大量 `invoke()` 指向 macOS 上不存在或返回错的命令 | 全 `src/state/*` 与 `src/views/*` |
| 5 | 每次操作都遇到 | 「此电脑」/盘符/驱动器、UNC、回收站 vs 废纸篓、隐藏文件开关、`.lnk`、UAC 菜单项 | `guide/index.ts`、`folderTree.ts`、`recycle/*`、`showHidden/*`、`contextMenuModel.ts:128-146` |
| 6 | 右键 / 双击明显不对 | 原生右键菜单、打开方式对话框、ShellNew 模板、图标与缩略图缺失 | `shellMenuEmbed.ts:13-28`、`navigatorBrowse.ts:1026`、`newFileTemplates.ts`、`shellIcons.ts:280-294` |
| 7 | 空格预览不符 | QL-Win / Peek 而非系统 Quick Look；设置枚举只有 `peek/quicklook/builtin` | `previewBridge.ts:3-19`、`settingsFields.ts:71-77` |
| 8 | 跨应用拖放失效 | WKWebView 无 `File.path`；入站只认盘符；修饰键语义相反；出站 OLE | `dnd/paths.ts:9,23-43`、`dnd/surface.ts:30-40`、`dnd/mode.ts`、`dragOut.ts` |
| 9 | 文案全在说 Windows | 5 语言写死 `Ctrl+Shift/P`、`F5`、`Alt+Enter`、`此电脑`、`回收站`、`资源管理器`、`%APPDATA%`、`C:\Windows`、`\\server\share` | `src/i18n/locales/{en,zh-CN,ja,de,fr}.json`：`commandsShortcut`、`chrome.refreshTitle`、`properties.multiHint`、`guide.rootLabel`/`drive*`、`recycleBin.*`、`libraryRootHint`、`blacklistPlaceholder`、`installPathPlaceholder`、`networkEmpty`、`health.*.webview2`、`osVerb` 等约 20 组字段 |
| 10 | 整体排版偏移 | Segoe/雅黑字体栈、`font-synthesis:none`、标题栏高度与 snap 插件硬同步 | `fluent-tokens.css:27-49`、`base.css:17-26` |
| 11 | 设置/健康页误导 | WebView2 检查项、`app-region` CSS、`color-mix()`、托盘文案 | `main.ts:18-28`、`tauri.conf.json:45-49`、约 50 处 `color-mix` |
| 12 | 细节观感 | 大小写折叠路径键、drive-*/onedrive 色调 token 无用、18px PNG 无 @2x | `pathKey.ts`、`favorites.ts`、`public/app-icon.png` |

**正例（无需改）**：`NavigatorView.vue:1329-1339,1389-1399` 已用 `ctrlKey || event.metaKey`；`shellIconThumbPolicy.ts:52` 同时处理 `/` 与 `\`；`provider/path.ts:73-76` 已明确禁止整串 lowercase；自绘对话框规避了 WKWebView 原生 dialog 差异；`revealItemInDir` / `openUrl` 已是跨平台 API。

### 3.3 构建 / CI / 发版 / 签名

**主题与证据**

| 主题 | 关键证据 |
|---|---|
| CI | `ci.yml:16` 唯一 runner `windows-latest`；`:41-46` PowerShell + WebView2 断言；`:33` rust-cache 键无 OS/target；`:52` `test:deps-surface`；`:56-58` `cargo test --lib` 无 `--target` |
| Release | `release.yml:110-111` matrix 只有 `{flavor, vite_ai}`；`:241-245` 缓存键 `release-${flavor}` 会与 macOS 产物串味；`:250-255` WebView2 校验；`:257-284` NSIS 产物路径与命名；`:298` feature-pack job 也在 Windows；`:330` finalize 是唯一 ubuntu job |
| 发版脚本 | `release-assets.mjs:20-39` 命名中枢硬编码 `x64` / `.exe` / `nsis`；`build-flavor.mjs:51` `--bundles nsis`；`build-release-artifacts.mjs:63-82` 路径 `bundle/nsis/`；`prepare-release.mjs:38-46` 资产计划；`sync-version.mjs` **完全平台中立**；`package-feature-packs.mjs:32-37` 的 `tar -a` 依赖 bsdtar（macOS 自带可用，Linux GNU tar 会失败） |
| 校验脚本 | `check-multi-pack-release.mjs:41-50` 断言 `.exe` 精确名；`check-deps-surface.mjs:18-22` 断言 embedBootstrapper；`check-arch-optimize.mjs:33` 断言 `/`→`\` 归一化；`check-autostart.mjs:49` 断言注册表；大量 `*-checks.ts` 夹具是 Windows 路径（纯字符串 mock 仍能跑通，但不提供 macOS 覆盖） |
| Feature Pack / flavor | Core/AI 是编译期 Cargo feature + `VITE_AFM_AI_ADDON`，概念平台中立；`pack_fs.rs:61-159` zip-slip 已兼容两种分隔符、reparse point 判断已 cfg 化 → **无需为 macOS 改造**。Windows 专属机制集中在 AI Skill 的 Junction、自启注册表、Explorer verb、NSIS/WebView2 |
| 健康体检 | 唯一「必装」项 WebView2 在 macOS 恒 false → `required_ok=false` 永久报红；其余 Peek / QL-Win / 书签 / 终端均为 Windows 可选项 |
| 文档 / ADR | README.md:3,9,16,18,26-28,59,64-66 全是 Windows 描述；`explorer-baseline.md` 以资源管理器为基线；ADR 0011 已允许 copy 回退；**无 macOS 路线图** |
| macOS 缺失项 | 双架构 targets、`.app`/`.dmg`、`macos-latest`、Developer ID 证书、`notarytool`/stapler、`APPLE_*` secrets、entitlements、hardened runtime、`Info.plist`、`minimumSystemVersion`、`macOSPrivateApi` 评估、universal binary、缓存 key 维度——**全部为零**（`icon.icns` 是唯一例外） |

**可执行改造清单（构建 → 打包 → 签名 → CI → 发版）**

1. `rust-toolchain.toml` 加 `targets = ["aarch64-apple-darwin","x86_64-apple-darwin"]`；文档补 Xcode CLT 前置。
2. `Cargo.toml` 把 `windows` 移入 target 门控；修 `autostart.rs:6` / `favorites/shell_verb.rs:7` / `shell_icons/mod.rs:18-28` 三处阻塞。
3. macOS 实机跑 `cargo check --target aarch64-apple-darwin`（Core `--no-default-features`、AI 默认）+ `npm run build`。
4. `check-packaging-flavors.mjs` 的 cargo 命令支持 `--target`；`check-deps-surface.mjs` 的断言改为仅 Windows 生效。
5. `tauri.conf.json` 加 `app`/`dmg` + 完整 `bundle.macOS`；新增 `Info.plist` / `*.entitlements` / dmg 布局；声明最低系统版本。
6. `build-flavor.mjs` 按平台注入 `app,dmg` 与 `--target`；`build-release-artifacts.mjs` 按平台选 bundle 子目录。
7. 本地验证产出 `A File Manager.app` / `.dmg`；确认 macOS bsdtar 能生成 Feature Pack zip。
8. 申请 Developer ID 证书 + App Store Connect / Apple ID；CI 注入 `APPLE_*` secrets。
9. 配 `signingIdentity` + `hardenedRuntime` + entitlements；CI 加 `codesign` → `notarytool submit --wait` → `stapler staple`。
10. 更新 SKILL.md:86 的「当前无代码签名证书」与 README Requirements。
11. `ci.yml` 拆 `windows-latest` + `macos-latest` matrix；PowerShell 步骤改 Node 脚本 / bash；缓存键加 OS/target。
12. `release.yml` 的 `flavor_matrix` 扩为 `{flavor, vite_ai, os, runner, target, bundles, asset_kind}`；新增 `build-app-macos` job；移除 macOS 侧 WebView2 校验。
13. 同步更新 `check-multi-pack-release.mjs` / `check-autostart.mjs` / `check-arch-optimize.mjs` 的契约断言。
14. `release-assets.mjs` 命名中枢加 dmg/app 资产名函数（含 arch 维度）；`prepare-release.mjs` / dry-run 输出随之更新。
15. `release.yml:187-195` release notes 按平台列资产；复用现有 `gh release upload --clobber` 机制。
16. 更新 README / SKILL.md；**新增 ADR（建议 0015「macOS 支持范围与签名/公证策略」）**，明确是否支持 MAS 沙箱、universal binary，以及 Windows 基线能力的替代方案。

---

## 4. 跨平台浏览器怎么实现底层能力

**核心答案**：不是「每个平台都写一套功能」，而是三件事——**定义与 OS 无关的上层契约 → 把差异压进一层薄适配器、由构建系统在编译期选择实现 → 对没有等价物的能力做能力协商与降级**。业务代码里几乎看不到 `if (is_mac)`。

| 机制 | 真实内核里的样子 | 对应到本项目的 Rust/Tauri |
|---|---|---|
| 中立契约 | Firefox `nsIFile.idl` 原文：*"is the correct platform-agnostic way to specify a file; you should always use this instead of a string"*——路径必须是对象，字符串会泄漏分隔符和盘符；`nsIDragService` 用 `DRAGDROP_ACTION_COPY/MOVE/LINK` + MIME data flavor，而非 `CF_HDROP` / `DROPEFFECT_LINK` | IPC 契约里禁止出现 `DriveInfo.letter`、`"X - Shortcut.lnk"`、`%APPDATA%`、Alt/Shift/Ctrl 拖放语义 |
| 编译期选择 | Chromium `ui/base/clipboard/BUILD.gn`：公共 `clipboard.cc/.h` 无条件编译，`if (is_mac) sources += ["clipboard_mac.mm"]`、`if (is_win) += ["clipboard_win.cc"]`，macOS 分支挂 AppKit / UniformTypeIdentifiers frameworks | `[target.'cfg(windows)'.dependencies]` + `#[cfg] mod windows` / `mod macos`，而不是 30 个模块里散落 `cfg` |
| 工厂 / DI | Firefox `widget/cocoa/nsWidgetFactory.mm` 把 contract ID `@mozilla.org/widget/clipboard;1` 注册到 `nsClipboardConstructor`，上层只拿 `nsIClipboard` | 启动时绑定 trait 实现，连「哪个实现」都不必硬写死 |
| 能力协商 | `nsIClipboard.supportsSelectionClipboard()` / `hasDataMatchingFlavors()`；X11 特有的 PRIMARY selection 不假装有，让调用方先查询 | `PlatformCapabilities` + `platform_capabilities()` 命令，前端按结果显隐菜单/设置/健康项/快捷键标签 |
| 进程隔离 | renderer 在沙箱里不能碰 OS，一切通过 Mojo IPC 向 browser process 请求；更高层收敛成 W3C 语义（Clipboard API、DnD、File System Access） | **Tauri 的 IPC command 恰好扮演同一角色**，所以 Vue 不该、也没必要知道 Win32 或 AppKit |

**为什么对本项目格外重要**：Electron 白拿 Chromium 的 PAL（剪贴板、拖放、路径、文件选择都由内核适配好）；**Tauri 的系统 WebView（WebView2 / WKWebView）只负责渲染，不提供任何平台抽象层**，这层得应用自己建——正是 30 多处 `replace('/', "\\")`、87 处 `cfg(windows)`、37–63 人日的根因。更贴近的直接参考是 VS Code：`vs/base/common/platform.ts` 只做最薄判断，能力都走 `vs/platform/` 的 service 接口。

**能力协商与降级矩阵（建议直接照抄为 `PlatformCapabilities` 字段）**

| 能力 | Windows 实现 | macOS 是否有等价 | 建议 |
|---|---|---|---|
| 删除到废纸篓 | `trash::delete` | ✅ `trash` crate | 复用 |
| 废纸篓列表 / 还原 | Shell.Application COM | 部分（`~/.Trash` 元数据） | 先降级为「只打开废纸篓」 |
| 系统缩略图 | IShellItemImageFactory | ✅ Quick Look Thumbnailing | 替换 |
| 全局搜索 | Everything IPC | ✅ Spotlight `mdfind` | 先降级目录遍历 |
| 原生右键菜单 | IContextMenu / Shell verb | 部分（Services/Quick Action） | 降级为 AFM 自绘菜单 |
| 开机自启 / 单实例 | 注册表 / Mutex | ✅ SMAppService / 官方插件 | 复用官方插件 |
| 跨应用文件剪贴板 | CF_HDROP | ✅ NSPasteboard | 后期实现 |
| 原生标题栏 / Snap | `tauri-plugin-frame` | ❌ 无等价 | macOS 用 `titleBarStyle: Overlay`，隐藏 Snap 相关 UI |
| UAC 提权 / NTFS ADS / 盘符 | ✅ | ❌ | 隐藏，不做假实现 |

另一个容易被忽略的红利：能力层能变成**测试替身**（浏览器内核有 `TestClipboard` 一类 mock），项目的 `check-*.mjs` 静态检查也能在纯逻辑层验证平台分支，不必三台真机。

---

## 5. 开源实现供给：有什么可以直接用

**结论**：有，而且项目里已经装了一套——**Tauri 本身就是「中立 IPC 契约 + 各平台实现」**（`tao` 抽象窗口、`wry` 抽象 WebView、`app.path()` 已是跨平台特殊目录解析）。但生态只做到**能力级**，没做到「文件管理器语义级」：没有一个库把回收站还原、系统缩略图、Shell 右键菜单、Everything/Spotlight、拖放互通打包成一套中立 trait 并实现三平台。

### 5.1 已在手边却没用起来的协议层（Tauri 官方 plugins-workspace）

| 上一节的缺口 | 官方插件 |
|---|---|
| HKCU Run 注册表自启动 | `tauri-plugin-autostart` |
| `CreateMutexW` 单实例 | `tauri-plugin-single-instance` |
| 前端完全没有 OS 抽象 | `tauri-plugin-os`（`platform()`） |
| 打开方式 / 原生对话框 | `tauri-plugin-dialog`、`opener`（**已装**） |
| 文件系统、窗口状态、全局快捷键 | `fs`、`window-state`、`global-shortcut` |

项目目前只用了 `opener`，等于把一批现成 PAL 实现白放着。官方有插件平台支持矩阵可查，比自己写 `#[cfg]` 稳。

### 5.2 社区插件（补「文件管理器语义」）

| 需求 | 候选 |
|---|---|
| 剪贴板文件互通（CF_HDROP → NSPasteboard） | `tauri-plugin-clipboard`：text/image/html/rtf/**files** + 变更监听 |
| macOS 原生拖出 | `tauri-plugin-dragout`（file promise）；与项目自带 `patches/drag` 的 macOS 实现**二选一** |
| 原生右键菜单 | `tauri-plugin-context-menu` |
| macOS 权限（Full Disk Access 等） | `tauri-plugin-macos-permissions` |
| 扩展文件操作 | `tauri-plugin-fs-pro` |

⚠️ 社区插件的维护状态、Tauri 2 兼容性与许可证要逐个核对，不要一口气全引。

### 5.3 Rust crate 级适配器（能力级 PAL）

| 痛点 | crate |
|---|---|
| 30 处 `replace('/', "\\")`、盘符 | `typed-path`（把 Windows/Unix 路径语法类型化、与宿主 OS 无关）、`camino` |
| `USERPROFILE` / `%APPDATA%` | `directories`、`dirs` |
| 卷列表 + 磁盘空间 | `sysinfo::Disks`（macOS/Windows/Linux，替掉 `GetDriveTypeW` / 手写 `statvfs`） |
| 无文件变更刷新 | `notify`（FSEvents / inotify / ReadDirectoryChangesW） |
| 废纸篓删除、文件图标 | `trash`、`file_icon_provider`（**项目已有**） |
| 文本/图片剪贴板、原生对话框 | `arboard`、`rfd` |

### 5.4 让契约真正成为「协议」

`tauri-specta`（MIT）：从 `#[tauri::command]` 生成 TS 类型 + 事件类型，前后端不再靠手写 interface 对齐；同类还有 `TauRPC`、`ts-rs`、`tauri-plugin-graphql`，Spacedrive 用的是 `rspc`。注意 `tauri-specta` v2 仍是 RC，且 Tauri/Specta 大版本必须配对。这层能防止 `DriveInfo.letter` 这类平台概念继续泄漏进契约。

### 5.5 架构范本（抄思想，别整包引入）

- **VS Code（MIT）**：最干净的教科书。`IFileService` 按 URI scheme 路由到 `IFileSystemProvider`，provider 用 `FileSystemProviderCapabilities` 位标志声明能力、由 `hasCapability()` 查询；OS 能力统一经主进程 `NativeHostMainService` 暴露给 renderer。与「trait + 能力协商 + IPC 边界」一一对应。
- **Apache OpenDAL（Apache-2.0）**：ports-and-adapters 的极致——`Operator` 是 port，`services/*` 50+ 后端是 adapter，`layers/*` 是可组合装饰器，还有 `capabilities` 与共享 behavior test suite（同一套正确性测试跑所有后端）。**架构值得抄，但不建议引入**：面向云存储，不碰图标/废纸篓/Shell verb。
- **Spacedrive（AGPL-3.0）**：Rust `sdcore` + Tauri + VDFS，UI 无业务逻辑，CQRS over RPC，macOS 侧挂 Swift 原生扩展。**架构最贴近本项目，但许可证要小心**。
- GLib `GFile`/GIO 是经典 VFS 抽象，对 Tauri 应用偏重。

### 5.6 真正必须自己写的，只剩薄薄一层

1. **中立数据模型**：`FsPath` / `Volume` / `Trash` / MIME flavor（用上面 crate 兜底，禁止字符串拼 `\`）。
2. **`PlatformCapabilities` + `platform_capabilities()` 命令**：没有任何库代劳，约半天工作量，但它是所有 `cfg(not(windows))` 占位的统一归处。
3. **一层 trait 包住现成 crate**：图标/缩略图、搜索 provider、废纸篓还原。
4. **平台化 UI**：primary modifier（Ctrl→⌘）、交通灯标题栏、原生菜单栏——本质上无库可代劳。

### 5.7 许可证

项目是 GPL-3.0-or-later：Tauri 官方/社区插件多为 MIT 或 Apache-2.0（兼容）、VS Code MIT、OpenDAL Apache-2.0、`tauri-specta` MIT 都可用；**Spacedrive 是 AGPL-3.0**，直接搬代码会把网络交互条款带进来，只借架构思路最稳妥。

---

## 6. 建议的落地方案

### 6.1 架构决策（建议写进 ADR 0015 前置讨论）

1. **保住已有 IPC 契约，但把它清洗中立。** 前端 154 个文件都基于一组 command 名，所以 macOS 实现可以整套替换而不动 UI。当前契约里泄漏了 Windows 概念——`DriveInfo.letter`、回收站列表/还原语义、`"X - Shortcut.lnk"`、Alt/Shift/Ctrl 拖放语义、`%APPDATA%`——应换成 `Volume/MountPoint`、`Trash`、MIME flavor、`$HOME`，否则抽象层建了也会继续漏。
2. **建 `src-tauri/src/platform/`**：`mod.rs` 定义 trait + `Capabilities` 结构体，`#[cfg]` 分别导出 `windows.rs` / `macos.rs`。`windows` 依赖移入 target 门控后，P0 那 3 处编译阻塞自然消失，且不会在每个新模块里重新长出来。所有路径处理收敛成一个 `FsPath` 新类型 + 特殊目录服务（`app.path()` / `dirs`），禁止字符串拼接 `\`。
3. **先加 `platform_capabilities()` command，再决定实现深度。** 前端按它显隐菜单项、设置开关、健康体检项和快捷键标签；`cfg(not(windows))` 那些返回 Err / no-op 的占位才有统一归处。优先复用跨平台 crate 与规范（`trash`、`file_icon_provider`、`tauri-plugin-opener`、XDG trash spec、`text/uri-list`）。

### 6.2 建议的 PoC（吃掉 P0+P1 约一半工作量）

装 `autostart` / `single-instance` / `os` 三个官方插件 + `typed-path` / `directories` / `sysinfo`，搭 `platform/` 骨架与 capabilities 命令，顺带修掉 3 处编译阻塞，再在 macOS 实机跑 `cargo check` + `npm run build` 验证。这一步能把「静态推断」全部变成「已验证事实」。

⚠️ 事后复盘：6.2 的验收标准被定成「能编译」，导致选型只挑编译必需项；第 5 节查到的大部分现成实现没有进入排期，7.6 的下一步又是从失败用例倒推、天然偏向自己改代码。**执行 P1/P2 前应先过 6.5 的选型门禁，并对照 6.4 的落地矩阵。**

### 6.3 待用户/团队决策的问题

- 支持范围：仅 macOS 直接分发 `.dmg`，还是也上 Mac App Store（沙箱 + 私有 API 限制会显著影响 `decorations:false` 方案）？
- 双架构分版 vs universal binary；最低系统版本（影响 WKWebView 的 `color-mix()` 兼容策略）。
- Windows 专属基线能力（Everything、Peek/QL-Win、Explorer verb、UAC）是找 macOS 等价实现，还是明确不做并在 UI 隐藏？
- 是否引入 `tauri-specta` 之类类型生成层（会影响全部 command 的写法）。
- 是否新增 ADR 0015 记录上述决策。

### 6.4 开源实现落地矩阵（对第 5 节的逐项回收）

第 5 节是「供给清单」，6.2 是「自建骨架」，两者之间缺一道**选型门禁**；7.6 又从失败用例倒推，天然偏向自己改代码。逐项盘点（状态截至 2026-11-14 PoC 结束）：

| 组件（出处） | 覆盖的缺口 | 6.2 / 7.6 是否安排 | 实际状态 |
|---|---|---|---|
| `directories` | `$HOME` / 特殊文件夹 | 6.2 排入 | ✅ 已引入，但只用于 `home_dir()`；`UserDirs` 自带的 `desktop_dir()` / `document_dir()` / `download_dir()` 未用，7.5 的 `navigator::places` 失败仍准备手写 |
| `file_icon_provider` | 系统图标 | 既有资产 | ✅ 非 Windows 分支兜底（`shell_icons/mod.rs`），macOS 能力已置 `true` |
| `trash` | 删除到废纸篓 | 既有资产 | ⚠️ 只用了 `trash::delete`（`file_ops/mod.rs:128`）；列表/还原仍写「重写 PowerShell COM」，未评估 crate 的 `os_limited` 能力 |
| `tauri-plugin-opener` | 打开 / 在文件夹中显示 | 已注册 | ⚠️ 前端 4 处调用；PoC 新增的 `open_with/mod.rs` 非 Windows 分支手写 `Command::new("open"/"xdg-open")`，绕过了已装插件 |
| `patches/drag` AppKit | 原生拖出 | 未列 | ⚠️ 实机编译通过、能力已置 `true`，但未接入验证 |
| 官方 `autostart` / `single-instance` / `os` | 自启动 / 单实例 / OS 判断 | 6.2 排入 → 7.3 收窄 | ⏸ 合理排除，硬约束见 7.3（静态门禁断言、已有自实现锁、能力层已覆盖） |
| `typed-path` / `sysinfo` | FsPath / 卷列表 | 6.2 排入 → 7.3 暂缓 | ❌ 暂缓后没有回到 7.6，P1 接入点未定义 |
| 官方 `dialog` / `fs` / `window-state` / `global-shortcut` | 原生对话框 / FS / 窗口状态 / 快捷键 | 未列 | ❌ 整组缺位；其中 `global-shortcut` 与 P2 快捷键改造直接相关 |
| 社区 5 插件（clipboard / dragout / context-menu / macos-permissions / fs-pro） | 剪贴板互通 / 原生菜单 / TCC 权限 | 未列 | ❌ 整组缺位，而 7.5 的废纸篓权限失败、P2 右键菜单与剪贴板正好对应它们 |
| `notify` / `arboard` / `rfd` / `camino` | 文件监听 / 剪贴板 / 原生对话框 | 未列 | ❌ 整组缺位 |
| `tauri-specta` | IPC 契约类型生成 | 仅 6.3 列为待决策 | ❌ 无排期；若决定不引入应明确记为「不做」，避免长期悬置 |
| VS Code / OpenDAL 架构 | trait + capabilities + 共享测试 | 6.1 采用 | ⚠️ 抄了 trait 与能力协商；**OpenDAL 的共享 behavior test suite 没抄**——它恰好是 7.5 那 17 个「同一套断言跨平台」问题的现成解法 |

**结构性缺口**：`PlatformCapabilities` 的 24 个字段目前是静态 bool（macOS 侧仅 4 项为 `true`），`Platform` trait 只有路径 / 比较 / home / 能力 4 个方法；图标、废纸篓、剪贴板、拖放、搜索仍散落在各自模块的 `#[cfg]` 里。5.6 承诺的「一层 trait 包住现成 crate」尚未排期，所以当前的能力协商只能用于 UI 显隐，还不能用来绑定或替换实现。

> 表中 ⚠️ / ❌ 是复盘当时的状态；其中 3 项代码级浪费已在第二轮（7.7）回收，最新状态以 7.7 为准。第三轮（7.8）把能力层的**前端契约**接上，并用静态断言守住 TS/Rust 字段同步；社区插件与 `sysinfo` 等排期仍按 6.5 执行。

### 6.5 修正后的选型门禁与排期

1. **门禁**：每处理一个 P1/P2 缺口，先在 6.4 矩阵标出 `复用 / 引入 / 自写 / 降级` 四选一，默认禁止直接自写；自写必须写明「哪个库不覆盖」的证据。
2. **P1 优先复用**：`directories::UserDirs` 补齐特殊文件夹 → 消掉 `navigator::places` 失败；`sysinfo::Disks` 替掉 `A:\…Z:\` 卷枚举与手写 `statvfs`；核实 `trash::os_limited` 是否能覆盖废纸篓列表/还原（不能则降级为「只打开 `~/.Trash`」）；7 个 Junction 用例确认无现成 crate 后再用 Unix symlink。
3. **P2 优先引入**：`global-shortcut` 与社区剪贴板 / 右键菜单插件逐个做 Tauri 2 兼容性与许可证复核后再引；`tauri-plugin-macos-permissions` 对应 7.5 的 TCC 权限失败。
4. **能力层升级**：把 `PlatformCapabilities` 从静态 bool 升级为「由已注册适配器回报」，给 trait 增加图标 / 废纸篓 / 搜索等 provider 方法；用 OpenDAL 式共享测试替代按平台复制断言。
5. **契约层闭环**：`tauri-specta` 在 6.3 给出明确「做 / 不做」结论，不要长期悬置。

---

## 7. PoC 落地记录（2026-11-14 晚）

### 7.1 环境与前置修复

- 本机 macOS 15.8.3 / x86_64。rustup 的 `stable` 原本停在 **1.47.0（2020-10）**，无法编译 edition 2021 / Tauri 2；`rustup update stable` 升到 **1.100.0（2026-11-09）**。
- `npm ci` + `npm run build`（`vue-tsc --noEmit` + vite）通过，生成 `dist/`（Tauri `frontendDist` 需要）。
- crates.io sparse index 在本机单连接仅 ~56 KB/s，首次 `cargo check` 卡在索引更新 7 分钟。改用**只对单次命令生效**的镜像配置（`cargo --config 'source.crates-io.replace-with="rsproxy-sparse"' --config 'source.rsproxy-sparse.registry="sparse+https://rsproxy.cn/index/"' ...`），未写 `~/.cargo/config.toml`，也未改任何仓库配置。

### 7.2 实际改动

| 文件 | 改动 |
|---|---|
| `src-tauri/Cargo.toml` | `windows = "0.62"` 移入 `[target.'cfg(windows)'.dependencies]`；新增 `directories = "6"`（platform 层实际使用）。`Cargo.lock` 仅 +10 行 |
| `src-tauri/src/autostart.rs`、`favorites/shell_verb.rs` | 顶层 `use std::os::windows::...` / `OsStr` / `PathBuf` / `to_wide()` / `current_exe()` / Windows 常量加 `#[cfg(windows)]`；`build_run_command` / `command_line` 改为 `#[cfg(any(windows, test))]`。Windows 逻辑一字未改 |
| `src-tauri/src/shell_icons/mod.rs` | 11 行 Win32 imports 合并为单个 `#[cfg(windows)] use windows::{...}`；`ensure_com_sta` 加 no-op stub，`hbitmap_to_rgba` / `system_thumbnail_data_url` 加 cfg + 非 Windows stub（返回 Err，由 `file_icon_provider` 兜底）；`resolve_icon` 改用 `paths::to_native_fs_path` |
| `src-tauri/src/platform/`（新增） | `mod.rs`：`Platform` trait、`PlatformCapabilities`（24 个字段）、`current()` 单例、`to_native_fs_path` / `normalize_for_comparison` / `home_dir` / `has_uri_scheme`、`platform_capabilities()` 与 `platform_home_dir()` 两个 command；`windows.rs` 保留原 Windows 路径语义与全能力；`macos.rs` 为 Unix 语义 + 诚实的能力声明 |
| `src-tauri/src/paths.rs` | 改为委托 platform 层；原 Windows 测试加 `#[cfg(windows)]`，新增 POSIX 用例 |
| `src-tauri/src/navigator/list.rs` | `default_path()` 改 `platform::home_dir()`（P0-4）；`display_path` / `display_paths_equal` 收敛到 `paths` 层；parent 测试平台化 |
| `src-tauri/src/open_with/mod.rs` | 非 Windows 用系统 `open`（macOS）/ `xdg-open`（Linux）打开路径；`open_recycle_bin_system` 打开 `~/.Trash`（P0-5） |
| `src-tauri/src/deps_probe.rs` | 非 Windows 上报「系统 WebView / WKWebView」为 `required + ok`（id `webview`），不再把 WebView2 恒判 missing（P0-7）；测试按平台断言 |
| `src-tauri/src/lib.rs` | `mod platform;` + 注册 `platform::platform_capabilities`、`platform::platform_home_dir` |
| `src-tauri/src/work_groups/{paths,sync}.rs` | 两处写死反斜杠的测试断言改为由 `to_native_fs_path` 生成期望值，并新增 POSIX 去重/大小写用例 |

### 7.3 有意收窄的范围（与 6.2 的建议 PoC 有差异）

| 原计划 | 实际 | 原因 |
|---|---|---|
| `tauri-plugin-autostart` | **未引入** | `scripts/check-autostart.mjs` 有硬断言 `!cargo.includes("tauri-plugin-autostart")`；AIA-60 明确选择自实现 HKCU Run，引入会破坏既有契约 |
| `tauri-plugin-single-instance` | **未引入** | 项目已有 `system_tray::try_acquire_single_instance_lock()`，重复注册会改变启动语义并可能双锁冲突 |
| `tauri-plugin-os` | **未引入** | `platform_capabilities()` 已返回 `os` / `arch` / `primaryModifier`，无需额外 Rust 依赖 + npm 包 |
| `typed-path` / `sysinfo` | **暂缓** | 本次无实际调用点；引入未使用依赖只会拖慢首次编译、放大 lock 变动。留到 P1 接入 FsPath / 卷列表时再引 |
| `directories` | ✅ 引入 | platform 层 `home_dir()` 实际使用 |

结论：PoC 聚焦「**能编译 + 能力协商骨架 + 单一路径入口**」，不追求一口气装插件，避免破坏 Windows 侧既有行为与静态门禁。其中前三项是**永久排除**（有硬约束）；`typed-path` / `sysinfo` 只是**暂缓**，按 6.5 必须回到 P1 排期，PoC 时它们没有出现在 7.6 里属于规划遗漏，已在 6.4 标记。

### 7.4 实机验证结果

| 验证项 | 结果 |
|---|---|
| `cargo check`（默认 `ai-addon`） | ✅ 通过 |
| `cargo check --no-default-features`（Core） | ✅ 通过 |
| `cargo test --lib` | **279 passed / 17 failed / 1 ignored**（详见 7.5） |
| `npm ci` + `npm run build` | ✅ 通过 |
| 12 个受影响的静态契约脚本 | ✅ 全通过（`test:autostart` / `shell-icons` / `favorites-shell-verb` / `path-key` / `arch-optimize` / `deps-surface` / `fil30-skill-open-freeze` / `recycle-bin` / `context-menu` / `open-project-folder` / `navigator-nonblocking` / `no-console-flash`） |
| `test:address-env` | ⚠️ 曾判为「沙箱 SIGTERM(137)」；7.7 复查确认根因是脚本内部派生的 `cargo test` 直连 crates.io 稀疏索引过慢，加临时镜像配置后 ✅ 通过（该 Rust 文件本轮未改） |
| 编译警告 | 14 条，均为既有 Windows 代码在非 Windows 下的 unused import / dead_code，不阻塞 |

**调研结论的实证更新**：第 2 节 P0-1 的「3 处编译阻塞」已由静态推断变为**实机确认并修复**；`tauri-plugin-frame`（含 `create_overlay_titlebar_with_height`）、`patches/drag` 的 AppKit 实现、`trash`、`file_icon_provider` 在 macOS 均编译通过，不需要额外 cfg 化。

⚠️ **能编译 ≠ 能启动、能跑**：尚未执行 `tauri dev` / 打包，窗口交通灯、WKWebView 实际渲染、图标缩略图、跨应用拖放、废纸篓权限等运行时行为未验证。

### 7.5 剩余 17 个失败用例 = P1 功能缺口清单（PoC 时的历史快照；第二轮后为 16 个，见 7.7）

| 类别 | 数量 | 用例（模块） | 对应缺口 |
|---|---|---|---|
| Junction / `mklink /J` | 7 | `projects::lifecycle` ×3、`projects::members` ×2、`projects::legacy_migrate` ×1、`ai::deploy` ×1 | P1：改 Unix symlink，`remove_dir` → `remove_file` |
| `%USERPROFILE%` / APPDATA 路径夹具 | 5 | `ai::agents`、`ai::skills` ×2、`ai::mcp`、`projects::index` | P1：`$HOME` / `app_data_dir`，测试夹具平台化 |
| 卷根判定与错误文案 | 3 | `file_ops::safe_io::refuse_volume_roots`、`file_ops::permanent_delete…`、`file_ops::rejects_awam_paths` | P1：`/Volumes`、`/` 根判定与平台化文案 |
| 废纸篓权限 | 1 | `file_ops::delete_to_recycle_local_file` | 本机 Finder AppleScript 权限违例（-10004）；需 TCC 授权或换实现 |
| 系统特殊文件夹 | 1 | `navigator::places::shell_places_libraries_come_from_user_folders` | P1：macOS 特殊目录映射 |

这些不是编译问题，也不是本次改动引入的回归：改动前同批测试在 macOS 也会因同样的 Windows 假设失败；本次把其中 4 个纯路径格式断言平台化后，失败数从 21 降到 17。

### 7.6 下一步（按性价比，已按 6.5 选型门禁修订）

0. **先过门禁**：把 7.5 的 17 个失败按 6.4 矩阵逐项标记 `复用 / 引入 / 自写 / 降级`，再动手；只有 Junction → symlink 这类确实无库可代劳的才自写。
1. 前端接入 `platform_capabilities()`：显隐 Windows 专属菜单/设置项、快捷键标签 Ctrl→⌘（复用 `src/shortcuts/`）；顺带把 `open_with` 非 Windows 分支换成已注册的 `tauri-plugin-opener`，消掉手写 `open` / `xdg-open`。**（`open_with` 部分 7.7 已完成；前端能力层与快捷键部分 7.8 已完成）**
2. `tauri.conf.json`：`targets` 加 `app`/`dmg`、`bundle.macOS`、`bundle.resources`、`titleBarStyle: Overlay`（P0-6）。**（bundle 三项 7.9 已完成；`titleBarStyle` 需 GUI 实机验证后再动，理由见 7.9）**
3. 消失败用例时优先复用现成实现：`directories::UserDirs` 处理 5 个特殊文件夹 / `%USERPROFILE%` 夹具、`sysinfo::Disks` 处理 3 个卷根判定、核实 `trash::os_limited` 后决定废纸篓权限用例走修复还是降级；7 个 Junction 用例确认无 crate 覆盖后用 Unix symlink。
4. `tauri dev` 实机验证窗口/图标/路径/打开方式，再谈 CI 双平台矩阵与签名公证（P3）。
5. 按 6.5 第 3–5 条推进 P2 插件复核、能力层从静态 bool 升级为适配器回报、以及 `tauri-specta` 的做/不做决策。

### 7.7 第二轮：回收已查到的开源实现（提交 `405c3be` 之后）

6.4 复盘指出的「已装却手写绕过」与「未核实的降级」逐项落地：

| 6.4 的问题 | 本轮处理 | 结果 |
|---|---|---|
| `tauri-plugin-opener` 被手写 `open` / `xdg-open` 绕过 | `open_with/mod.rs` 两个非 Windows 分支改用 `tauri_plugin_opener::open_path(path, None::<&str>)`（2.7.2 提供的自由函数，无需 `AppHandle`；内部走 `open::that_detached` 并校验路径存在） | ✅ 删掉全部手写 `Command::new` 与 `cfg!(target_os)`；Windows 的 ShellExecute / explorer 与 FIL-30 契约未动 |
| `directories` 只用了 `home_dir()` | `navigator/places.rs` 非 Windows 分支改用 `UserDirs` 的 `desktop_dir` / `document_dir` / `download_dir` / `picture_dir` / `audio_dir` / `video_dir`；新增 `paths::display_path` 作为统一显示路径入口（由 platform 层决定分隔符） | ✅ 消掉 7.5 的 `navigator::places::shell_places_libraries_come_from_user_folders` 失败 |
| `trash::os_limited` 未核实 | 查 `trash-5.2.9/src/lib.rs:346-349`：`os_limited` 的 cfg 是 `windows` 或 `unix && !macos`，**macOS 被明确排除**，list / restore / purge 无现成实现 | ✅ 结论是降级而非接入：`file_ops/recycle.rs` 两个 pub fn 在非 Windows 返回明确错误、PowerShell 实现整体 `#[cfg(windows)]` 化；`platform/macos.rs` 显式 `trash_list_restore = false` 并加注释与单测断言 |

顺带清理同一模块的两处无意义 OS 调用：`list_network_places()` 非 Windows 直接返回空（原先会 spawn `cmd /C net use` 再失败）；`list_drives()` 非 Windows 直接返回空（原先 26 次 `A:\`…`Z:\` stat 永远为空），并移除两个不再被非 Windows 引用的 stub。真实卷列表 / 磁盘空间仍按 6.5 留给 P1 的 `sysinfo::Disks`；**本轮未引入新依赖**，`Cargo.lock` 不变。

**实机验证**（macOS 15.8.3 / rustc 1.100.0）：

| 项目 | 结果 |
|---|---|
| `cargo check`（默认 `ai-addon`） | ✅ 通过；14 条警告与改动前一致，均为既有 Windows 代码在非 Windows 下的 unused / dead_code |
| `cargo check --no-default-features`（Core） | ✅ 通过 |
| `cargo test --lib` | **280 passed / 16 failed / 1 ignored**（上一轮 279 / 17）；新增通过的是 `shell_places_libraries_come_from_user_folders`，剩余 16 个仍是 7.5 的 P1 缺口，无回归 |
| 静态契约脚本 | ✅ **19 个全通过**（12 个常规 + 5 个导航/发布契约 + `test:shell-places` / `test:address-env` 两个会派生 `cargo test` 的脚本）；后两者需临时镜像配置，见下方说明 |

**验证环境的本机事实**：`check-shell-places.mjs` / `check-address-env.mjs` 内部 `spawnSync("cargo", ...)` 不会继承命令行的 `--config`，会直连 crates.io 稀疏索引（本机 ~56 KB/s）而长时间挂起；在项目根放一份临时 `.cargo/config.toml` 指向镜像即可跑通，验证后配置已删除。`CARGO_NET_OFFLINE=true` 不可用——它会让 `[patch.crates-io]` 的 `drag` 依赖解析失败；`CARGO_SOURCE_*` 环境变量对 `[source]` 表也不生效。

⚠️ 本轮执行时仍未启动 `tauri dev` / 打包：`open_path` 的实际 GUI 行为、`~/.Trash` 不存在时的错误提示、WKWebView 渲染与拖放都未做运行时验证。本轮改动已提交为 `a76dd4c`，与 `405c3be` 分开。

### 7.8 第三轮：前端接入能力层（`mac` 分支，7.6 第 1 步）

分支整理后（本地领先的 2 个提交迁到 `mac`，`master` 回到 `origin/master`），按 6.5 门禁处理 7.6 第 1 步。**选型结论：这一层无库可代劳**——Tauri 官方插件只覆盖能力级（窗口 / 对话框 / OS 信息），跨平台浏览器的「中立契约 + 能力协商」在 Tauri 应用里必须由应用自建；因此复用的是第 4 节的架构思想与项目已有的 IPC command，不引入新依赖。

| 文件 | 改动 |
|---|---|
| `src/config/platform.ts`（新增） | 定义与 Rust `PlatformCapabilities` 一一对应的 TS 类型；`NEUTRAL_PLATFORM`（`os: ""` 作为「未知」哨兵）；`hydratePlatform()` 单次 IPC 且永不 reject；`isMacPlatform()`（caps 权威 + UA 冷启动兜底）；`platformShortcutLabel()` / `platformAriaKeys()`。**不静态 import `@tauri-apps/api` 或 Vue**，否则 `scripts/shortcut-checks.ts` 在 Node 里会加载浏览器模块 |
| `src/shortcuts/chord.ts` | 注册表继续 Windows-first（`ctrl: true`），`eventMatchesChord` 在 macOS 交换 `ctrl`/`meta`；`formatChordLabel` 显示 ⌘/⌥/⇧。**`chordKey` 刻意保持平台无关**，因为 `resolveDispatchMap`、冲突检测、`chordKeyForId` 都以它为键 |
| `src/main.ts` | bootstrap 起始处 `void hydratePlatform()` 与 settings 水合并行预热，不增加串行 IPC |
| `src/views/SettingsView.vue` | `platformCaps` 为响应式 ref，`onMounted` 再取一次保证冷启动后的行显隐会重算；`visibleDefs` 过滤平台不支持的设置；收藏夹 OS verb 开关按 `osShellVerb` 隐藏 |
| `src/state/settingsUi.ts` | 新增纯函数 `isPlatformUnsupportedSetting(key, caps)`：`launchAtStartup` 系列依据 `nativeAutostart/registryAutostart`、`everythingPreferred` 依据 `globalSearch`；`os === ""` 或 `windows` 时一律不隐藏，schema/document 键不删 |
| `AppShell.vue` / `DiscoveryView.vue` / `CommandPanel.vue` | 命令按钮 title、命令面板 hint、`aria-keyshortcuts` 走运行时文案转换（⌘ / Meta），**5 语言 locale 文件不改**，避免重复翻译与遗漏 |

**契约闭环**：`scripts/shortcut-checks.ts` 新增两组断言——① 模拟 WKWebView UA 后验证 `Cmd+C` 命中 `ctrl`-注册的和弦、物理 Ctrl 不命中、`Cmd+Shift+P` 仍开命令面板、`chordKey` 不变；② 解析 Rust `pub struct PlatformCapabilities` 的字段并与 `Object.keys(NEUTRAL_PLATFORM)` 深度比对，防止后端加字段前端漏同步。这是 6.5 第 5 条「`tauri-specta` 做/不做」的轻量替代：**当前决定不引入 `tauri-specta`**，用静态契约守住这唯一一处手写类型，等 command 数量增长或出现第二个手写契约时再评估。

**实机验证**（macOS 15.8.3 / rustc 1.100.0）：

| 项目 | 结果 |
|---|---|
| `npm run build`（`vue-tsc --noEmit` + `vite build`） | ✅ 通过，无类型错误；仅既有的 chunk 大小 / 动态导入提示 |
| `npm run test:shortcuts` | ✅ 通过（58 builtins + macOS 主修饰键 + 能力字段同步） |
| 相关静态契约脚本 | ✅ 14 个全通过（含 `test:tokens`、`test:command-panel`、`test:settings-ui-regroup`、`test:i18n`、`test:startup-async`、`test:window-controls` 等） |
| `cargo check` / `cargo test --lib` | ⏸ 本轮 **Rust 零改动**，沿用 7.7 的 280 passed / 16 failed 基准；执行时本机另一个 `tauri dev` 持有 cargo 包缓存锁，未重复跑以免干扰 |

**尚未覆盖**（仍属 P2，别误判为已完成）：`Delete`/`Backspace` 语义、`Cmd+Backspace` 移废纸篓、`Cmd+I` 属性、`Option` 与 `Alt` 的冲突、F 键被媒体键占用、i18n 中「此电脑 / 回收站 / 资源管理器」等 Windows 名词、交通灯标题栏与原生菜单栏；`platform_capabilities()` 的 24 字段仍是静态 bool，尚未升级为「由适配器回报」。另外本轮只做了**代码与构建验证，未启动 GUI**，上述 UI 行为仍是推断。

### 7.9 第四轮：macOS 打包配置与静态契约（`mac` 分支，7.6 第 2 步）

**门禁结论**（6.5 第 1 条）：P0-6 属「配置声明」缺口，没有第三方库可代劳；可复用的是 Tauri CLI 自带的 `node_modules/@tauri-apps/cli/config.schema.json`，用它核实字段而不是凭记忆写。已确认存在 `BundleConfig.targets / resources / macOS`、`MacConfig.minimumSystemVersion / dmg`、`WindowConfig.titleBarStyle / hiddenTitle / trafficLightPosition`。本轮**不引入新依赖**。

| 文件 | 改动 |
|---|---|
| `src-tauri/tauri.conf.json` | `bundle.targets: ["nsis"] → ["nsis","app","dmg"]`（nsis 保留，Windows 发布流程不受影响）；新增 `bundle.resources: ["icons/32x32.png"]`，使 `src/state/dnd/dragOut.ts:21` 的 `resolveResource("icons/32x32.png")` 在打包后不再缺拖影；新增 `bundle.macOS.minimumSystemVersion: "10.15"`（Tauri 默认 10.13，显式钉到 Tauri 2 / WKWebView 的实际下限）。`bundle.windows.webviewInstallMode` 与 `app.windows` 一字未改 |
| `scripts/check-macos-bundle.mjs`（新增） | 静态契约：三个 target 齐备；资源路径与 `resolveResource` 实参一致且文件存在；`icon.icns` 存在；`minimumSystemVersion` 不被回退；Windows 的 `embedBootstrapper`、`decorations:false` 不被破坏；发布脚本仍显式 `--bundles nsis` |
| `package.json` | 注册 `test:macos-bundle` |

**刻意推迟的一项**：7.6 第 2 步里的 `titleBarStyle: Overlay` / `hiddenTitle`。当前窗口是 `decorations: false` + 前端自绘 Fluent 标题栏（3 个静态脚本对该字段有硬断言），macOS 交通灯是否显示、落点是否遮挡前端控件都必须实机看窗口才能定；纯配置改动不该代替 GUI 验证。该字段仅 macOS 生效，留给 7.6 第 4 步一起处理。签名公证（`signingIdentity`）、CI `macos-latest` matrix、`rust-toolchain.toml` 的 apple targets 仍属 P3。

**验证**（macOS 15.8.3 / rustc 1.100.0）：

| 项目 | 结果 |
|---|---|
| `cargo check`（`tauri-build` 会解析 config，等价于 schema 合法性校验） | ✅ 通过，2.97s，14 条警告与 7.7 一致，均为既有 Windows 代码的 unused / dead_code |
| `npm run build`（`vue-tsc --noEmit` + vite） | ✅ 通过，17.32s，仅既有 chunk 大小提示 |
| `test:macos-bundle` + 8 个读取 `tauri.conf.json` 的既有静态脚本 | ✅ 全通过：`deps-surface` / `startup-window-flash` / `window-controls` / `dual-pane` / `file-drag-out` / `packaging-flavors` / `multi-pack-release` / `shortcuts` |
| `npx tauri build --debug --bundles app` | ✅ exit 0，产出 53.99 MiB 的 `A File Manager.app` |

**实机打包验证**（把 P0-6 从「配置写了」推进到「包里确实有」）：

- `.app/Contents/Info.plist`：`LSMinimumSystemVersion = 10.15`、`CFBundleIdentifier = com.a-file-manager.app`、`CFBundleIconFile = icon.icns`；`Contents/MacOS/a-file-manager` 为 54 MiB 可执行文件。
- `Contents/Resources/icons/32x32.png`（1.1 KiB）确实随包分发 → `resolveResource` 的拖影图标在 release 包里不再缺失，这是 `bundle.resources` 的直接验收。
- `npx tauri build --debug --bundles dmg` 的编译与 `.app` 阶段都成功，最后 `bundle_dmg.sh` 调 `hdiutil create` 报 `操作不被允许`；日志末尾的沙箱提示显示被拒的是 `/Volumes/A File Manager/...` 挂载卷里的 file-delete。这是**本会话沙箱对 dmg 挂载卷的限制**，不是 Tauri 配置错误；在普通终端重跑该命令应能产出 `.dmg`。签名/公证仍属 P3，未配置 `signingIdentity`，当前为 ad-hoc 签名。
- 仍未启动 GUI：WKWebView 渲染、交通灯位置、拖放、TCC 弹窗都未实机看窗口，7.6 第 4 步的 `tauri dev` 验证依旧待办。

---

## 附录 A · 关键证据索引（便于复核）

| 主题 | 文件 |
|---|---|
| 编译阻塞 | `src-tauri/src/autostart.rs:6,21-26`；`src-tauri/src/favorites/shell_verb.rs:7,21-26`；`src-tauri/src/shell_icons/mod.rs:18-28,124-227` |
| 依赖 / 配置 | `src-tauri/Cargo.toml:33-50,52-78`；`src-tauri/tauri.conf.json:14-51`；`rust-toolchain.toml:1-4`；`src-tauri/capabilities/default.json` |
| 路径 | `src-tauri/src/paths.rs:5-55`；`navigator/list.rs:127-155`；`navigator/stat.rs:92`；`src/state/pathKey.ts`；`src/state/navHistory.ts`；`src/state/fileClipboard.ts:108`；`src/state/dnd/paths.ts` |
| 前端快捷键 | `src/shortcuts/types.ts:3-11`、`chord.ts:6-60`、`registry.ts:10-283`、`runtime.ts:12-54`、`src/components/shell/AppShell.vue:717-747` |
| 标题栏 | `index.html:10-22`；`AppShell.vue:1768-1854,2648-2733`；`src/styles/fluent-tokens.css:19-20,27-49`；`src/state/revealMainWindow.ts`；`closeBehavior.ts` |
| 构建发布 | `.github/workflows/ci.yml`；`.github/workflows/release.yml`；`scripts/release-assets.mjs`；`scripts/build-flavor.mjs`；`scripts/build-release-artifacts.mjs`；`scripts/check-*.mjs`；`.cursor/skills/afm-multi-pack-release/SKILL.md:86` |
| 可复用资产 | `src-tauri/patches/drag/Cargo.toml:61-131`；`patches/drag/src/platform_impl/macos/mod.rs`；`src-tauri/src/extension_host/pack_fs.rs:61-159`；`src/state/shellIcons.ts:34-348` |
| PoC 平台层（新增） | `src-tauri/src/platform/{mod,windows,macos}.rs`；`src-tauri/Cargo.toml` 的 `[target.'cfg(windows)'.dependencies]`；`src-tauri/src/lib.rs` 的 `mod platform;` 与两个 command |
| 前端能力层（7.8） | `src/config/platform.ts`；`src/shortcuts/chord.ts`；`src/main.ts`；`src/views/SettingsView.vue`；`src/state/settingsUi.ts`；`scripts/shortcut-checks.ts` |

## 附录 B · 参考资源

以下为本次检索命中的入口（外部资料，仅供复核；社区项目需自行确认维护状态与许可证）。

- Chromium 平台适配：`https://source.chromium.org/chromium/chromium/src/+/main:/ui/base/clipboard/BUILD.gn`、`https://www.chromium.org/Home/`
- Firefox XPCOM 契约：`nsIFile.idl` / `nsIClipboard.idl` / `nsIDragService.idl` / `widget/cocoa/nsWidgetFactory.mm`（可在 `searchfox.org` 查阅源码；MDN/UDN 有接口文档镜像）
- Tauri 官方插件目录：`https://v2.tauri.app/plugin/`；社区清单 `https://github.com/tauri-apps/awesome-tauri`
- VS Code：`https://www.deepwiki.com/microsoft/vscode/2-electron-main-process`、`https://github.com/microsoft/vscode`
- Apache OpenDAL：`https://opendal.apache.org/`
- Spacedrive 架构：`https://v2.spacedrive.com/core/architecture`
- Rust crate 文档：`https://docs.rs/sysinfo`、`typed-path` / `notify` / `trash` / `file_icon_provider` / `tauri-specta` 均在 crates.io / docs.rs

## 附录 C · 方法与限制

- 调研阶段为三路并行只读排查（Rust 后端 / 前端 / 构建发布），逐文件核对 `cfg`、`windows::`、路径处理、快捷键、CI 与发版脚本；PoC 阶段按第 7 节修改了源码。
- 第二轮（跨平台内核机制）与第三轮（开源供给）通过公开资料检索完成，**插件与 crate 的版本/维护状态需在实际引入前重新核实**。
- 第 2 节 P0 的编译阻塞判断最初是静态推断，现已由 macOS 实机 `cargo check`（两种 feature）+ `cargo test --lib` + `npm run build` 验证，详见 7.4；**仍未执行 `tauri dev` / 打包 / 签名**，因此运行时行为（窗口、WKWebView、图标、拖放、权限）仍是推断。
- 工作量是粗估，不含证书申请、Apple 审核/公证排队与前端大规模视觉重构；第 7 节 PoC 实际耗时约 1 小时（含工具链升级与依赖下载），7.7 的第二轮复用改造约半小时。
- 第二轮（7.7）修正了一条环境结论：`test:address-env` 的挂起不是沙箱限制，而是脚本内部 `cargo test` 的网络路径问题（详见 7.7）；该脚本现已通过。
- 第三轮（7.8）只改前端，Rust 侧零改动，因此未重复跑 `cargo check` / `cargo test --lib`；执行验证时本机已有另一个 `tauri dev` 持有 cargo 包缓存锁（1420 端口有 vite），为不干扰该进程而跳过。前端的 ⌘ 行为由 Node 静态断言模拟 UA 验证，**真实 WKWebView 的按键、窗口与菜单行为仍未实机确认**。
