# Feature Pack / 工具管理运行时

可后装的产品附加能力，以及 AFM 对 awam-tool-manager 网关的薄客户端面。与 AI Add-on 的中央仓库分发无关。

包根目录的 `pack.json` 为安装清单（字段见 [`feature-pack-format.md`](./feature-pack-format.md)，样例见仓库 `packs/_template/pack.json`）。

## Language

**Feature Pack**：
可后装的产品附加包（含前端贡献与可选 sidecar）；普通版默认不打包。安装来源为白名单商店（受批准的 GitHub 仓库/直链，见 ADR 0019）或本机选择 zip/目录（兜底），经 ExtensionHost 校验清单与权限后启用。作为托管依赖时分发与版本归 ExtensionHost（ADR 0020）。
_Avoid_: 插件（泛称）、扩展（与 ExtensionHost 条目混淆时）、自由商店、marketplace 包

**ExtensionHost**：
Feature Pack 的安装、启停、白名单商店与权限壳（UI 入口 `/extensions`）；承载「包 / 分发 / 版本」面，不负责任意代码热加载，也不承载依赖进程的运行时启停（那归托管依赖 / Health，见 ADR 0020）。
_Avoid_: 插件系统、依赖运行时控制面、Sigma 扩展宿主

**工具管理运行时**：
AFM 内对 awam-tool-manager 的薄客户端面：按统一模型展现已注册工具并调用其 capability；一级侧栏独立入口。
_Avoid_: 本机工具运行时、ToolPort、工具商店、MCP 目录、中央仓库、Extensions 主场

**工具管理网关**：
本机上的 awam-tool-manager 权威进程（可先附着已有实例，否则由 Pack 拉起 sidecar）；持有注册表、策略与调用路由。
_Avoid_: AFM 内嵌引擎、中央仓库、MCP 目录

**工具（tool_id）**：
工具管理网关注册表中的一条提供方（kind 为 app / mcp / launcher），由网关权威持有。
_Avoid_: MCP 目录条目、Library Skill、Open tool（IDE 预设）、插件

**Capability**：
某工具可调用的一个动作或子工具；UI 上为「工具 → capabilities → 参数表单」，经网关统一调用。破坏性调用由 AFM 确认后再请求网关放行。
_Avoid_: 命令（命令面板）、uiCommand、后端工具（对外叙事）

**Open tool**：
Core 内硬编码的 IDE/编辑器启动预设（Settings → Open tools）；与工具管理运行时长期并存、互不迁移为对方。
_Avoid_: 工具、Feature Pack、MCP 服务器

## 托管依赖词汇（ADR 0019 / 0020）

**托管依赖 / ManagedDependency**：
awam-file、awam-tool-manager、awam-knowledge 三个**常驻服务**依赖的统一 Core 抽象；一条记录声明两种模式、连接参数与 credential 来源，并持有进程/健康态（与 `InstalledPackRecord` 分离，且与其 1:1）。运行时启停归 Health/deps_probe，包/商店/版本归 ExtensionHost。knowledge-desensitization 不属此列（无常驻进程）。
_Avoid_: 依赖项（仅指 deps_probe 只读行时）、sidecar（仅指 spawn 贡献项时）、Provider、插件、脱敏 pipeline（批处理，非依赖服务）

**config-mode / 连接模式**：
托管依赖的一种模式：连接远程或本机已有实例、不拉起进程。默认入口，现有能力保留；Settings→高级 退化为此模式的「连已有实例」。其远程地址**不受 allow-list 限制**（有意不对称：allow-list 只约束 managed-mode 产物来源）。
_Avoid_: 托管、离线、远程商店

**managed-mode / 托管模式**：
托管依赖的一种模式：由 AFM 拉起/守护**本机自带隔离产物**（R3，产物来自商店下载）。停止/重启只作用于 AFM 自己 spawn 且记录 pid 的进程。
_Avoid_: 附着、远程连接、守护全部实例、共享运行时拉起

**隔离产物 / Self-contained Artifact（R3）**：
每个依赖自带解释器与依赖的隔离包（PyInstaller onefile / 含 `.venv` 的 zip / wheel+launcher），由商店按 release 摘要校验下载。采用 R3 而非共享运行时的根因：file 与 tool-manager 的 mcp 大版本互斥（`>=2.0` vs `<2.0`），且运行时跨 Python 与 Node 两种，无法共用单一环境。前置：各来源仓需先建产出带校验值的 release job。
_Avoid_: 共享 venv、系统 Python 直跑源码、预打包单一 exe（措辞不准确）、自由运行时

**attach-or-spawn / 先附着否则拉起**：
依赖解析优先级：先探 config-mode 实例，健康即附着；不健康或未配置才走 managed-mode 拉起。是带 fallback 的偏好，非互斥开关（沿用 ADR 0013）。
_Avoid_: 二选一模式、互斥开关

**白名单商店 / Store allow-list**：
非自由商店：仅一组受批准的 GitHub 仓库/直链来源，经固定远端的「元清单」列出可装 pack 及隔离产物的 release 指针；产物按其 GitHub Release 附属摘要校验，失败回退本机 zip/目录。仅约束 managed-mode 产物来源。取代 ADR 0014「不做商店」。
_Avoid_: 自由商店、marketplace 投稿、任意远程拉取、约束 config-mode 地址
