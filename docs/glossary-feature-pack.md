# Feature Pack / 工具管理运行时

可后装的产品附加能力，以及 AFM 对 awam-tool-manager 网关的薄客户端面。与 AI Add-on 的中央仓库分发无关。

包根目录的 `pack.json` 为安装清单（字段见 [`feature-pack-format.md`](./feature-pack-format.md)，样例见仓库 `packs/_template/pack.json`）。

## Language

**Feature Pack**：
可后装的产品附加包（含前端贡献与可选 sidecar）；普通版默认不打包，v1 以本机选择包（zip/目录）经 ExtensionHost 校验清单与权限后启用，不做远程商店。
_Avoid_: 插件（泛称）、扩展（与 ExtensionHost 条目混淆时）、商店包、marketplace 包

**ExtensionHost**：
Feature Pack 的安装、启停与权限壳（UI 入口 `/extensions`）；不负责任意代码热加载，也不充当能力商店；与工具管理运行时的「使用」页分离。
_Avoid_: 插件系统、商店、marketplace、Sigma 扩展宿主

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
