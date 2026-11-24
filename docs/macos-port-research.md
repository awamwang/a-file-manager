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
| P0-8 | CI 加 `macos-latest` matrix、去掉 WebView2/NSIS 断言、release 产 `.dmg`/`.app`；`rust-toolchain.toml` 补两个 apple targets | ci.yml / release.yml | 【运行】（7.25 已做双平台矩阵 + Mac 打包 job；WebView2/NSIS 断言改为**仅 Windows 执行**而非删除；apple targets 由 workflow 的 `dtolnay/rust-toolchain` 按矩阵注入，未写进 `rust-toolchain.toml` 以免所有开发者被迫下载） |

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
- **右键 / 属性 / 打开方式**：`TrackPopupMenu` → `NSMenu`/Services 或降级为自绘菜单；`open_native_open_with_dialog` 无等价 API → `open -a` 或降级为默认程序打开；ShellNew 模板、`- Copy` 命名、UAC 提权项、非法字符表（`file_ops/mod.rs:62-73` 的 Windows 保留字符）统一平台化。**`.lnk`：复制为链接（TR-203）已用 Unix symlink + `.link` 后缀降级完成（2026-11-24），link 预览走 `read_link`。**
- **空格预览**：`preview_bridge/mod.rs` 的「QuickLook」其实是 Windows 移植版 QL-Win 命名管道，Peek 是 PowerToys；macOS 用 `qlmanage -p` / `QLPreviewPanel`；好消息是 `auto` 模式已能优雅降级到内置预览面板（`:426-456`）。
- **剪贴板 / 自启动 / 单实例**：CF_HDROP → NSPasteboard；HKCU Run → `SMAppService` / LaunchAgent / `tauri-plugin-autostart`；`CreateMutexW` → `tauri-plugin-single-instance`。建议先降级禁用并在设置页隐藏。
- **CSS / 字体 / i18n**：Segoe UI / 微软雅黑 / Consolas 全缺 → SF / PingFang / Menlo；约 50 处 `color-mix()` 在旧版 WKWebView 整条声明失效；5 语言写死 `Ctrl+Shift+P`、`F5`、`Alt+Enter`、`此电脑`、`回收站`、`资源管理器`、`%APPDATA%`、`C:\Windows`、`\\server\share`、`WebView2 Runtime`。（**7.22 已让 24 组主文案按宿主切换；`F5` 因 chord 未变有意保留，其余示例路径 / 后端硬编码中文 / 字体仍待办**）
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
| 11 Junction / 快捷方式 / 剪贴板 | `projects/members.rs:169-206`、`ai/deploy.rs:51-89`、`shortcut.rs:10-65`、`clipboard.rs:121-284` | `mklink /J`；`"X - Shortcut.lnk"`；CF_HDROP；ShellNew | Unix symlink + `remove_file`；复制为链接已降级为 symlink + `.link` 后缀（`shortcut.rs` 非 Windows 分支 + `read_link` 预览）；NSPasteboard | 【运行】/【降级】·复制为链接 ✅ |
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
| 9 | 文案全在说 Windows | 5 语言写死 `Ctrl+Shift/P`、`F5`、`Alt+Enter`、`此电脑`、`回收站`、`资源管理器`、`%APPDATA%`、`C:\Windows`、`\\server\share` | `src/i18n/locales/{en,zh-CN,ja,de,fr}.json`：`commandsShortcut`、`chrome.refreshTitle`、`properties.multiHint`、`guide.rootLabel`/`drive*`、`recycleBin.*`、`libraryRootHint`、`blacklistPlaceholder`、`installPathPlaceholder`、`networkEmpty`、`health.*.webview2`、`osVerb` 等约 20 组字段（**7.22 已按宿主覆盖其中 24 组主文案，Windows 正文零改动；示例路径与后端硬编码中文仍残留**） |
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

1. `rust-toolchain.toml` 加 `targets = ["aarch64-apple-darwin","x86_64-apple-darwin"]`；文档补 Xcode CLT 前置。**（7.25 改为 workflow 按 job 注入 target，理由见 P0-8；`rust-toolchain.toml` 不动。）**
2. `Cargo.toml` 把 `windows` 移入 target 门控；修 `autostart.rs:6` / `favorites/shell_verb.rs:7` / `shell_icons/mod.rs:18-28` 三处阻塞。
3. macOS 实机跑 `cargo check --target aarch64-apple-darwin`（Core `--no-default-features`、AI 默认）+ `npm run build`。
4. `check-packaging-flavors.mjs` 的 cargo 命令支持 `--target`；`check-deps-surface.mjs` 的断言改为仅 Windows 生效。
5. `tauri.conf.json` 加 `app`/`dmg` + 完整 `bundle.macOS`；新增 `Info.plist` / `*.entitlements` / dmg 布局；声明最低系统版本。
6. `build-flavor.mjs` 按平台注入 `app,dmg` 与 `--target`；`build-release-artifacts.mjs` 按平台选 bundle 子目录。
7. 本地验证产出 `A File Manager.app` / `.dmg`；确认 macOS bsdtar 能生成 Feature Pack zip。
8. 申请 Developer ID 证书 + App Store Connect / Apple ID；CI 注入 `APPLE_*` secrets。
9. 配 `signingIdentity` + `hardenedRuntime` + entitlements；CI 加 `codesign` → `notarytool submit --wait` → `stapler staple`。
10. 更新 SKILL.md:86 的「当前无代码签名证书」与 README Requirements。
11. `ci.yml` 拆 `windows-latest` + `macos-latest` matrix；PowerShell 步骤改 Node 脚本 / bash；缓存键加 OS/target。**（7.25 已完成。）**
12. `release.yml` 的 `flavor_matrix` 扩为 `{flavor, vite_ai, os, runner, target, bundles, asset_kind}`；新增 `build-app-macos` job；移除 macOS 侧 WebView2 校验。**（7.25 已完成主体：新增独立的 `mac_matrix` 输出与 `build-app-macos` job，WebView2 校验条件化到 Windows；未额外扩 `flavor_matrix` 字段，避免碰 Windows 既有矩阵。）**
13. 同步更新 `check-multi-pack-release.mjs` / `check-autostart.mjs` / `check-arch-optimize.mjs` 的契约断言。
14. `release-assets.mjs` 命名中枢加 dmg/app 资产名函数（含 arch 维度）；`prepare-release.mjs` / dry-run 输出随之更新。**（7.25 已加 `tauriDmgFileName` / `dmgAssetName` / `appZipAssetName` / `macReleaseMatrix`；`prepare-release.mjs` 未改。）**
15. `release.yml:187-195` release notes 按平台列资产；复用现有 `gh release upload --clobber` 机制。**（7.25 已完成。）**
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

**结构性缺口**（复盘当时）：`PlatformCapabilities` 的 24 个字段是静态 bool（macOS 侧仅 4 项为 `true`），`Platform` trait 只有路径 / 比较 / home / 能力 4 个方法；图标、废纸篓、剪贴板、拖放、搜索仍散落在各自模块的 `#[cfg]` 里。5.6 承诺的「一层 trait 包住现成 crate」尚未排期，所以能力协商只能用于 UI 显隐，不能用来绑定或替换实现。

> **7.16 部分回收**：卷列表 / 磁盘空间 / 隐藏文件 / 窗口 chrome 四项已收进 `Platform` trait（12 个方法、26 个字段），前端真实路径收敛到 `src/state/fsPath.ts`，并补了上一条「架构归位门禁」。图标、废纸篓、剪贴板、搜索仍在各自模块的 `#[cfg]` 里；`PlatformCapabilities` 仍是静态 bool，6.5 第 4 条的「由实现动态回报」与 5.6 第 1 条的 `FsPath` 新类型仍属未做，别误判为已完成。
>
> **7.20 再回收一项**：P1 第一行「终端 / IDE」的发现与启动也归位适配层（trait 12 → 17 个方法），macOS 用 `.app` + `/usr/bin/open -a`，Windows 侧逐字搬迁。仍未归位的是图标 / 废纸篓 / 剪贴板 / 搜索四个 provider，以及静态 bool 能力层。
>
> **7.21 回收「搜索」**：全局索引后端（Spotlight / Everything）归位适配层（trait 17 → 20 个方法），并新增能力字段 `global_search_engine` 让 UI 按真实引擎出文案——这是 6.5 第 4 条「能力层绑定实现」的第一小步，但字段值仍是静态枚举、**不是**由已注册 provider 动态回报。仍未归位：图标 / 废纸篓 / 剪贴板三个 provider，以及静态 bool 能力层本身。
>
> **7.23 回收「图标 provider + 进程树」**：trait 20 → 24 个方法，新增中立 `RgbaIcon`；Windows COM / `IShellItemImageFactory` 与 macOS `file_icon_provider` 各自落在适配器，`shell_icons` 只剩中立编码与缓存；进程树终止与 sidecar 解释器也归位适配层（`sysinfo` 的 `system` feature；Windows 保留 `taskkill /T`）。**仍未归位：废纸篓 / 剪贴板两个 provider，以及静态 bool 能力层本身**；macOS 内容缩略图按 7.23 的实测结果显式降级，不是已实现。**（7.24 已回收：两个 provider 收进 trait，四个 provider-backed 能力字段改为从注册关系推导，并用同一份共享测试覆盖两端。）**

> 表中 ⚠️ / ❌ 是复盘当时的状态；其中 3 项代码级浪费已在第二轮（7.7）回收，最新状态以 7.7 为准。第三轮（7.8）把能力层的**前端契约**接上，并用静态断言守住 TS/Rust 字段同步；社区插件与 `sysinfo` 等排期仍按 6.5 执行。

### 6.5 修正后的选型门禁与排期

1. **门禁**：每处理一个 P1/P2 缺口，先在 6.4 矩阵标出 `复用 / 引入 / 自写 / 降级` 四选一，默认禁止直接自写；自写必须写明「哪个库不覆盖」的证据。
2. **P1 优先复用**：`directories::UserDirs` 补齐特殊文件夹 → 消掉 `navigator::places` 失败；`sysinfo::Disks` 替掉 `A:\…Z:\` 卷枚举与手写 `statvfs`；核实 `trash::os_limited` 是否能覆盖废纸篓列表/还原（不能则降级为「只打开 `~/.Trash`」）；7 个 Junction 用例确认无现成 crate 后再用 Unix symlink。
3. **P2 优先引入**：`global-shortcut` 与社区剪贴板 / 右键菜单插件逐个做 Tauri 2 兼容性与许可证复核后再引；`tauri-plugin-macos-permissions` 对应 7.5 的 TCC 权限失败。
4. **能力层升级**：把 `PlatformCapabilities` 从静态 bool 升级为「由已注册适配器回报」，给 trait 增加图标 / 废纸篓 / 搜索等 provider 方法；用 OpenDAL 式共享测试替代按平台复制断言。
5. **契约层闭环**：`tauri-specta` 在 6.3 给出明确「做 / 不做」结论，不要长期悬置。
6. **架构归位门禁（7.16 补）**：每新增一处 `#[cfg(windows)]` / `#[cfg(not(windows))]` / `isMacPlatform()` 分支，必须先回答「为什么不能放进 `platform/` 适配层或 `fsPath.ts`」。选型门禁（第 1 条）只管「用什么实现」，不管「放在哪里」——7~10 轮全部过了选型门禁却持续长出旁路，就是因为缺这一条。纯 UI 展现差异（如自绘栏留白）与整模块 Windows-only 集成（如 `autostart.rs`）可豁免，但须在提交说明里写明理由。

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
| 编译警告 | ✅ 已清零（14 → 0）；原 14 条均为 Windows-only 代码在 macOS 下的 unused import / dead_code，第 6 轮用 `#[cfg(windows)]` / `#[allow(dead_code)]` 收口（见 7.11） |

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
3. 消失败用例时优先复用现成实现：`directories::UserDirs` 处理 5 个特殊文件夹 / `%USERPROFILE%` 夹具、`sysinfo::Disks` 处理 3 个卷根判定、核实 `trash::os_limited` 后决定废纸篓权限用例走修复还是降级；7 个 Junction 用例确认无 crate 覆盖后用 Unix symlink。**（7.10 已完成，16 个失败归零；门禁复核后 `sysinfo::Disks` 不适用于「给定路径是否为卷根」的判定，改由 platform 分支自写，Disks 仍留给 `list_drives` / `disk_space` 的真实缺口）**
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

### 7.10 第五轮：消掉 16 个 P1 失败用例（`mac` 分支，7.6 第 3 步）

复现基线 `280 passed / 16 failed / 1 ignored`。按 6.5 门禁逐组标记 `复用 / 引入 / 自写 / 降级`，**未引入任何新依赖**，`Cargo.lock` 不变。

| 组 | 数量 | 门禁决策 | 证据 |
|---|---|---|---|
| 路径归一化 / `%USERPROFILE%` 夹具 | 5 | **复用** | `crate::paths::to_native_fs_path`、`platform::home_dir()`、`directories::BaseDirs` 都已在依赖里；根因是三个模块各自复制了一份 `replace('/', "\\")`，把 `/var/folders/...` 变成相对路径 `\var\folders\...` |
| Junction `mklink /J` | 7 | **自写（标准库）** | 无 crate 覆盖「创建 / 删除目录链接」；`std::os::unix::fs::symlink` + `remove_file` 即可，Windows 分支的 `cmd /C mklink /J` 一字未改 |
| 卷根判定 | 3 | **自写** | `sysinfo::Disks` 只列已挂载卷与剩余空间，**不判定给定路径是否为卷根**；它属于 `list_drives` / `disk_space` 的缺口，不该为一条安全护栏引入重依赖。理由已写进代码注释 |
| 废纸篓 TCC(-10004) | 1 | **降级** | `trash 5.2` 在 macOS 走 Finder AppleScript，需要 TCC Automation 授权；7.7 已核实 `os_limited` 排除 macOS，授权不是代码能代劳的 |

**改动**（9 个 Rust 文件，+182/−39）：

- 路径入口：`paths.rs` 新增 `join_relative(root, rel)`；`ai/{deploy,skills,mcp}.rs` 三份私有归一化副本委托 `crate::paths`；`projects/index.rs` 的 `projects_root` 比较改用 `normalize_path_for_comparison`（这个隐藏 bug 会让每次 load 都判定 root 变化、清掉 `active_project`）；`file_ops/safe_io.rs` 的 `display_path` 也收敛到 `crate::paths`。
- `ai/agents.rs::expand_agent_path`：Windows 仍优先读 env var；macOS 用 `platform::home_dir()` + `directories::BaseDirs` 兜底 `%USERPROFILE%` / `%APPDATA%` / `%LOCALAPPDATA%`，`~` 用平台分隔符拼接，末尾统一 `to_native_fs_path`。
- Junction：`projects/members.rs` 与 `ai/deploy.rs` 的 create / remove 拆 `#[cfg(windows)]`（`mklink /J` + `remove_dir`）/ `#[cfg(not(windows))]`（symlink + `remove_file`）。静态契约要求的 `mklink` 字符串保留在 Windows 分支里。
- 卷根：`safe_io::is_volume_root` 按平台分支 —— Windows 保持原 `Prefix`/`RootDir` 判定，Unix 认 `/` 与 `/Volumes/<name>`；`refuse_volume_root` 的错误文案不变，因此 `file_ops/mod.rs` 的字符串替换键无需同步。3 个测试改用当前平台的真实卷根 / 目标目录，不再写死 `C:\`。
- 废纸篓：`delete_to_recycle_local_file` 在 macOS 只接受 `Failed to move to Recycle Bin` 的**明确失败**（TCC 未授权），绝不接受「静默成功但文件还在」；Windows 仍断言真删成功，`trash::delete` 调用保留。

**验证**：

| 项目 | 结果 |
|---|---|
| `cargo test --lib` | ✅ **296 passed / 0 failed / 1 ignored**（原 280 / 16 / 1） |
| `cargo check --no-default-features`（Core） | ✅ 通过，警告数维持 14 条基线（`join_relative` 在 Core 下用 `cfg_attr(not(feature="ai-addon"), allow(dead_code))` 豁免） |
| `npm run build` | ✅ 通过（vue-tsc + vite，18.46s，仅既有 chunk 提示） |
| 16 个相关静态契约脚本（含 5 个会派生 `cargo test` 的） | ✅ 全通过：`ai-addon-shell` / `fil30-skill-open-freeze` / `file-ops` / `file-undo` / `deps-surface` / `status-extras` / `address-env` / `shell-places` / `path-key` / `wg-promote-jump` / `wg-project-copy` / `projects-ops-cleanup` / `navigator-nonblocking` / `autostart` / `recycle-bin` / `macos-bundle` |
| 历史残留 | 旧归一化在 `src-tauri/` 下产生的 11 个 `\var\folders\...` 怪目录 / 文件已移到 `/tmp/afm-test-leftovers/`（**只移不删**）；修复后测试不再产生 |

**仍未解决（别误判为已完成）**：① macOS 上「删除到废纸篓」需要用户在系统设置里授予 Automation/Finder 权限，或改用不依赖 Finder 的 `~/.Trash` 移动实现；② `sysinfo::Disks` 对应的真实卷列表 / 磁盘空间（当前 `list_drives` 非 Windows 返回空、`disk_space` 非 Windows 返回 Err）；③ 7.6 第 4 步的 GUI 实机验证（WKWebView、交通灯、拖放、TCC 弹窗）。

### 7.11 编译警告清零（14 → 0，第 6 轮）

构建日志里的 14 条警告**全部是 Windows-only 代码在 macOS 构建下被 `#[cfg(windows)]` 剔除后产生的 unused import / dead_code**，与功能无关，正属于平台抽象应收口的部分。逐一定位并修复：

| 文件 | 警告 | 修复 |
|---|---|---|
| `discovery/everything_ipc.rs` | `DiscoveryHit` 未用导入（仅被 `#[cfg(windows)] mod win` 使用） | `DiscoveryHit` 改 `#[cfg(windows)] use`，保留 `EverythingProbe`/`EverythingQueryResult` 跨平台 |
| `preview_bridge/mod.rs` | `std::process::Command` 未用 | 导入改为 `#[cfg(windows)]`（`Command` 仅用于 Windows Peek 分支） |
| `win_process.rs` | `Stdio` 未用 + `let mut cmd` 未变 | `Stdio` 改 `#[cfg(windows)] use`；`mut` 加 `#[cfg_attr(not(windows), allow(unused_mut))]` |
| `file_ops/clipboard.rs` | `DROPEFFECT_*`、`encode/decode_hdrop`、`*_list`、`mode/effect_to_mode` 死代码 | HDROP 字节编解码与 mode↔effect 是**跨平台契约**（前端 IPC + 测试用），保留并加 `#[cfg_attr(not(test), allow(dead_code))`；`DROPEFFECT_*` 因被引用自然不再死代码 |
| `navigator/attrs.rs` | `merge_attribute_bits` 死代码 | 纯位运算、被跨平台测试调用，加 `#[cfg_attr(not(test), allow(dead_code))]` |
| `navigator/places.rs` | `DriveKind` 六个变体从不构造 | 枚举是**前端 specta 契约类型**（不能 gate，否则 macOS 丢失 TS 类型），加 `#[allow(dead_code)]` |

**关键约束**：`DriveKind` 与 HDROP 编解码函数**不能**用 `#[cfg(windows)]` 整段剔除——前者是前端类型契约、后者是跨端 IPC 字节格式；只在 lib 非测试构建下静默 dead_code，测试构建仍全平台运行（含 `hdrop_roundtrip_preserves_paths`、`cut_effect_is_move`）。

**验证**：`cargo check`（默认）0 警告 · `cargo check --no-default-features`（Core）0 警告 · `cargo test --lib` 296 passed / 0 failed / 1 ignored · `npm run build` ✅。

### 7.12 GUI 实机验证：窗口终于在 macOS 上显示（第 7 轮）

`tauri dev` 在 macOS 上能编译、能跑（Dock 有图标），但**窗口始终不显示**。`lsappinfo` 在本机对**任何** app 都返回 `Windows=[NULL]`，是无效探测手段；最终用 `screencapture` 截图 + 视觉确认才定位真相。

**根因（两层叠加）**：

1. **WKWebView 隐藏时 rAF 节流导致 reveal 死锁**（`src/state/revealMainWindow.ts`）。
   `afterNextPaint()` 连等两帧 `requestAnimationFrame`。窗口 `visible:false` 时 WKWebView 会**暂停/节流 rAF**，于是 rAF 永不触发 → `win.show()` 永远不被调用 → 窗口永远隐藏。Windows 的 WebView2 在隐藏态 rAF 仍会跑，所以 Windows 没事、macOS 卡死。这是 7.6 第 4 步一直没过的最关键坑。
2. **macOS borderless 窗口上 `maximize()` 抛错**（`revealMainWindow.ts` 的 Windows 专属 maximize 舞步）。
   原逻辑对 borderless 的 `visible:false` 窗口调 `win.maximize()` 会抛错，被外层 `try/catch` 吞掉，`win.show()` 没执行到。

**修复**：

- `src/state/revealMainWindow.ts`：
  - `afterNextPaint()` 增加 `setTimeout(resolve, 100)` 兜底，避免隐藏态 rAF 节流造成的死锁（`win.show()` 必定被调用）。
  - 用 `navigator.userAgent` 判定 macOS，**跳过** `isMaximized()/maximize()` 舞步（该舞步只为 Windows `decorations:false` 的 DWM 几何闪屏），并把它包进 `try/catch` 兜底；paint 后直接 `win.show()` + `setFocus()`。
- `src-tauri/src/lib.rs`（setup）：macOS 上 `window.set_decorations(true)`，恢复原生标题栏。因为 Windows-only 的 `tauri-plugin-frame` Fluent 覆盖标题栏在 macOS 是 no-op，`decorations:false` 会变成无边框、无标题栏、不可拖动的窗口。该 `#[cfg(target_os = "macos")]` 分支不影响 Windows。
- `src-tauri/tauri.conf.json`：维持原设计 `visible:false` + `decorations:false` + `maximized:true`（reveal 机制现在可用，无需降级为 `visible:true`）。

**验证（截图实机确认）**：`tauri dev`（ai-addon flavor）启动后，窗口**完整显示**——原生交通灯（红/黄/绿）、可拖动标题栏、完整文件管理器 UI（侧边栏、标签页、`此电脑`、`主文件夹/桌面/下载` 文件列表），确实在屏幕上而非仅 Dock。`npm run build` ✓ · `test:macos-bundle` ✓ · 4 个读取 `tauri.conf.json` 的静态脚本全 OK · `cargo check` 0 警告。

**仍待 GUI 实机确认（非本任务范围）**：拖放（`tauri-plugin-drag`）、Finder/TCC 废纸篓授权、原生菜单、i18n 里的 Windows 名词、标题栏与前端 Fluent 栏的叠放关系。窗口显示这一最大阻塞已解除。

---

### 7.13 GUI 实机验证：macOS 上 `tauri-plugin-drag` 的 `RecvError` 崩溃（第 8 轮）

**症状**：`npm run dev:ai` 启动、窗口能显示后，进程随即被 `abort`——日志末尾 `thread 'tokio-rt-worker' panicked at .../tauri-plugin-drag-2.1.1/src/commands.rs:162 ... called Result::unwrap() on an Err value: RecvError` / `fatal runtime error: Rust cannot catch foreign exceptions, aborting`。Dock 图标随之消失。

**根因**：`tauri-plugin-drag` 2.1.1 的 `start_dragging` 命令用 `std::sync::mpsc` 在「主线程闭包」与「调用方」之间传递结果：闭包内 `tx.send(r).unwrap()`，调用方 `rx.recv().unwrap()`。`run_on_main_thread` 一旦没能把闭包真正排上主线程（app 重建/teardown，或 macOS 该路径异常），`tx` 随闭包被丢弃，`rx.recv()` 拿到 `RecvError` → `unwrap()` panic → 整个进程 `abort`。该命令**只在文件出站拖拽时**被调用（`src/state/dnd/dragOut.ts` 的 `startNativeFileDragOut` → `startDrag`）。macOS 上任意一次文件拖出都会触发；且 `tauri-plugin-drag` 是 Windows OLE 专属机制——能力层 `nativeDragOut` 在 macOS 本就是 `false`。

**关键约束**：`capabilities/default.json` 引用了 `drag:default` 权限（由 `tauri-plugin-drag` 提供）。若直接把 `.plugin(tauri_plugin_drag::init())` 用 `#[cfg(windows)]` 排除，macOS 构建期 `tauri-build` 会报 `Permission drag:default not found`（权限解析失败），牵动 capability 体系，风险高。**因此保留插件在 macOS 注册**，仅靠前端门控确保 `start_dragging` 永不在 macOS 被调用。

**修复**：
- `src/state/dnd/dragOut.ts`：`startNativeFileDragOut` 入口 `if (isMacPlatform()) return;` 提前返回（OLE 通道在 macOS 不存在/会崩），绝不触达崩溃的 `startDrag` 命令；`armHostOutboundPaths` 同理在 macOS 提前返回（`dnd_arm_outbound_paths` 命令本就是 `#[cfg(windows)]`，调用只会走 catch 被忽略）。
- `src/views/NavigatorView.vue` 的 `onDragStart`：
  - **macOS 分支**：不 `preventDefault`、不调用 OLE；改为在 `dataTransfer` 上挂 `text/uri-list`（`file://` 路径），由 WebView 原生 HTML5 文件拖出承担跨应用拖出，`effectAllowed = "copyMove"`。
  - **Windows 分支**：维持原 OLE 行为（`preventDefault` + `armOutboundPathsForPress` + `startNativeFileDragOut`），契约字面量 `startNativeFileDragOut(paths)` / `armOutboundPathsForPress` 均保留以满足静态脚本。
- `isMacPlatform()` 取自 `src/config/platform.ts`（同步：缓存能力优先、UA 兜底），与 `revealMainWindow.ts` 判定方式一致。

**验证**：`cargo check` 0 警告 · `npm run build` ✓ · `test:file-drag-out` ✓ · `test:cross-app-dnd` ✓；`test:file-drag-inout` 失败是**既存 macOS 失败用例**（`pathToFileURL("C:\\Temp\\x")` 在 macOS 按 POSIX 解析返回 `null`，仅 Windows 可过），与本次改动无关（`git stash` 已证实改动前同样失败）；**该脚本已在 7.15 平台化，`npm test` 现全绿**。`tauri dev` 重启后进程**不再 abort**，日志无 `RecvError` panic。`lsappinfo` 在本沙箱无法确认窗口可见性；窗口显示本身与 7.12 一致（reveal 逻辑未改动）。**拖放真实交互（拖文件到 Finder）仍需本机桌面实机确认**——代码层已证明 `start_dragging` 在 macOS 永不被调用，崩溃路径已切断。

---

### 7.14 用户实机反馈（第 9 轮）：POSIX 路径被 Windows 化 + 双层标题栏

**前提**：本轮反馈发生在 `mac` 分支合并 `origin/master`（`f3979fd`）之后。该 merge 引入任务中心 / 文件操作进度等约 1792 行改动，并把 reveal 的最后一步从 JS `win.show()+setFocus()` 换成宿主命令 `afm_reveal_main_window`（Windows FIL-72）。复核确认：`show_main_window()` 本体是跨平台的（`show` / `unminimize` / `set_focus`），Windows 专属的强制前置激活是 `#[cfg(windows)]`；第 7 轮的 `setTimeout` 兜底 + macOS 跳过 maximize、第 8 轮的 OLE 门控 + `text/uri-list` 拖出**均仍在**。用户能看到界面 → 新的宿主 reveal 路径在 macOS 上也可用。

**反馈 1（已修）：`Path does not exist: Users\demo`**

根因不在 Rust（7.10 已把 `paths.rs` / platform adapter 平台化，macOS 的 `to_native_fs_path` 不改写分隔符），而在**前端仍有一批 Windows 假设的路径工具**，且它们的输出会被当成真实路径交给 `navigator_list`：

| 位置 | 坏行为 | 影响 |
|---|---|---|
| `src/state/navHistory.ts` `buildBreadcrumbs` | `trimmed.replace(/\//g, "\\")` 后按 `\` 分段并丢掉前导根 → `/Users/demo` 变 `Users\demo` | 地址栏面包屑、`navigableParentPath()`（上一级）把它当真实路径 |
| `src/state/pathKey.ts` `toNativeFsPath` | 无条件 `/`→`\` → `/Users/demo` 变 `\Users\demo` | `openState/model.ts`、`tabPins.ts`、`navigatorBrowse.ts` 都用它生成真实路径 |
| `src/state/navigatorBrowse.ts` `joinListingChildPath` | 硬编码 `` `${native}\\${name}` `` 拼子项 | 文件列表里每个条目的 `path` 在 macOS 都是反斜杠路径 |

**修复（按输入形态分流，不改 Windows 行为）**：三处都新增「POSIX 绝对路径」分支——以**单个** `/` 开头且不是 `//`（UNC）时保留 `/` 与前导根、按 `/` 分段/拼接；Windows 盘符与 UNC 路径不会出现这种形态，因此原分支与其静态断言全部保持不变。补回归断言到 `scripts/nav-history-checks.ts`（`/Users/demo/Docs` 面包屑、上一级）与 `scripts/path-key-checks.ts`（`toNativeFsPath` / `normalizePathKey` / openState 转出）。

**验证**：`test:nav-history` ✓ · `test:path-key` ✓ · `test:open-state` ✓ · `npm run build` ✓（27–29s，仅既有 chunk 提示）。

**仍未平台化的同类工具（本轮刻意不动）**：`folderTree.ts`、`favorites.ts`、`dnd/location.ts` / `mode.ts` / `surface.ts` / `paths.ts`、`settingsEffectiveDefaults.ts`、`navHistory.normalizeKey` 仍按 `\` 规范化，但它们只用于内部比较键、拖放同位置判定或设置页展示文案，不产生交给后端的真实路径。`settingsEffectiveDefaults.ts` 的 `toWindowsPath(await join(configDir,"projects"))` 会让设置页显示 `/`→`\` 的路径，属 UI 文案瑕疵，留待后续。

**反馈 2（待定）：原生标题栏 + 前端自绘 Fluent 栏双层叠放**

用户实机确认 7.12 预留的问题属实：`lib.rs` 在 macOS `set_decorations(true)` 给出一条原生标题栏（交通灯 + app 名），前端 `AppShell.vue` 的 `<header class="titlebar">` 又是 40px 自绘 Fluent 栏（刷新、品牌名、命令面板、Win11 三键），控制按钮与品牌重复。

三个可选方向：
- **A · 只用原生标题栏**：macOS 下 `display:none` 掉整条自绘栏。零窗口层改动、最稳，但丢「刷新窗口 / 命令面板」两个按钮（命令面板仍有 ⌘⇧P）。
- **B · 保留两层但去重复**：隐藏自绘栏的 `.window-controls` 与品牌区，使其变成「系统标题栏 + 应用工具栏」（Finder/Safari 式布局）。
- **C · `titleBarStyle: Overlay` + `hiddenTitle`**（7.9 原计划）：交通灯浮在自绘 Fluent 栏左侧，只剩一条栏，功能按钮全保留；需给左侧留约 80px 并隐藏 Win11 三键。

约束：`scripts/check-window-controls.mjs` 硬断言 `mainWindow.decorations === false`，所以 C **只能新增** `titleBarStyle` / `hiddenTitle` 字段，不能改 `decorations`；`set_decorations(true)` 仍需保留在运行时。C 的视觉效果本会话无法截图验证，需用户选定后再动。

---

### 7.15 按排期继续兼容实施（第 10 轮）：标题栏 / 卷信息 / 隐藏文件 / 契约平台化

延续 7.6 遗留项与 P1 缺口。**每项先按 6.5 第 1 条门禁标注四选一**：

| 缺口 | 门禁决策 | 依据 |
|---|---|---|
| 双层标题栏（7.14 反馈 2） | **复用** | Tauri 2.12.5 自带运行时 `WebviewWindow::set_title_bar_style` / `tauri::TitleBarStyle::Overlay`，无需新依赖、无需 objc、无需改 config |
| 卷列表 / 磁盘空间（7.11 ②） | **引入** | 6.5 第 2 条明确指定 `sysinfo::Disks`；自写 `statvfs` 绑定被门禁禁止 |
| POSIX 隐藏文件（P1） | **自写（标准库）** | POSIX 没有 hidden attribute，dot-file 是命名约定，无 crate 可代劳 |
| 设置页默认路径（7.14 遗留） | **复用** | `@tauri-apps/api/path` 的 `join()` 已返回 host 原生路径，只需去掉多余的 `\` 改写 |

**改动**：

- **标题栏方案 C**（7.9 原计划，7.14 三选一中的 C）：`src-tauri/src/lib.rs` 在 macOS 的 `set_decorations(true)` 之后追加 `set_title_bar_style(tauri::TitleBarStyle::Overlay)` + `set_title("")`。查证 `tauri-runtime-wry` 的 `SetTitleBarStyle` 处理：Overlay 会 `set_titlebar_transparent(true)` + `set_fullsize_content_view(true)`，交通灯浮在 webview 内容上。**关键收益**：用运行时 API 实现，`tauri.conf.json` 的 `decorations:false` 一字未改（`check-window-controls.mjs` 硬断言），Windows 完全不受影响。`AppShell.vue` 引入 `isMacPlatform()` 给 `.shell` 加 `mac-overlay-titlebar` class：自绘 Fluent 栏左侧留 78px 给交通灯、隐藏重复的 Win11 三键；刷新与命令面板按钮保留。
- **卷 / 磁盘空间**：`src-tauri/Cargo.toml` 引入 `sysinfo = { version = "0.40.3", default-features = false, features = ["disk"] }`（只开 disk，避免拖入 component/gpu/network 等）。`navigator/places.rs::list_drives` 非 Windows 分支改为遍历 `Disks::new_with_refreshed_list()`，只暴露 `/` 与 `/Volumes/*`（过滤 Finder 不可见的 `/System/Volumes/*` 辅助卷），`letter` 用挂载点 basename；`navigator/disk_space.rs` 非 Windows 分支改为「最长匹配挂载点」返回 `available_space()` / `total_space()`，替换恒 `Err`。`volume_root` 保留给 Windows 并加 `cfg_attr(not(windows), allow(dead_code))`。两处各补一个 macOS 单测，`places.rs` 的卷路径断言改为 `cfg` 分支（Windows 验 `\` 结尾、POSIX 验 `/` 开头）。
- **隐藏文件**：`navigator/list.rs::entry_is_hidden` 签名加 `name: &str`，Windows 仍用 `FILE_ATTRIBUTE_HIDDEN`，非 Windows 用 `name.starts_with('.')`（排除 `.` / `..`）。`list.rs` / `stat.rs` / `search.rs` 三处调用同步传入文件名。顺带把 `stat_path` 的 `pb.to_string_lossy().replace('/', "\\")` 换成 `crate::paths::display_path(&pb)`——这是 7.14 之后发现的又一处真实路径 Windows 化。
- **设置页**：`settingsEffectiveDefaults.ts` 的 `toWindowsPath` 改为 `toNativeDisplayPath`，POSIX 绝对路径保留 `/`；`check-settings-ui-regroup.mjs` 加断言守住。
- **静态契约平台化（顺手修，两项均为既存漂移）**：① `check-perf-probe.mjs` 两处 `from "./perfProbe"` 断言改正则允许 `.ts` 后缀（源码一直写 `./perfProbe.ts`，`69f171e` 版本即如此）；② `check-file-drag-inout.mjs:95` 的 `pathToFileURL("C:\\Temp\\x")` 断言按 `process.platform` 门控，非 Windows 补一条 POSIX URL 断言。7.13 记为「既存 macOS 失败用例」的这一条至此修复。

**验证**：

| 项目 | 结果 |
|---|---|
| `cargo check`（默认） / `--no-default-features`（Core） | ✅ 均 0 警告 |
| `cargo test --lib` | ✅ 299 passed / 0 failed / 1 ignored（原 296，本轮 +2 个卷/磁盘用例 +1 个既有用例计数差异） |
| `npm run build`（`vue-tsc` + vite） | ✅ 通过 |
| **`npm test`（全量前端契约）** | ✅ **exit 0，macOS 上首次全绿**（此前唯一阻塞 `test:file-drag-inout` 已平台化） |
| 5 个读 `tauri.conf.json` 的静态脚本 + perf/show-hidden/properties/settings 等 | ✅ 全通过 |

**仍待用户实机确认**：`set_title("")` 对窗口菜单的影响、「此电脑」卷列表呈现、dot-file 隐藏开关、磁盘空间用量条。交通灯落点与 78px 留白已在 7.17 实机截图验证并补齐：初版只做了水平避让，垂直方向仍错开约 26pt。其余项沙箱内无法截图。

**下一轮候选（按 6.5 门禁逐项判定，勿误判为已完成）**：Rust 侧仍有约 20 处 `replace('/', "\\")` 规范化（`visit_stats` / `projects` / `favorites` / `discovery` / `ai/*` / `file_ops` / `terminal` 等）。多数是比较键 / 缓存键，不产生交给后端的真实路径；但 `terminal/mod.rs:142`、`ai/open_editor.rs:58`、`ai/harness.rs:28`、`file_ops/recycle.rs:26` 疑似产出真实路径，需逐个确认后再平台化。P1 的终端/IDE `open -a`、废纸篓 TCC（或改 `~/.Trash` 移动）、全局搜索 Spotlight、进程树清理，以及 P2 的 i18n Windows 名词 / 原生菜单，仍按原排期。**（7.20 已逐个确认并把「真实路径」类收口：终端 / IDE 归位适配层、`ai/*` 与 `open_editor` / `harness` / `visit_stats` / `projects/index` 委托 `crate::paths`；`recycle` / `safe_io` / Windows-only 模块确认为可豁免，见 7.20。7.21 已完成「全局搜索」Spotlight 归位，并纠正了当时把 `discovery/blacklist` 判为「仅比较键」的遗漏——它其实会回显到设置页。**7.22 已回收 24 组主文案的 P2 i18n Windows 名词（系统文件管理器 / 废纸篓 / 此电脑 / 快捷键 / 默认路径），但仍有示例路径、后端硬编码中文等残留，见 7.22 边界。仍未排期：废纸篓 TCC / `~/.Trash` 列表还原、原生菜单；进程树清理与 sidecar 解释器已于 7.23 归位适配层。）**

### 7.16 架构收口（第 11 轮）：把第 7~10 轮的旁路压回适配层

**触发**：用户实机截图（交通灯单栏、无双层标题栏，地址栏 `/Users/demo/Downloads` 已是 POSIX 路径）+「收口」。上一轮审计结论：适配层骨架符合 6.1 设计，但 7~10 轮的修复全在业务模块就地打平台分支。

**改动前基线**：Rust 端 210 处平台 `#[cfg]` 分布在 36 个文件；`Platform` trait 仅 8 个方法、`PlatformCapabilities` 24 个静态 bool 且前端只消费 3–4 个；前端 `navHistory.ts` / `pathKey.ts` / `navigatorBrowse.ts` / `settingsEffectiveDefaults.ts` 四份各自实现 POSIX/Windows 判定；`macos.rs` 仍声明 `native_drag_out = true`，而 7.13 已禁用 AppKit 拖出改走 WebView `text/uri-list` —— 能力层不再是单一事实源。

| 收口项 | 做法 | 落点 |
|---|---|---|
| 前端真实路径 | 新增 `src/state/fsPath.ts` 单一入口：`isPosixAbsolute` / `fsPathSeparator` / `toNativeFsPath` / `joinFsPath` / `splitFsPath`，四个文件改为委托 | `pathKey.ts` 保留 `export function toNativeFsPath` 薄包装（既有契约与公共 API 不变）；`navigatorBrowse.ts` 只保留 provider URI 分支；`navHistory.ts` 只保留 Windows 面包屑展现逻辑 |
| Rust 四项能力 | `Platform` trait 增 `volumes()` / `volume_space()` / `is_hidden()` / `window_chrome()`，中立类型 `Volume` / `VolumeKind` / `VolumeSpace` / `WindowChrome` | Win32 `GetLogicalDrives` / `GetVolumeInformationW` / `GetDiskFreeSpaceExW` 与 `sysinfo::Disks` 分别搬进 `platform/windows.rs` / `macos.rs`；`places.rs::list_drives` 与 `disk_space.rs` 只剩「中立类型 → wire 类型」映射；`list.rs::entry_is_hidden` 委托；`lib.rs` 改为按 `window_chrome()` 判定，删掉 `#[cfg(target_os="macos")]` 分支 |
| 能力层 | `native_drag_out` 在 macOS 归 false（7.13 已禁用），新增 `webviewDragOut`（macOS true）与 `volumeList`（两端 true）；Rust struct / TS type / `NEUTRAL_PLATFORM` 同步 | `test:shortcuts` 的字段同步断言守住 |
| 门禁 | 6.5 新增第 6 条「架构归位门禁」：新增平台分支必须先回答「为何不能放进 `platform/` 或 `fsPath.ts`」 | 新增 `scripts/check-fs-path.mjs` + `fs-path-checks.ts`，静态断言四个文件必须委托 `fsPath.ts`、不得再本地做 POSIX 判定 |

**契约同步（4 处既有断言随实现位置更新，非放宽）**：`check-arch-optimize.mjs`（Windows 规范化实现移到 fsPath）、`check-navigator-guide.mjs`（`GetDriveTypeW` / `GetVolumeInformationW` 移到 Windows adapter）、`check-navigator-nonblocking.mjs`（卷枚举委托 + 两端 adapter 各有实现）、`check-settings-ui-regroup.mjs`（设置页委托 fsPath）。`check-path-key.mjs` / `check-window-controls.mjs` 等一字未改仍通过。

| 验证 | 结果 |
|---|---|
| `cargo check`（默认 / `--no-default-features`） | ✅ 均 0 警告 |
| `cargo test --lib` | ✅ 299 passed / 0 failed / 1 ignored（搬走 1 个 Win32 纯函数测试到 windows.rs，补 1 个跨平台空路径用例，计数不变） |
| `npm run build` / **`npm test` 全量** | ✅ / ✅ **exit 0** |
| `test:shortcuts`（能力字段同步）/ `test:fs-path`（新增） | ✅ / ✅ |

**Windows 侧限制**：`platform/windows.rs` 的 Win32 代码是逐字搬迁（仅 `DriveKind` → `VolumeKind`），本机 macOS 无法编译 Windows target；静态契约守住了调用点，仍需 Windows CI 实机编译确认。

**效果与边界**：平台 `#[cfg]` 210 → 205（多数是整模块 Windows-only 集成，不算散落，故降幅有限）；真正的变化是 4 项能力的**实现位置**从业务模块移到适配器，trait 8 → 12 个方法。仍未做（勿误判为完成）：6.5 第 4 条的动态能力探测（仍是静态 bool）、5.6 第 1 条的 `FsPath` 新类型（`typed-path` 未回排期）、6.1 第 1 条的 IPC 契约清洗（`DriveInfo.letter` 等 Windows 名词仍在）、图标 / 废纸篓 / 剪贴板 / 搜索四个 provider 仍在各自模块的 `#[cfg]` 里。

### 7.17 交通灯与自绘标题栏真正同一行（第 12 轮）

**触发**：用户实机反馈「红绿灯位置还是单行的」，并追问「在刷新按钮前留出的空白，难道不是为了给红绿灯留出空间从而让红绿灯和我们的标题栏在同一水平吗？」——指出 7.15 的 78px 留白只解决了水平避让，垂直方向没对齐。

**实机测量**（`screencapture -l <windowId>` 单独截窗口 + 像素刻度，Retina 2x）：

| 元素 | 窗口 y（pt） |
|---|---|
| 原生交通灯中心 | ~17 |
| WKWebView 网页内容起点 | ~28（titlebar safe area） |
| 自绘 Fluent 栏（40pt）内刷新按钮 / 品牌中心 | ~45 |

`NSWindow.styleMask = 32783`（`Titled|Closable|Miniaturizable|Resizable|FullSizeContentView`），`contentLayoutRect` 高度比 `contentView` 少 28pt。即 **Overlay / FullSizeContentView 已生效，但 WKWebView 仍把网页内容限制在安全区内**，而交通灯由 AppKit 画在安全区上方。所以 78px 留白避开了「不存在于同一行」的按钮。

**四试四否**（均有实机截图/诊断数据）：① 运行时把 `set_title_bar_style(Overlay)` 延后到 `set_decorations(true)` 的异步 styleMask 之后 —— mask 本来就对，布局不变；② `tauri.conf.json` 加 `titleBarStyle: "Overlay"` + `hiddenTitle: true` —— 保留（窗口创建时即 Overlay、隐藏原生标题），但不改变 safe area；③ `index.html` 加 `viewport-fit=cover` —— macOS WKWebView 无效，已撤回；④ 前端负 margin / transform 上移 `.shell`，以及原生把 `nsView.frame` 设为 `contentView.bounds` —— 前者把自绘栏推出视口被裁、后者 frame 本来就相等，均已撤回。

**采用方案（AppKit 按钮位移，归位适配层）**：照 tao `inset_traffic_lights` 的思路扩大交通灯私有容器并重置三个按钮的水平位置，把它们下移到自绘栏中心。

| 落点 | 内容 |
|---|---|
| `platform/macos.rs` | `align_traffic_lights(&WebviewWindow)` + 标定常量 `TRAFFIC_LIGHT_INSET_PT = 43.0`；所有 `objc2` / AppKit 细节都在 macOS 适配器内，且 `#[cfg(target_os="macos")]` 双层门控（该文件也是 Linux fallback） |
| `platform/mod.rs` | `align_window_chrome(&WebviewWindow)` 单一入口，非 macOS 为空实现，lib.rs 不再出现 AppKit |
| `lib.rs` | ① Overlay 设置排到 `set_decorations(true)` 的异步 styleMask 之后；② `on_window_event(Resized)` 主线程重新对齐（AppKit 在 resize / 标题变化后会重置按钮 frame）；③ 启动 1.6s 兜底一次（reveal 无 resize 时），调用幂等 |
| `Cargo.toml` | macOS-only 引入 `objc2` / `objc2-app-kit`（tao/tauri 已用同版本，Cargo.lock 仅 +2 行） |
| `tauri.conf.json` | 只**新增** `titleBarStyle` / `hiddenTitle`，`decorations:false` 一字未改（`check-window-controls.mjs` 硬断言） |

**6.5 门禁判定「引入」**：Tauri 2.12.5 的运行时 `WebviewWindow` 没有 `set_traffic_light_position`（只有 builder-only 的 `traffic_light_position`），也没有任何调整 `NSWindow` 标准按钮 frame 的等价 API；这是 macOS 原生窗口按钮定位的唯一途径，且依赖按 target 门控、Windows 不受影响。

**契约**：`check-window-controls.mjs` 新增 5 条断言守住 config 两字段、`lib.rs` 必须委托 `platform::align_window_chrome` 且不得出现 `objc2_app_kit`、适配器必须实现 inset、`platform/mod.rs` 必须暴露入口、AppKit 依赖必须 target-gated。

| 验证 | 结果 |
|---|---|
| 实机截图（刻度图） | ✅ 交通灯 / 刷新按钮 / 品牌中心同在 y≈165–175 px（2x），误差 ≤ 5pt，78px 留白无重叠 |
| `cargo check`（默认 / `--no-default-features`）/ `cargo test --lib` | ✅ 0 警告 / 299 passed, 0 failed, 1 ignored |
| `npm test` 全量 / `npm run build` / `test:window-controls` / `test:macos-bundle` | ✅ 全通过 |

**边界（勿误判为完成）**：43pt 是本机 macOS 15.x 的实机标定值（AppKit 会把按钮在容器内垂直居中，容器增高与按钮下移非线性），换系统版本或窗口缩放需重新标定；全屏进出、窗口 tabbing、标题变化是否重置按钮位置尚未逐个实机验证（目前只覆盖 `Resized`）；窗口顶部 28pt 安全区仍在，但与自绘栏同色且无分隔线，视觉上是一条栏而非双层标题栏。

> **7.18 整体替换**：下面找到了这 28pt 带的真正来源（运行时 `set_decorations(true)` 丢了 `FullSizeContentView`），7.17 的按钮位移属于治标，已连同 `TRAFFIC_LIGHT_INSET_PT` 一起删除；交通灯改回 AppKit 原生位置。

### 7.18 标题栏上移到窗口顶沿（第 13 轮）

**触发**：用户实机反馈「没必要留上面的一条了吧，可以让软件的标题栏位置上移」——7.17 收尾时窗口最上方仍有一条约 28pt 的浅灰空白带（原生标题栏区域），自绘 Fluent 栏被压在它下面，交通灯则被人工下移到自绘栏里。

**定位方式**：在 macOS 适配器里临时注入一段 ObjC 诊断（打印 `window.frame` / `styleMask` / `contentLayoutRect` / `contentView.frame` / `WryWebViewParent.frame` / `safeAreaRect` / 视图层级类名），实机读数：

| 项 | 修复前 | 修复后 |
|---|---|---|
| `NSWindow.frame.size.height` | 1175 | 1175 |
| `styleMask` | **15**（`Titled\|Closable\|Miniaturizable\|Resizable`） | **32783**（含 `FullSizeContentView`） |
| `titlebarAppearsTransparent` / `titleVisibility` | `true` / `Hidden` | 不变 |
| `contentView.frame.size.height` | **1147**（比窗口矮 28pt） | **1175**（铺满） |
| `WryWebViewParent.frame` | 1147 | 1175 |
| `contentLayoutRect` | 1147 | 1147（相对 contentView 的布局区） |

**根因**：`tao::window::set_decorations(true)` 用 `set_style_mask_async` 以 `Titled|Closable|Miniaturizable|Resizable` **整体重写** styleMask，把窗口创建时 `TitleBarStyle::Overlay` 带上的 `FullSizeContentView` 位清掉了。AppKit 随即把 `contentView` 收矮 28pt，WKWebView 只能从原生标题栏带下方开始绘制 —— 这才是那条空白带的来源，与 7.17 判断的「WKWebView titlebar safe area」不是一回事。运行时 `set_title_bar_style(Overlay)` 虽会调 `set_fullsize_content_view(true)`，但它经 Tauri 事件循环异步投递，最终生效的仍是清位后的 mask。

**采用方案**：让适配层把窗口几何恢复到 Overlay 应有的状态，而不是继续挪按钮。

| 落点 | 内容 |
|---|---|
| `platform/macos.rs` | `extend_content_under_titlebar(&WebviewWindow)`：补 `NSWindowStyleMask::FullSizeContentView`、保持 `titlebarAppearsTransparent(true)`，并把 wry 的 parent view `setFrame` 到新的 `contentView.bounds`。**不再触碰任何按钮 frame**，交通灯保持 AppKit 原生位置，此时恰好落在自绘栏中心线上（约 6pt 偏差，肉眼齐平）；删掉 7.17 的 `align_traffic_lights` 与 43pt 标定常量 |
| `platform/mod.rs` | 单一入口改名 `extend_content_under_titlebar`，非 macOS 空实现 |
| `lib.rs` | `set_title_bar_style(Overlay)` 之后立即在**同一主线程任务**里调适配层；`Resized` 事件与启动 1.6s 兜底继续幂等重设（AppKit 在 reveal / resize 后会重排 contentView），避免启动瞬间露出空白带 |
| `AppShell.vue` | 顶部可拖动的原生带消失后，左侧 78px 留白不能变成死区：由 `padding-inline-start` 改为真实的 `mac-traffic-light-gutter` flex 项，带 `data-tauri-drag-region` 与双击最大化；`v-if="isMacPlatform()"`，Windows 不渲染 |
| 静态契约 | `check-window-controls.mjs` 把 5 条 7.17 断言改为 7.18：委托新入口、适配器必须出现 `FullSizeContentView` / `setTitlebarAppearsTransparent` 且**不得**再出现 `TRAFFIC_LIGHT_INSET_PT`、mod.rs 暴露新入口、AppShell 必须有 78px gutter；config 两字段与 AppKit 依赖 target 门控断言保留 |

**6.5 门禁**：仍属「引入」AppKit（Tauri 运行时无等价 API 可恢复 styleMask），但比 7.17 收窄：只改窗口样式位与 root view frame，不再依赖私有按钮容器布局与本机标定常量；依赖仍按 `cfg(target_os="macos")` 门控，Windows 侧零影响。

| 验证 | 结果 |
|---|---|
| 实机截图（窗口单独截 + 顶部放大） | ✅ 浅灰带完全消失，自绘栏贴到窗口圆角内最上沿；交通灯 / 刷新按钮 / 品牌同处一行、无重叠、无裁切 |
| `cargo check`（默认 / `--no-default-features`） | ✅ 0 警告 |
| `cargo test --lib` / `npm test` 全量 / `npm run build` | ✅ 299 passed, 0 failed, 1 ignored / exit 0 / OK |
| `test:window-controls` / `test:macos-bundle` | ✅ |

**边界（勿误判为完成）**：`FullSizeContentView` 与 wry parent view frame 是 AppKit 行为，仍需 macOS 实机；全屏进出、窗口 tabbing、标题变化后 AppKit 是否再次清位只靠 `Resized` + 启动兜底覆盖，未逐个验证；78px gutter 的拖动 / 双击最大化需用户实机确认；Windows 侧未实机编译（`v-if` 不渲染 gutter，但前端构建已过）。「不再触碰按钮 frame」这句在 7.19 被修正：用户要求交通灯精确纵向居中，改为动态调整按钮 origin。

### 7.19 交通灯在标题行纵向居中（第 14 轮）

**触发**：用户实机反馈「红绿灯下移一些。应该在标题行纵向居中」——7.18 保留的 AppKit 原生位置仍比自绘栏中心高约 6pt。

**精确测量**（把 `screencapture -l` 截图用 ImageMagick 转 P6，再用 Python 标准库按颜色聚类，避免依赖 PIL / 目测读刻度；坐标为 Retina 2x 原图像素）：

| 元素 | 中心 y（原图 px） | 换算 pt |
|---|---|---|
| 三个交通灯圆点 | **103.5** | 51.75 |
| 刷新按钮蓝色边框中心 | **114.5** | 57.25 |
| 标题栏白区 y=80..155 / 底部分隔线 ~155 | ~116 | ~58 |

即需要下移约 **12px = 6.25pt**。

**先否决配置级方案**：`tauri-utils` 的 `WindowConfig` 确实有 `traffic_light_position`（`trafficLightPosition`），tao 也有 `inset_traffic_lights` / `reapply_traffic_light_inset`（在 `drawRect:` 与窗口 delegate 里自动重设，比手写钩子更可靠）。但该字段文档明确「Requires titleBarStyle: Overlay and **decorations: true**」，而本项目主配置是 `decorations:false`、运行时才改 true；tao 的 `inset_traffic_lights` 对 `standardWindowButton(...)` 直接 `unwrap()`，创建时无按钮会 panic。Tauri 2.12.5 的高层 `WebviewWindow` 也没有运行时 `set_traffic_light_position`（只有 builder-only）。所以不能用配置，也不为此改动跨平台窗口配置。

**采用方案（动态居中，不用标定常量）**：`platform/macos.rs` 新增 `center_traffic_lights_on_caption(&NSWindow)`：

1. 用 `NSView::convertRect_toView(button.bounds, nil)` 把每个按钮转到窗口坐标系，算出它当前中心距窗口顶部的距离；
2. 与 `CAPTION_HEIGHT_PT / 2 = 20pt` 比较得到 delta，按 delta 调整 `setFrameOrigin`（AppKit y 轴向上，下移即减小 `origin.y`）；
3. `|delta| <= 0.5pt` 直接跳过 → **幂等**，且每次都从当前实测位置计算，macOS 版本改变原生落点也能自适应；**不动私有容器**，7.17 的 43pt 容器增高 hack 不回来。

`CAPTION_HEIGHT_PT = 40.0` 镜像 `src/styles/fluent-tokens.css` 的 `--fluent-titlebar-height: 40px`，契约脚本同时断言两处，防止 CSS 改高度后 Rust 没跟上。调用点沿用 7.18 的三处幂等钩子（Overlay 主线程任务、`Resized` + 新增 `Focused`、启动 1.6s 兜底），`Focused` 用来兜住 AppKit 在标题变化 / 全屏进出后把按钮重设回默认位置的情况。

| 验证 | 结果 |
|---|---|
| 实机像素测量 | ✅ 交通灯中心 115.5px、刷新按钮中心 114.5px，差 1px ≈ **0.5pt**；三个圆点完整无裁切、与刷新按钮无重叠 |
| `cargo check`（默认 / `--no-default-features`） | ✅ 0 警告 |
| `cargo test --lib` / `npm test` 全量 / `npm run build` | ✅ 299 passed, 0 failed, 1 ignored / exit 0 / OK |
| `test:window-controls`（含新增 7.19 常量同步断言）/ `test:macos-bundle` | ✅ |

**边界**：AppKit 在标题变化 / 全屏进出 / tabbing 后重置按钮 frame 的具体时机未逐个实机验证（`Resized` + `Focused` + 启动兜底覆盖常见路径）；`CAPTION_HEIGHT_PT` 与 CSS 的同步靠静态断言而非运行时读取；Windows 侧不执行该函数（适配层 macOS 门控）。

### 7.20 回到 Mac 适配：终端 / IDE 归位适配层 + 真实路径尾巴（第 15 轮）

**触发**：用户「继续回到 Mac 适配」。回到 7.15 结尾列出的下一轮候选，以及 P1 表第一行「终端 / IDE」——这两项在 macOS 上此前完全不可用：`TERMINAL_DEFS` / `IDE_DEFS` 只有 `wt` / `powershell` / `.cmd` + `C:\Program Files`，`USERNAME` 环境变量在 macOS 也不存在。

**先过 6.5 第 1 / 6 条门禁**：

| 缺口 | 四选一 | 依据 |
|---|---|---|
| 终端 / IDE 应用发现与启动 | **自写（标准库 + 系统 `/usr/bin/open`，无新依赖）** | `tauri-plugin-opener` 只负责「用默认程序打开路径」，不覆盖「在指定目录打开指定终端 / 用指定 IDE 打开」；PATH 探测复用 7.10 已有的 `win_process::resolve_on_path`。按第 6 条，平台差异全部放进 `platform/`，业务模块零 `#[cfg]` |
| 真实路径 Windows 化（P0-3 尾巴） | **复用** | `crate::paths::{join_relative, to_native_fs_path, normalize_path_for_comparison}` 在 7.10 / 7.16 已建好，不再各模块手写 |
| 静态契约 / 依赖体检文案 | **自写**（项目既有脚本模式） | 无库可代劳 |

**适配层（12 → 17 个 trait 方法）**：`platform/mod.rs` 新增中立类型 `LaunchTarget { id, name, resolved_path, supports_elevation }` 与五组入口 `terminal_targets` / `ide_targets` / `configured_launch_target` / `launch_terminal` / `launch_ide`。`resolved_path = None` 表示未安装，前端继续显示「不可用」行而不必知道发现逻辑。

| 落点 | 内容 |
|---|---|
| `platform/windows.rs` | 逐字搬入 `TERMINAL_DEFS` / `IDE_DEFS`（含 Qoder 的 Coder/Corder 拼写）、`expand_user(USERNAME)`、`resolve_candidate`、`terminal_args`、`spawn_elevated`（`ShellExecuteExW` runas）、`win_process::command` 启动 IDE；Windows 行为一字未改 |
| `platform/macos.rs` | `TERMINAL_APPS`（Terminal / iTerm2 / Warp）与 `IDE_APPS`（VS Code / Cursor / Trae / Qoder）在 `/Applications`、`~/Applications`、系统 Utilities 下探测 `.app`，裸名走 PATH CLI；启动用 `/usr/bin/open -a <.app> <dir>`，CLI 直接 spawn；`supports_elevation = false`（无 UAC，`configured_launch_target` 接受 `.app` **目录**） |
| `terminal/mod.rs` / `ide_launch/mod.rs` | 只留 wire 类型、目录解析、设置覆盖顺序（auto → configured → none）与 `platform::launch_*` 调用；删除全部 Windows 常量、`USERNAME`、`replace('/', "\\")`、`#[cfg(windows)]` |
| `deps_probe.rs` | `terminal_status` 改显示适配器给的 `name`，删掉「确认 cmd.exe / powershell.exe」「可选安装 Windows Terminal / Git Bash」两条 Windows 指引 |

**顺带收掉 7.15 遗留的真实路径尾巴**（逐个确认，不是一刀切全替）：

- 真实路径：`ai/open_editor.rs` 的 `opened`（此前会把 POSIX 路径 Windows 化后交给 `ide_open_path`）、`ai/harness.rs` / `ai/harvest.rs` 的报告路径、`ai/mcp.rs` 与 `ai/mod.rs` 两处 `ws.join(rel.replace(...))`、`ai/agents.rs` 四处配置输入改写 → 统一委托 `crate::paths`。
- **真 bug**：`projects/index.rs::active_still_under_root` 的 `root_norm` 已在 7.10 改为 host 原生路径，`path_norm` 却仍硬编码 `\` + `\` 前缀，macOS 上每次加载都会判定 active project 不在 root 下 → 两侧都走 `normalize_path_for_comparison`，分隔符取 `platform::current().path_separator()`。
- `visit_stats::normalize_path` 是比较 / 持久化键，改委托 `to_native_fs_path`；原 `C:\Docs` / `C:/Docs` 测试夹具改成 host 分隔符变体（Windows `\` vs `/`，POSIX 尾斜杠），并补一条「POSIX 路径永不变成反斜杠键」断言。
- **确认后豁免**（6.5 第 6 条）：`open_with/mod.rs` 与 `shell_menu/mod.rs` 的 `replace` 位于整模块 `#[cfg(windows)] mod windows_impl`；`file_ops/recycle.rs::normalize_path` 是 `#[cfg(any(windows, test))]`；`file_ops/safe_io.rs::volume_id` 是纯 Windows 卷标识（POSIX 返回 `None` → 走 copy+verify）；`favorites` / `projects/ops` / `discovery/blacklist` / `providers/util` 四处只作比较 / 缓存键。均不产出交给后端的真实路径，留按 P2 平台化。

**本机实机发现**（`platform/macos.rs` 新增断言直接读磁盘，不是模拟）：

| 目标 | 结果 |
|---|---|
| Terminal.app | 存在：`/System/Applications/Utilities/Terminal.app` → `available` |
| iTerm.app / Visual Studio Code.app / Cursor.app | 存在 → `available` |
| Warp / Trae / Qoder | 未安装 → `unavailable`（仍返回预设行） |
| `cursor` CLI | 实测 `/usr/local/bin/cursor` 在 PATH，但 `.app` 候选优先命中 |

**契约**：新增 `scripts/check-launch-targets.mjs`（注册 `test:launch-targets` 并纳入 `npm test`）：断言中立类型与 5 组入口、两端适配器都实现 5 个方法、Windows 预设逐字保留（回收从 `ide_launch` 移走的两条 Rust 测试）、macOS 预设 id 与 `SettingsView` 的 `OPEN_TOOL_IDS` 一致、业务模块不得出现 `USERNAME` / `C:\Program Files` / `win_process` / `#[cfg(windows)]` / 斜杠改写、8 个真实路径生产模块不得再本地改写分隔符、`deps_probe` 文案不得回退。`check-no-console-flash.mjs` 的两条断言按 7.16 惯例改指向适配层（**未放宽**：仍禁止 `where.exe`、仍要求 `win_process::resolve_on_path` / `win_process::command`）。

| 验证 | 结果 |
|---|---|
| `cargo check`（默认 / `--no-default-features`） | 通过，均 0 警告 |
| `cargo test --lib` | 通过，**301 passed / 0 failed / 1 ignored**（原 299；−2 个 Windows-only IDE 夹具测试，+4 个适配层 / 路径断言） |
| `npm test` 全量 / `npm run build` | exit 0 / 通过 |
| `test:launch-targets` / `test:window-controls` / `test:macos-bundle` / `git diff --check` | 全通过 |
| 改动过的 Rust 文件 `rustfmt --check` | exit 0（含 macOS 不编译的 `windows.rs`，至少语法级校验） |

**Windows 侧限制**：本机 `rustup target add x86_64-pc-windows-gnu` 成功，但 `cargo check --target x86_64-pc-windows-gnu` 在 C 依赖的 `cc-rs` 处因缺 `x86_64-w64-mingw32-gcc` 失败（未为此装整套 mingw）。因此 Windows adapter 的**类型检查仍待 Windows CI**；本机只有语法解析 + 静态契约守住调用点，别误判为已编译验证。

**边界（勿误判为完成）**：`/usr/bin/open -a <.app> <dir>` 是否在 iTerm2 / Warp 里也都落到指定目录、右键菜单与设置页「打开工具」的实际展示与点击，仍需用户在 GUI 实机确认（本轮只验证了发现结果与命令构造）；macOS 不再渲染「以管理员身份」子项是预期行为；5 语言 i18n 里的「资源管理器」「可选 Code.exe 路径」等 Windows 名词仍是 P2 遗留，本轮只清了 Rust 里的硬编码文案；`PlatformCapabilities` 仍是静态 bool（6.5 第 4 条）、图标 / 废纸篓 / 剪贴板 / 搜索四个 provider 未归位、`DriveInfo.letter` 等 IPC Windows 名词未清洗。**（7.21 已回收「搜索」这一项，见下。）**

---

### 7.21 全局搜索归位适配层：Spotlight + 路径 / 文案平台化（第 16 轮）

**触发**：用户「已经有了两种终端，继续推进」——7.20 的终端发现算 GUI 实机确认通过（Terminal / iTerm2 两种）。按 P1 表下一行核对时发现 **Workspace / AI Skills 的链接创建其实早已完成**（`projects/members.rs` / `ai/deploy.rs` 都有 `#[cfg(not(windows))]` 的 `std::os::unix::fs::symlink` + `remove_file`），P1 表该行是初始调研状态、未随实现更新。因此按 7.15 候选清单推进**全局搜索**：`discovery/everything_ipc.rs` 非 Windows 是恒 `Err` 的 stub、`fallback.rs` 恒空结果，macOS 上搜索页**永远没有命中**。

**先过 6.5 第 1 / 6 条门禁**：

| 缺口 | 四选一 | 依据 |
|---|---|---|
| 主机级索引 | **复用系统能力**（`/usr/bin/mdfind`，无新依赖） | Spotlight 与 Finder 共用同一索引；`NSMetadataQuery` 需要 AppKit + run loop，覆盖不了更多场景，只会把 objc 细节带进业务层。`mdfind` 是 macOS 自带 CLI，与 7.20 复用 `/usr/bin/open` 同性质 |
| backend 接入点 | **自写薄适配**（`PlatformSearchBackend`） | `EverythingBackend` 是项目自有 trait，无库可代劳；它只做中立类型映射，零 OS 细节 |
| 真实路径 / 文案平台化 | **复用** `crate::platform::to_native_fs_path` / `src/state/fsPath.ts` | 7.10 / 7.16 已建好，不再各模块手写 |

**改动**：

| 落点 | 内容 |
|---|---|
| `platform/mod.rs` | 新增中立类型 `GlobalSearchEngine` / `SearchHit` / `SearchQueryResult` / `SearchProbe`；trait 17 → **20 个方法**（`global_search_engine` / `probe_global_search` / `global_search`）；能力层加 `global_search_engine: "everything" \| "spotlight" \| "none"` 字段，macOS 的 `global_search` 同步由 `false` 修正为 `true`（现在确实有主机索引） |
| `platform/macos.rs` | `/usr/bin/mdfind -0 -name '"<literal>"'`：`-0` 用 NUL 分隔避免文件名含换行；整词加引号让 `a(b)!?` 变成字面短语（实测不加引号会匹配到无关文件）；`\`/`"` 转义防止跳出短语；空查询短路（`mdfind -name ""` 会报错却 exit 0）；`probe` 只 stat 二进制，不跑查询；`parse_mdfind_output` 按 skip/take 分页、`Path::is_dir()` 判目录。非 macOS Unix 返回 `None` |
| `platform/windows.rs` | 只声明 `EverythingIpc`，`global_search()` 返回 `Err`——Windows 仍走原 `everything_ipc.rs`，**未实现第二个索引、行为一字未改** |
| `discovery/` | `DiscoveryEngine` 加 `Spotlight`；`EverythingBackend` 加默认 `engine()`；新增 `platform_backend.rs` 薄映射；`mod.rs` 按 `platform::global_search_engine()` 选 backend（**零 `#[cfg]`**）；`service.rs` 的 status / search / 结果标签改用 `self.everything.engine()`，Spotlight 跳过 Everything 的空查询 totitems 估算 |
| `discovery/blacklist.rs` | **纠正 7.20 的「只作比较键」判断**：黑名单值会被持久化并回显到设置页 textarea，是真实路径。改委托 `to_native_fs_path`，前缀匹配按路径边界接受两种分隔符；Windows 夹具改平台感知，新增 POSIX 不产生 `\Users\…` 断言 |
| `DiscoveryView.vue` | **修两个真 bug**：`normalizeHitPath()` 与 `openHit()` 都无条件 `replace(/\//g, "\\")`，macOS 上即便搜到结果也打不开 → 统一走 `toNativeFsPath`；引擎类型加 `spotlight`，卡片标题 / 状态 / 配置文案按能力字段或 live status 切换 |
| i18n 5 语言 | `engines.spotlight` 新增；`everythingConfigHint` / `fallback*` / `fallbackBlacklistNote` 改成 `{engine}` 插值；`syntaxHint` 去掉 Everything 专属说法；黑名单 placeholder 补 POSIX 示例。`settingsUi.ts` 的 `core.everythingPreferred` 隐藏条件由 `!caps.globalSearch` 改为 `caps.globalSearchEngine !== "everything"`，避免 macOS 因 `global_search=true` 误显示 Windows 开关 |

**本机实机发现**（`mdfind` 行为逐条实测，不是查文档推断）：

| 探测 | 结果 |
|---|---|
| `/usr/bin/mdfind` / `mdutil -s /` | 二进制存在、`Indexing enabled`；`mdfind -name "a-file-manager"` 约 0.95s 返回 |
| `-name "Info.plist"` / 中文「终端」 | 正常返回绝对路径 |
| `-name "a(b)!?"` / `"readme*"` | 不加引号被当查询表达式，返回无关结果 / 空 → 采用整词引号字面化 |
| `-name ""` | 打印 `Failed to create query for ''` 却 exit 0 → 空查询短路 |
| `-0` | NUL 分隔可用；stderr 的 `[UserQueryParser] Loading keywords…` 是日志，不是错误 |

**契约**：新增 `scripts/check-global-search-provider.mjs`（注册 `test:global-search-provider` 并纳入 `npm test`）：断言中立类型与 3 组入口、两端适配器都声明引擎、macOS 不得声称 Everything、Windows 不得依赖 `mdfind`、`discovery/mod.rs` 必须经适配层选 backend 且不得新开 `#[cfg]`、`platform_backend.rs` 代码行不得出现 OS 集成（注释用 `codeOnly` 排除）、service 必须路由 Spotlight 且跳过空查询估算、黑名单不得再本地改写分隔符、`DiscoveryView` 不得自行 `/\//g → \\`、TS/Rust 能力字段与 5 语言 `{engine}` 插值逐一对齐。

| 验证 | 结果 |
|---|---|
| `cargo check`（默认 / `--no-default-features`） | 通过，均 0 警告 |
| `cargo test --lib` | **310 passed / 0 failed / 2 ignored**（原 301；+9 个 Spotlight / 引擎 / serde / 路径断言） |
| Spotlight 实机用例 `cargo test --lib live_spotlight_query -- --ignored` | ✅ 真实返回 `Info.plist` 绝对路径 |
| `npm test` 全量 / `npm run build` | exit 0 / 通过（`vue-tsc` 类型检查通过） |
| `test:global-search-provider` / `test:i18n` / `test:shortcuts` / `test:discovery-search` / `test:settings-ui-regroup` / `test:fs-path` / `git diff --check` | 全通过 |
| 改动过的 Rust 文件 `rustfmt --check`（`skip_children=true`） | exit 0；用该配置避免 7.20 的连带格式化无关子模块 |

**边界（勿误判为完成）**：`mdfind` 没有分页参数，实现是把全部结果读进内存再 skip/take，超大结果集（数万条）的内存 / 耗时未压测，也没设查询超时；Spotlight 索引关闭或正在重建时结果可能为空，本轮 `probe` 只检查二进制存在、没查 `mdutil` 状态；`-name` 的整词引号使 `*` 等通配符变成字面量（与 Everything 语义不同，已在 `syntaxHint` 提示）；搜索页的引擎卡片、黑名单回显、结果双击跳转仍需用户在 GUI 实机确认（本轮只跑到 IPC 契约 + 实机 `mdfind`）；`discovery/fallback.rs` 仍是恒空 stub（降级路径未实现）；Windows adapter 的**类型检查仍待 Windows CI**（同 7.20 限制）。仍未归位的是图标 / 废纸篓 / 剪贴板三个 provider、静态 bool 能力层（6.5 第 4 条）、`DriveInfo.letter` 等 IPC Windows 名词、进程树清理、原生菜单与其余 P2 i18n 名词（**7.22 回收了其中 24 组主文案，但仍有残留，见下**）。

### 7.22 平台名词 / 快捷键文案平台化（第 17 轮）

**触发**：用户在 7.21 之后问「下一步」，并在四个候选中选了「P2 平台名词/快捷键文案」。7.8 只让**动态生成**的快捷键标签走 `platformShortcutLabel`，5 语言 locale 文件里写死的 `Ctrl+Shift+P` / `Alt+Enter` / `此电脑` / `回收站` / `资源管理器` / `%APPDATA%\...` 完全没被覆盖。

**先过 6.5 第 1 / 6 条门禁**：

| 缺口 | 四选一 | 依据 |
|---|---|---|
| 平台文案覆盖机制 | **自写薄适配**（`platformTerms.ts` 的纯函数合并） | vue-i18n 的 `messages` 是静态对象，没有「按宿主覆盖部分 key」的内建机制；7.8 的 `platformShortcutLabel` 只能处理运行时拼出的标签，够不到 locale JSON 里的整句文案。无库可代劳，故自写且限定为纯函数 |
| 快捷键写法 | **复用既有格式约定** `⌘+⇧+P` / `⌥+Enter`，用契约脚本绑定 | locale JSON 不能调函数；契约断言覆盖值含 `⌘` / `⌥` 且不含 `Ctrl+` / `Alt+`，并锁住 `registry.ts` 的刷新 chord |
| 宿主判断 | **复用** `src/config/platform.ts` 的 `isMacPlatform()` | 6.5 第 6 条：组件层零新增 `isMacPlatform()` 分支，判断只在 `i18n/index.ts` 一处 |
| 默认路径示例 | **复用** `tauri.conf.json` 的 identifier + `directories::BaseDirs::config_dir()` | macOS 实际为 `~/Library/Application Support/com.a-file-manager.app/.agents/skills`；`ai/agents.rs::expand_agent_path` 已在 7.20 委托 `to_native_fs_path`，文案与运行时一致 |

**设计**：Windows 正文**一字不改**（零回归），每个 locale 顶层新增 `platform.macos`：一张 `dotted.key → macOS 文案` 的扁平表。`i18n/index.ts` 在 `createI18n` 之前调用 `localizeCatalog()` 去掉内部段并按宿主合并；`hydratePlatform()` resolve 后由 `applyPlatformTerms()` 用 `setLocaleMessage` 重合并（响应式，无需 reload）。能力未知时保持 Windows 文案。所有 `t()` 调用点零改动。

| 落点 | 内容 |
|---|---|
| 5 语言 `locales/*.json` | 新增 **24 个 key** 的 macOS 覆盖：系统文件管理器 7、废纸篓 6、此电脑 4、Skill 库默认路径 1、快捷键 4、`PC → Mac` 2。中文「资源管理器」→「访达（Finder）」、「回收站」→「废纸篓」、「此电脑」→「这台 Mac」；英文 `Recycle Bin`→`Trash`；日/德/法各自本地化（德语/法语的 Papierkorb / Corbeille 本就跨平台，保留）。法语冠词 `l’Explorateur`→`le Finder`、日语 Finder 与助词间空格、根目录示例换成 `~/Library/Application Support/...` 均逐条处理 |
| `src/i18n/platformTerms.ts` | `macosOverrides()` / `localizeCatalog()` 纯函数：深拷贝、删内部段、只覆盖**已存在**的字符串叶节点（坏 key 不会凭空造文案）、不改 import 的 JSON 模块 |
| `src/i18n/index.ts` / `src/main.ts` | 保留 catalog 的推断形状（否则 vue-i18n 会把 `global.locale` 推成 string，`vue-tsc` 报错），导出 `applyPlatformTerms()` 并在 bootstrap 的 `hydratePlatform().then(...)` 调用 |
| `scripts/check-platform-terms.mjs` + `platform-terms-checks.ts` | 注册 `test:platform-terms` 并纳入 `npm test`。静态断言 24 key 集合 / 5 语言一致 / 每个 key 有正文可覆盖 / macOS 文案无 Windows 残留 / ⌘⇧P 与 ⌥ 写法 / 默认路径 / `chrome.refreshTitle` 有意不覆盖且 chord 仍是 F5 / 合并挂在主链路 / `platformTerms.ts` 不碰 Tauri 与宿主判断 / `platformShortcutLabel` 未被绕过；纯函数单测覆盖 win / mac / 不修改原对象 / 坏 key / 真实 5 语言 catalog 全量合并 |

**有意不改（写清理由，勿误判为遗漏）**：`commands.openNavigator`、`nav.navigator`、`navigator.title`、`startupPage.options.navigator`、`previewPane.description` 等说的是 **AFM 自己的 Navigator 视图**，不是系统文件管理器，改成 Finder 反而错；`favorites.osVerb` / `settings.favoritesMeta` 是 Windows 右键动词，macOS 已由 `SettingsView.vue:70` 的能力门控隐藏；`health.deps.items.webview2.title` 是 Windows-only 依赖（macOS 走 `webview` id + 后端 title）；`chrome.refreshTitle` 的 `F5` 与 `registry.ts` 里真实的 `key: "f5"` 一致，改文案会撒谎。

**验证**：`npm test` 全量 exit 0（含 `test:platform-terms`、Rust **310 passed / 0 failed / 2 ignored**）；`npm run build`（`vue-tsc --noEmit` + vite）通过；`git diff --check` 通过；5 个 locale 各只 +28 行，Windows 正文 diff 为 0。运行时单测 `platform copy merge checks OK`，契约 `24 keys × 5 locales, 108 non-identical`。

**边界（勿误判为完成）**：本轮**未启动 GUI**，macOS 上实际显示的「访达 / 废纸篓 / 这台 Mac / ⌘+⇧+P」需用户实机确认；P2 表第 97 行列的 `C:\Windows`、`\\server\share`、`WebView2 Runtime`、AI 文案里的 `APPDATA` 概念词、`blacklistPlaceholder` / `installPathPlaceholder` / `networkEmpty` 示例路径、extensions 里日语「この PC」仍未平台化；`deps_probe` 后端的依赖标题 / 说明是**硬编码中文**、不走 i18n；F 键被 macOS 媒体键占用、原生菜单栏 / Cmd+Q / Cmd+W、字体与 `color-mix()` 仍按原 P2 排期。7.21 结尾列的废纸篓 TCC / 列表还原、进程树清理、三个 provider 归位、静态 bool 能力层、`DriveInfo.letter` 均**未动**。

### 7.23 P1 收尾：进程树清理 + 图标 provider 归位（第 18 轮）

**触发**：用户在 7.22 之后的两项候选中选了「P1 收尾」（进程树 + 图标 provider）。7.22 的文档边界与 P1 表第 87 行都明确：`win_process::kill_process_tree` 非 Windows 是空实现，rclone 超时后子进程会泄漏；sidecar 默认命令写死 `python`；图标实现散在 `shell_icons` 的 `#[cfg(windows)]` 里，macOS 没有内容缩略图。

**先过 6.5 第 1 / 6 条门禁**：

| 缺口 | 四选一 | 依据 |
|---|---|---|
| 进程树终止 | **复用**已引入的 `sysinfo 0.40.3`（`features` 加 `system`，`Cargo.lock` 仅 +11 行） | `Process::parent()` / `kill()` 已覆盖 OS 细节；只自写「按根收集后代、逆序杀、防环」这一层语义，不重新实现进程枚举。Windows 的 `taskkill /F /T` 原样保留 |
| sidecar 解释器 | **复用**新增的能力层契约 `python_command()` | 纯字符串差异，不需要新 crate；Windows 仍返回 `python` |
| 文件类型图标 | **复用**已装的 `file_icon_provider`（NSWorkspace / freedesktop），只做归位 | 6.4 矩阵第 306 行已确认该 crate 覆盖 macOS 图标，重复实现只会引入 ObjC 细节 |
| 系统内容缩略图 | **降级**，保持显式 `Err` + 类型图标兜底 | 本机实测 `/usr/bin/qlmanage -t` 返回 `sandbox initialization failed: Operation not permitted`（退出码 255，无输出文件）；`QLThumbnailGenerator` 需异步 ObjC run loop + 新依赖。宁可明确降级，也不引入无法在本会话验证的代码；契约禁止静默加 `qlmanage` 调用 |

**改动**：

| 落点 | 内容 |
|---|---|
| `platform/mod.rs` | 新增中立 `RgbaIcon { width, height, pixels }`；trait **20 → 24 个方法**（`kill_process_tree` / `python_command` / `file_icon` / `system_thumbnail`），OS 图像类型不越过适配层 |
| `platform/windows.rs` | 从 `shell_icons` **逐字搬迁** COM STA、`HBITMAP→RGBA`、`IShellItemImageFactory` 的 cache-only → handler 两次尝试；进程树仍走 `taskkill /F /T /PID`（经 `win_process::command`，CREATE_NO_WINDOW 不变）；`python_command()` 返回 `python` |
| `platform/macos.rs` | `sysinfo` 构建 PPID 图 → 纯函数 `collect_process_tree()` 深度优先收集（带 visited 防环）→ **叶到根** `Process::kill()`，避免父先死造成孤儿；`file_icon_provider` 出 NSWorkspace 图标；`system_thumbnail()` 返回明确错误；`python_command()` 返回 `python3` |
| `win_process.rs` / `tool_runtime/sidecar.rs` | `kill_process_tree()` 变薄委托，零 `#[cfg]`；sidecar 默认贡献与两处测试断言改用 `platform::python_command()`，Windows 语义不变 |
| `shell_icons/mod.rs` | 删掉全部 Win32 imports、`ensure_com_sta`、`hbitmap_to_rgba` 与两个 cfg 版 `system_thumbnail_data_url`，只保留中立的 RGBA→PNG 编码 + 缓存 + `thumbnail → file_icon` 兜底；业务模块不再出现 OS 图像类型 |
| 静态契约 | `check-shell-icons.mjs` 与 `check-no-console-flash.mjs` 的断言**改指向适配层但未放宽**：SIIGBF 缓存优先、禁止自解码、`taskkill /T`、`child.kill()`、CREATE_NO_WINDOW 全部保留；新增「`shell_icons` 不得含 `use windows::` / `HBITMAP`」「两端必须实现 4 个新方法」「macOS 不得调用 `qlmanage`」「python/python3 不能被写回业务模块」 |

**本机实测**：`qlmanage -t -s 64 -o <tmp> <png>` → exit 255、stderr `sandbox initialization failed: Operation not permitted`、输出目录为空（因此缩略图降级，非文档推断）。

**验证**：`cargo check`（默认 / `--no-default-features`）均 0 警告；`cargo test --lib` **332 passed / 0 failed / 2 ignored**（上一轮 310，+22）；`npm test` 全量 exit 0（含 `test:shell-icons` / `test:no-console-flash` / `test:platform-terms`）；`npm run build`、`rustfmt --check`（`skip_children=true`）、`git diff --check` 全通过。

**边界（勿误判为完成）**：Windows 适配器的**类型检查仍待 Windows CI**（本机无 `x86_64-w64-mingw32-gcc`，同 7.20/7.21；本轮只做了 `rustfmt --check` 语法级校验）；macOS 内容缩略图仍是**降级**，Quick Look 未在真实 app（非会话沙箱）验证；进程树用 `Process::kill()`（Unix SIGKILL），没有优雅退出 / 超时退避，也没有真实子进程的集成测试（只测了纯 PPID walk 的防环与顺序）；`RgbaIcon → PNG` 编码仍在业务模块，但已是中立像素操作；`PlatformCapabilities` 仍是静态 bool（6.5 第 4 条未完），废纸篓 / 剪贴板两个 provider 未归位，`DriveInfo.letter` 未清洗，原生菜单与 P2 其余项未动。**（7.24 已回收静态 bool 与废纸篓 / 剪贴板 provider；其余边界仍成立。）**

---

### 7.24 能力层 provider 化（6.5 第 4 条，第 19 轮）

**触发**：用户在 7.23 之后两项候选中选了「架构：能力层 provider 化」。6.5 第 4 条要求把 `PlatformCapabilities` 从静态 bool 升级为「由已注册适配器回报」，给 trait 增加图标 / 废纸篓 / 搜索等 provider 方法，并用 OpenDAL 式共享测试替代按平台复制断言。图标（7.23）、搜索（7.21）已在位，本轮补齐废纸篓 / 剪贴板与能力字段推导。

**先过 6.5 第 1 / 6 条门禁**：

| 缺口 | 四选一 | 依据 |
|---|---|---|
| 废纸篓 provider | **复用**现有 `trash` crate + Shell.Application 后端，只加 trait 与入口 | 两端实现已存在且经过验证；无 Windows 编译器，搬迁 ~300 行 PowerShell 只会带来行为漂移风险 |
| 剪贴板 provider | **复用**现有 CF_HDROP 后端，只加 trait 与入口 | 同上；macOS 没有对应后端，正确做法是「不注册 provider」而非造一个假 stub |
| 能力字段 | **自写**一处共享推导 `apply_provider_capabilities()` | 没有现成 crate；这是本项目的 IPC 契约（TS/Rust 字段同步由 `shortcut-checks.ts` 守住），必须显式定义 |

**改动**：

| 落点 | 内容 |
|---|---|
| `platform/mod.rs` | trait 新增 `trash_provider()` / `clipboard_provider()`；新增中立 `TrashProvider`（delete / list / restore / `supports_list_restore`）与 `ClipboardProvider`（set / get / clear）两个子 trait；把 `ShellClipboard` / `RecycleBinItem` 两个 DTO 提升为平台中立类型（serde camelCase 不变） |
| `platform/mod.rs` | 新增 `apply_provider_capabilities()`：`trash_delete` / `trash_list_restore` / `shell_clipboard` / `global_search` 四个字段**从实际注册的 provider 推导**，不再是两端手写的 `caps.x = true`；新增 `delete_to_trash` / `trash_list` / `trash_restore` / `clipboard_{set,get,clear}` 中立入口，缺失 provider 时按原有语义降级（set → Err、get → 空、clear → Ok） |
| `platform/{windows,macos}.rs` | 两端显式注册 provider：Windows 注册 `NATIVE_TRASH` + `NATIVE_CLIPBOARD`，macOS 只注册 `NATIVE_TRASH`；`capabilities()` 末尾统一调用 `caps.apply_provider_capabilities(self)`，删掉手写的能力 bool；macOS 的 `clipboard_provider()` 返回 `None` |
| `file_ops/{mod,clipboard,recycle}.rs` | 所有命令/删除入口改为调用 `platform::*` 中立函数；OS 实现保留为 provider 后端并在文件头注明；删掉非 Windows 的剪贴板 stub（降级逻辑已集中到 `platform::clipboard_*`，避免两条降级路径） |
| `config/platform.ts` | `PlatformCapabilities` / `NEUTRAL_PLATFORM` 加 `shellClipboard` 字段，与 Rust 结构严格同步 |
| 共享测试 | `assert_provider_capability_contract(&dyn Platform)` 由 `platform/mod.rs` 与两端测试模块**同一份函数**调用；断言能力位等于 provider 存在性、搜索位等于引擎非 None、缺失剪贴板 provider 时三条入口均不 panic |
| 静态契约 | 新增 `check-provider-capabilities.mjs`（并纳入 `npm test`）：断言 trait / 子 trait / 共享推导 / 中立入口齐全、两端不得手写 provider 能力位、命令层不得直接调 OS 后端、DTO 归属平台、TS 字段同步；注释先剥离再匹配，避免 7.21/7.22 的注释误判 |

**验证**：`cargo check`（默认 / `--no-default-features`）0 警告；`cargo test --lib` **334 passed / 0 failed / 2 ignored**（+2：共享契约在 `current()` 与 macOS 适配器各跑一次；Windows 适配器在 CI 上会跑第三份）；`npm test` 全量 exit 0（含新契约）；`npm run build` / `rustfmt --check` / `git diff --check` 全绿。

**边界（勿误判为完成）**：provider 的**注册关系在两端各写一次**（`Some(&NATIVE_TRASH)` / `Some(&NATIVE_CLIPBOARD)`），还没有做到「插件式动态注册」——但 6.5 第 4 条要求的是「由已注册适配器回报」，本轮已满足；`NativeTrash` / `NativeClipboard` 调用的 OS 实现**仍在 `file_ops/*`**，只是入口收归平台层（理由：无 Windows 编译器，避免搬迁 600 行 Win32 / PowerShell 代码造成行为漂移）；`shellClipboard` 字段目前只上报、前端还没有用它隐藏粘贴项；Windows 适配器类型检查仍待 CI；`DriveInfo.letter` 清洗、原生菜单、静态事实类能力（注册表自启 / WebView2 等）仍按原排期。

---

### 7.25 GitHub CI 双平台 + macOS 打包 job（第 20 轮）

**触发**：用户问「将来要实现 GitHub CI 打包，Mac 包可以吗」，并选了「CI 矩阵 + Mac 打包 job」。结论是**可以**，但本轮只能交付「流水线正确 + 静态契约守住」，真实 `.dmg` 必须在 GitHub 的 macOS runner 上才能产出（本机 `hdiutil` 挂载卷操作被沙箱拒绝，见 7.9）。

**先过 6.5 第 1 / 6 条门禁**：

| 缺口 | 四选一 | 依据 |
|---|---|---|
| CI 双平台 | **复用**现有 `npm test` 全量链与 `dtolnay/rust-toolchain` / `swatinem/rust-cache` | 不为 Mac 复制一套测试步骤；矩阵只切 runner 与 target，两条分支跑同一条链 |
| Mac 打包入口 | **复用**`build-flavor.mjs` 的 flavor / Cargo feature 逻辑，只按 `process.platform` 切 bundle 集合并转发 `--target` | Windows NSIS 行为逐字保留；不新写构建脚本 |
| 签名 / 公证 | **降级并显式写明** | 没有 Developer ID / `APPLE_*` secrets，本机无法验证；契约断言 workflow **不引用**任何 `APPLE_*`，保持 opt-in |

**改动**：

| 落点 | 内容 |
|---|---|
| `.github/workflows/ci.yml` | 单 job 双 runner 矩阵（`windows-latest` + `x86_64-pc-windows-msvc`、`macos-latest` + `aarch64-apple-darwin`）；全局 `shell: bash` 替掉 PowerShell；缓存键 `ci-${{ matrix.os }}-${{ matrix.rust_target }}`；两端跑 `npm run build` + `npm test` + `cargo check --target`；WebView2 / `test:packaging-flavors` **条件化到 Windows**（不是删除） |
| `.github/workflows/release.yml` | `prepare` 新增 `mac_matrix` 输出（由 `macReleaseMatrix(packs)` 生成，feature-pack-only 时为空并跳过）；新增 `build-app-macos` job：原生双架构 × 两个 flavor = 4 个构建，构建 `app,dmg`、把 `.app` 用 `ditto -c -k --keepParent` 压成 zip、`gh release upload --clobber` 挂到同一 Release；`finalize` 的 `needs` 与结果检查纳入该 job；release notes 加 Mac 资产表与「未签名需右键打开」说明 |
| `scripts/build-flavor.mjs` | 新增 `--target <triple>`；`--bundle` 时按宿主选 bundle（win32→`nsis`，darwin→`app,dmg`，其他→`appimage`）；Windows 默认 `nsis` 一字未改 |
| `scripts/release-assets.mjs` | 新增 `MAC_ARCHES`、`tauriDmgFileName`、`dmgAssetName`、`appZipAssetName`、`macReleaseMatrix`；Tauri 源名含空格 `A File Manager_<ver>_<arch>.dmg`，GitHub 资产名沿用「空格→点 + flavor 后缀」约定 |
| `scripts/check-ci-macos.mjs` | 新增静态契约并纳入 `npm test`：双 runner / 缓存维度 / Windows 断言门控 / Mac job 的 target 与资产命名 / `finalize` 依赖 / 不引用 `APPLE_*` / 命名函数精确值；`test:macos-bundle` 也纳入全量链 |
| `docs/macos-port-research.md` | P0-8、3.3 清单第 1/11/12/14/15 项标注本轮状态与偏离理由 |

**本机验证**：两个 workflow 用 Ruby Psych 解析通过；逐段抽出全部 `run` 脚本做 `bash -n`，27 段全 OK；`npm test` 全量 exit 0（334 Rust 用例 + 新增契约）；`git diff --check` 通过。**不代表 CI 已跑过**——GitHub Actions、实机 `.dmg` 生成、跨架构交叉编译都还没执行。

**边界（勿误判为完成）**：

- 产物是**未签名 / ad-hoc**，没有 `signingIdentity` / entitlements；用户首次打开需右键 → 打开。（7.26 已搭好 opt-in 签名公证骨架，但签名路径尚未在真实 CI 上验证过，详见 7.26。）
- `rust-toolchain.toml` 未加 apple targets（由 workflow 注入），本机开发者不会被迫下载；若将来要本地交叉编译需自己 `rustup target add x86_64-apple-darwin`。
- `prepare-release.mjs` / `build-release-artifacts.mjs` 的 dry-run 输出还没列出 Mac 资产（release.yml 的 notes 与实际上传已覆盖）；`check-multi-pack-release.mjs` 尚未断言 Mac 资产。
- `macos-latest` 目前是 Apple Silicon runner，交叉编 `x64` 依赖 Xcode 提供的 linker；这条路只有 GitHub Actions 能验证。Tauri 的 dmg 文件名按 v2 约定（`<产品名>_<版本>_<架构>.dmg`）推导，未在真实产物上核对。
- `ci.yml` 的 macOS runner 跑 `npm test` 包含 `test:rust`（334 用例）与 Provider 共享测试；Windows runner 额外跑 `test:packaging-flavors`，两边不再共享 PowerShell 步骤。

---

### 7.26 签名公证 opt-in 骨架（第 21 轮）

**触发**：v0.3.0 上线后用户实测 macOS 弹 Gatekeeper「Apple 无法验证…是否包含恶意软件」（只有「完成 / 移到废纸篓」），确认未签名包无法双击打开。用户选择「先搭 CI 签名骨架」——配好 secrets 就签、没配保持 ad-hoc。

**机制（查 Tauri CLI 2.12.4 源码与官方文档确认）**：

- `hardenedRuntime` 在 config schema 里**默认 true**，不必显式写；`tauri.conf.json` 保持无 `signingIdentity`，签不签完全由环境变量决定。
- Tauri CLI 能从 `APPLE_CERTIFICATE`（base64 `.p12`）**自动推断签名身份**，无需手填 `APPLE_SIGNING_IDENTITY`（2.12 的 changelog 明确支持）。
- 公证二选一：`APPLE_ID` + `APPLE_PASSWORD` + `APPLE_TEAM_ID`（app 专用密码，Team ID 现在必填），或 `APPLE_API_KEY` + `APPLE_API_ISSUER` + `APPLE_API_KEY_PATH`。本轮用前者。
- 凭证齐全时 `tauri build` 自动完成 codesign → notarize → staple，无需手写 `xcrun notarytool` / `stapler`。

**实现**：

| 文件 | 改动 |
|---|---|
| `.github/workflows/release.yml` | `prepare` 读取 5 个 `APPLE_*` secret、只在全齐时输出 `mac_signing=true`（部分提供视为未配置，不硬失败）；`build-app-macos` 新增条件化「Import Apple Developer Certificate」步骤（临时 keychain，`set-key-partition-list` 放通 codesign）；build 步骤把凭证传给 Tauri，**ad-hoc 路径显式 `unset` 全部 `APPLE_*`**（Tauri 把「设置了但为空」当成已配置，会尝试解码空证书而失败）；release notes 按 `mac_signing` 出「可直接双击」/「需右键打开」两种文案。 |
| `scripts/check-ci-macos.mjs` | 把旧的「禁止任何 `secrets.APPLE_`」断言改为 opt-in 契约：5 个 secret 都接线、导入步骤 gated、partition-list 存在、ad-hoc 分支 unset、notes 双文案；注释剥离后匹配，避免注释满足断言。 |

**需在 GitHub 仓库配置的 Secrets**（Settings → Secrets and variables → Actions）：

| Secret | 值 |
|---|---|
| `APPLE_CERTIFICATE` | `openssl base64 -A -in cert.p12` 的输出 |
| `APPLE_CERTIFICATE_PASSWORD` | 导出 `.p12` 时设的密码 |
| `APPLE_ID` | Apple ID 邮箱 |
| `APPLE_PASSWORD` | App 专用密码（不是登录密码） |
| `APPLE_TEAM_ID` | Membership Details 里的 Team ID |
| `KEYCHAIN_PASSWORD`（可选） | 临时 keychain 密码，缺省用固定值即可 |

**本机验证**：`npm test` 全量 exit 0（334 Rust 用例 + 新契约）；两个 workflow YAML 解析通过；逐段 `bash -n` 全 OK；`git diff --check` 通过。

**边界（勿误判为完成）**：签名路径**从未真实跑过**——本机 `security find-identity` 为 0，无 Apple 开发者账号，签名/公证只能在配好 secrets 的 GitHub Actions 上验证；填完 secrets 后应打一个新 tag 走 CI，检查 `security find-identity` 输出、Tauri 的 notarize/staple 日志，并下载 dmg 实机双击确认 Gatekeeper 不再拦截。尚未支持 App Store Connect API key 认证路径（`APPLE_API_KEY` 三件套），只做了 app 专用密码路径。临时 keychain 用 `RUNNER_TEMP`，不污染 runner 登录 keychain。`KEYCHAIN_PASSWORD` 有缺省固定值，若追求严格可将其也设为 secret。

---

### 7.27 无证书路径收口：显式 ad-hoc + 包内打开帮助（第 22 轮）

**触发**：用户确认「没有配置 Apple ID / password 就自动用无证书方式打包」，并反馈 v0.3.0 用右键→打开也打不开。通过 GitHub API 下载真实 `A.File.Manager_0.3.0_x64.app.zip` 实测：

```
codesign -dv --verbose=4 'A File Manager.app'
  → code object is not signed at all
spctl -a -vvv 'A File Manager.app'
  → rejected / source=no usable signature
```

也就是说 7.26 的无证书路径虽然能出包，但产物是**完全未签名**，不是 ad-hoc。Apple 的支持文档与近期实践都表明：右键→打开只对部分系统版本/部分拦截形态有效；macOS 15/26 上「Apple 无法验证…」常常只给「完成 / 移到废纸篓」，此时必须走「系统设置 → 隐私与安全性 → 仍要打开」或移除 `com.apple.quarantine`。

**实现**：

| 落点 | 内容 |
|---|---|
| `.github/workflows/release.yml` | 保留 7.26 的 `mac_signing` 门控：5 个 `APPLE_*` 任一缺失/为空 → `mac_signing=false`，build 里 `unset` 全部变量，绝不因半套凭据失败；新增明确日志 `Apple signing credentials incomplete or absent — macOS builds use ad-hoc fallback`。构建后新增 `Finalize unsigned macOS bundle` 步骤（仅 `mac_signing != true` 时运行），调用 7.27 收尾脚本。Release Notes 改为可操作的三步：系统设置仍要打开 / `xattr -dr com.apple.quarantine` / 右键打开（注明仅部分版本有效，不要点「移到废纸篓」）。 |
| `scripts/finalize-macos-adhoc.mjs` | 新增。`codesign --force --deep --sign - --timestamp=none` 给 `.app` 显式 ad-hoc 签名并严格校验；重建 Tauri 生成的 DMG：把签名后的 `.app`、根目录 `打开帮助.txt`、`/Applications` 拖放目标放进临时 staging，用 `hdiutil create ... -format UDZO -o <dmg>` 覆盖输出，再 `hdiutil verify`。签名/公证路径明确不调用此脚本，避免破坏 Developer ID 签名。 |
| `src-tauri/resources/macos-open-help.txt` | 新增包内帮助：来源核验、系统设置仍要打开、`xattr` 命令（含拖 App 到终端的用法）、右键打开的适用边界、为何需要 Developer ID + 公证、文件在 DMG/`.app`/Release 中的位置。 |
| `src-tauri/tauri.conf.json` | `bundle.resources` 加入帮助文件，签名构建也会把它一起签进 `.app`；ad-hoc 构建由收尾脚本保证它存在。 |
| `scripts/check-ci-macos.mjs` | 新增 2c 契约：回退日志、收尾步骤门控、帮助文件同时出现在 bundle resources / DMG / Release 资产、finalizer 的 codesign + `hdiutil verify` + `-o` 输出、帮助文案包含「仍要打开 / xattr / 右键 → 打开 / 不要点移到废纸篓」等关键指引。 |

**本机演练（真实资产，非模拟）**：用 v0.3.0 的 x64 `.app` 复制成 Tauri 产物布局，先造一个未签名 DMG，再跑收尾脚本。结果：`codesign -dv` 显示 `Signature=adhoc`、`TeamIdentifier=not set`，`codesign --verify --deep --strict` 输出 `valid on disk` / `satisfies its Designated Requirement`；`hdiutil verify` 校验和 VALID；挂载后根目录可见 `打开帮助.txt`、`A File Manager.app`、`Applications` 软链。演练中还发现 `hdiutil create` 用位置参数指定输出时在本机产生瞬时文件、脚本随后读不到，已改为显式 `-o` 并写入契约。

**边界（勿误判为完成）**：ad-hoc 只提供完整性印章，**不能替代 Developer ID + 公证**，double-click 仍可能被 Gatekeeper 拦；它改善的是「无 usable signature」状态，并让包内明确携带可操作的帮助。7.27 尚未在新 tag 的 GitHub Actions 上真实跑过；签名/公证路径仍未验证。帮助里的命令按「拖 App 到终端」设计，避免用户手敲路径出错。

---

## 7.24 · `6c73fa2..2f17ab4` 新提交的增量 macOS 适配（2026-11-23）

拉取 master 后逐项审查本批新提交，发现 6 处 Mac 适配点，本轮处理 5 处 + 1 处核查后无需改动。
这些是**新增提交引入的增量缺口**，不属于前面各节的存量移植清单。

| # | 来源提交 | 缺口 | 处理 |
|---|---|---|---|
| 1 | `e005e23`/`106dbc2` LC-007 逐项错误弹窗 | `classify_transfer_error` 只认 Windows 系统码且未知码提前 `return None`，Mac 的 EBUSY/ENOSPC/EPERM 落空 → 弹窗不弹 | `class_from_os_code` 按平台分表（Windows 5/13/32/33/36/39/112；POSIX 1/13/16/26/28/69），未知码继续走英文回退；新增 `classify_transfer_error_maps_posix_errnos` |
| 2 | `ba928fe` 网络节点（ADR 0026） | 非 Windows 分支用 `mount -p`，本机实测 `illegal option -- p`；parser 期望 tab 四列，Mac 输出 `dev on /path (fstype, …)` → 节点恒空 | 改 bare `mount`；`parse_mount_line` 兼容两种格式（含带空格挂载点）；新增 `parse_mount_network_accepts_macos_bare_mount_output`；SM-28 |
| 3 | `85d52f1` NM-003/NM-007 文件名校验 | Win32 保留名 / 尾点空格 strip / 非法字符集无平台门控，误伤 Mac 合法名（`CON`、`a.`、`a:b`） | `validate_file_name` 按平台分支：Windows 保持原规则，POSIX 仅拒绝 `/`、`.`/`..`、空名；两个 Windows 测试加 `#[cfg(windows)]`，新增 `validate_name_keeps_posix_legal_names` |
| 4 | `fc1d0d4` NM-004 大小写改名 | `is_same_entry` 非 Windows 用字节比较 + `platform/macos.rs` 静态 `case_insensitive_paths=false` → APFS 上 `a.txt→A.txt` 误报冲突 | Unix 分支改 **(dev, inode)** 身份比较（对大小写敏感/不敏感 APFS 均正确）；静态能力位保留 `false` 并加注说明；新增 `rename_case_only_uses_inode_identity_on_unix` |
| 5 | `4e902d9` Alt+0…9 工作组切换 | `chord.ts` 用 `event.key` 匹配，macOS ⌥+数字产生字形（⌥0=`` ` ``、⌥1=`¡`）→ 快捷键失效，而 locale 已写 `⌥+1–9` | 新增 `digitKeyFromEventCode`，单数字 chord 用 `event.code`（`Digit0…9`/`Numpad0…9`）匹配；`chordFromEvent` 同步；`shortcut-checks.ts` 补 3 类断言 |
| 6 | `5a574a7` Alt+单击复制标签 | 审查 `AppShell.vue::onTabClick`：`altKey` 在 macOS WKWebView 正常触发，右键菜单文案 `chrome.duplicateTab` 不含 Windows 修饰键 → **核查后无需改动** | 不改代码；在 GUI 冒烟清单平台前置对照表记录 |

**仍然未覆盖 / 有意退化**（本轮未改，避免扩大范围）：
- RustDesk 远程文件剪贴板（ADR 0027）：`#[cfg(windows)]`，Mac `clipboard_provider()` 返回 None，优雅退化不 panic。
- undo 删除恢复（UN-004）：`supports_list_restore()=cfg!(windows)`，能力位已声明 false，Mac 引导用 Finder 废纸篓。
- QuickLook 命名管道探测：Windows-only；Mac 走原生 `preview_provider()`。
- 外置卷 `.Trashes` 探测：Mac 对所有本地非 UNC 路径返回可回收并交给 `trash`，exFAT 卷的实际行为需 GUI 实测（SM-01 已标注预期差异）。
- 未挂载服务器的 Bonjour / WS-Discovery 发现：Mac 不适用，仅列已挂载共享。

**验证**：`cargo test --lib`（相关模块全绿）；`node scripts/shortcut-checks.ts` 通过；`npm run build` / `npm test` 全链结果见本次会话记录。基线文档同步见 `file-ops-scenario-baseline.md`、`external-behavior-baseline-coverage.md`、`qa/gui-smoke-checklist.md`、`qa/smoke-test-mapping.md` 与 ADR 0026。

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
| 全局搜索（7.21） | `src-tauri/src/platform/{mod,macos,windows}.rs`；`src-tauri/src/discovery/{mod,types,backend,service,blacklist,platform_backend}.rs`；`src/views/DiscoveryView.vue`；`scripts/check-global-search-provider.mjs` |

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
- 第 2 节 P0 的编译阻塞判断最初是静态推断，现已由 macOS 实机 `cargo check`（两种 feature）+ `cargo test --lib` + `npm run build` 验证，详见 7.4；**`tauri dev` 的 GUI 验证已在 7.12 完成（窗口正常显示）**，但拖放、TCC 授权、原生菜单等运行时行为仍未逐一确认。
- 工作量是粗估，不含证书申请、Apple 审核/公证排队与前端大规模视觉重构；第 7 节 PoC 实际耗时约 1 小时（含工具链升级与依赖下载），7.7 的第二轮复用改造约半小时。
- 第二轮（7.7）修正了一条环境结论：`test:address-env` 的挂起不是沙箱限制，而是脚本内部 `cargo test` 的网络路径问题（详见 7.7）；该脚本现已通过。
- 第三轮（7.8）只改前端，Rust 侧零改动，因此未重复跑 `cargo check` / `cargo test --lib`；执行验证时本机已有另一个 `tauri dev` 持有 cargo 包缓存锁（1420 端口有 vite），为不干扰该进程而跳过。前端的 ⌘ 行为由 Node 静态断言模拟 UA 验证，**真实 WKWebView 的按键、窗口与菜单行为仍未实机确认**。
