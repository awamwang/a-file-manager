# AI Add-on（Skills / MCP）

面向 Agent 的公共库与按项目分发能力：中央仓库收录 MCP 与 Skills，再部署到已启用的 Agent Target（全局或项目相对路径）。不与 ToolPort、Skills Manager 等外部工具对接；外部方案仅作设计参考。

## Language

**Agent Target**：
一种可发现、可启用的 Agent 运行时落点（如 Cursor、Claude Code），携带其全局/项目相对的 Skills 目录与 MCP 配置路径；含 `enabled`（是否参与全局部署）与 `defaultInProjects`（新项目启用集的默认勾选）。
_Avoid_: Agent、IDE、客户端、编辑器

**中央仓库**：
本机 APPDATA 中由 AFM 持有的公共 Skills 库与 MCP 目录；是 AFM UI 管理与分发的唯一权威来源。
_Avoid_: 注册表、商店、marketplace、ToolPort、Skills Manager

**MCP 目录条目**：
中央仓库中的一条 MCP 服务器定义（id、显示名、server JSON）；部署时以 `afm__<id>` 键合并进目标 MCP JSON。
_Avoid_: MCP 服务器、工具、插件

**Library Skill**：
中央仓库中的一个 Skill 包（根目录含 `SKILL.md`）。
_Avoid_: 技能包、插件、prompt

**Harvest（收获导入）**：
用户在 MCP / Skills 面板手动发起：从已配置 Agent Target 的全局路径扫描候选，勾选后快照写入中央仓库；同名 id 跳过。无自动发现、无外部工具对接。
_Avoid_: 同步、镜像、adopt、自动发现

**部署**：
把中央仓库中的 MCP 目录条目或 Library Skill 写入指定范围（全局和/或**某一个** Project Workspace）下、该范围启用集内的 Agent Target 路径。项目默认不启用任何 Skill/MCP，须显式「添加到所选项目」。
_Avoid_: 安装、发布、分发（产品叙述可用「分发」，领域动词用「部署」）

**项目 Agent 启用集**：
某个 Project Workspace 上勾选了哪些 Agent Target 参与该项目的 MCP/Skills 部署；持久化在工作区内 AI Add-on 自管文件（如 `.afm/ai-project.json`），不写入 Core `project.json`。新建/首次初始化时按各 Target 的 `defaultInProjects` 预填，可按项目覆盖且不回写中央。
_Avoid_: 项目 Agent 列表、本地 Agent 配置

**Skill 部署方式**：
将 Library Skill 落到 Agent 路径的策略：默认 Junction（链接）；可在 AI 配置中改为复制。为全局偏好，作用于之后所有 Skill 部署。
_Avoid_: 安装模式、同步模式

**Project Workspace**：
Core FM 中的一个项目工作区目录；项目维度的 MCP/Skills 部署落在其下的各 Agent 项目相对路径。
_Avoid_: 仓库、repo、工作目录
