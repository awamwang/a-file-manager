# Feature Pack / 工具管理运行时

可后装的产品附加能力，以及 AFM 对 awam-tool-manager 网关的薄客户端面。与 AI Add-on 的中央仓库分发无关。

包根目录的 `pack.json` 为安装清单（字段见 [`feature-pack-format.md`](./feature-pack-format.md)，样例见仓库 `packs/_template/pack.json`）。

## Language

**Feature Pack**：
可后装的产品附加包；**一个 pack = UI 贡献 + 后端 sidecar**，单一版本、单一安装/启停（ADR 0022）。普通版默认不打包。后端产物由 **AFM 作为集成方从自有依赖仓源码 in-tree 构建**，经 AFM 自有发布 feed 分发；安装来源为白名单商店（现语义 = AFM 自有发布通道）或本机选择 zip/目录（兜底），经 ExtensionHost 校验清单与权限后启用。进程/健康态归托管依赖 facet（与包记录分离，见 ADR 0020）。
_Avoid_: 插件（泛称）、扩展（与 ExtensionHost 条目混淆时）、自由商店、marketplace 包、把后端塞进 AFM 主进程

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
托管依赖的一种模式：由 AFM 拉起/守护本机自带的产物（由 AFM 集成方自构建；knowledge 内置、file/tool-manager 首启用从自有 feed 拉取，见 ADR 0022）。**运行时按 per-artifact 声明解析（shared-first + 自带优先）**：产物 `runtime: self` 用它自带的解释器，否则用 AFM 的共享运行时。停止/重启只作用于 AFM 自己 spawn 且记录 pid 的进程。自构建产物**跳过 versionCompat 内部门**（构建时锁定版本）。
_Avoid_: 附着、远程连接、守护全部实例、下载第三方 GitHub release 资产、把 spawned_pid/health 并入 InstalledPackRecord

**自带产物 / runtime: self（隔离档）**：
`SidecarContribution` 的一种运行时来源声明：该产物自带解释器与依赖的隔离包（PyInstaller onefile / 含 `.venv` 的 zip / wheel+launcher），spawn 时用它自带的，不碰共享环境。原 R3「每依赖一律自带隔离产物」的默认地位已降为**可选档**——跨仓「统一 Python+mcp 版本」落地后，默认走共享运行时，自带只用于个别无法/不愿对齐版本的产物（逃生口）。仍是 AFM 集成方自构建、登记摘要。
_Avoid_: 系统 Python 直跑源码、预打包单一 exe（措辞不准确）、自由运行时、上游仓各自产 release

**共享运行时 / Shared Runtime**：
由 AFM 统一提供、多个同类产物共用的解释器环境：一套共享 Python 环境（承载 file + tool-manager，前提是二者 Python 与 `mcp` 版本已对齐）+ 一套共享 Node 运行时（承载 knowledge）。Python 与 Node 不能合成一个，故至少两种运行时。`runtime: shared(python|node)` 的产物只含 wheel/源码薄产物，由共享解释器执行。**precedence 规则：自带则用自带，否则用共享。** 前置成本 = mcp 跨大版本迁移 + 共享 venv 的可重定位打包（不能直接 zip 分发）。
_Avoid_: 单一全局运行时（不存在，至少 Python+Node 两套）、把 venv 当可重定位 zip 直接发、用共享 env 跑自带 onefile


**attach-or-spawn / 先附着否则拉起**：
依赖解析优先级：先探 config-mode 实例，健康即附着；不健康或未配置才走 managed-mode 拉起。是带 fallback 的偏好，非互斥开关（沿用 ADR 0013）。
_Avoid_: 二选一模式、互斥开关

**白名单商店 / Store allow-list**：
非自由商店：现语义收窄为 **AFM 自有发布通道**——AFM 作为集成方自构建产物并发布，仅 AFM 受批准来源；不再开放第三方 GitHub 仓库/直链、不再依赖上游 release job（ADR 0022 取代 0019 下载条款）。`config-mode` 不在此约束内。保留本机 zip/目录兜底。
_Avoid_: 自由商店、marketplace 投稿、任意远程拉取、约束 config-mode 地址、下载第三方 GitHub 产物

**集成方构建 / Integrator Build**：
AFM 承担 integrator 角色：用 per-OS CI agent 从各依赖仓源码（submodule 锁 tag/SHA）in-tree 构建产物、维护共享运行时、登记摘要、打进对应 Feature Pack 的 sidecar、经自有 feed 发布；把「协调上游产 release 的跨仓成本」换成「AFM 扛多语言×多 OS 构建矩阵 + CVE 重 pin 重建 + 一次 mcp 跨大版本迁移」。
_Avoid_: 二进制再分发、上游自产 release、AFM 内联跑 Python/Node 源码（指在 AFM 主进程内）

