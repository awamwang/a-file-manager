# Agent 启用的三层：`enabled` / `defaultInProjects` / 项目启用集

全局部署只看 Agent Target 的 `enabled`。另增 `defaultInProjects`，用于首次生成某项目的「项目 Agent 启用集」时的默认勾选（缺省时与当时的 `enabled` 对齐）。项目启用集可按 Project Workspace 覆盖，且不回写中央。项目范围的 MCP/Skills 部署只写入该启用集中的 Target。

曾考虑用单一 `enabled` 同时控制全局与项目：无法表达「全局开、某项目关」或反过来。曾考虑每项目强制手选而无中央默认：新建项目成本高。

## 修订（2026-11）：中央 `enabled` 升级为源头开关

Agent Target 是源头：中央 `enabled` 从「仅控制全局部署扇出」升级为全场景闸门——

1. **项目可勾选集合**：项目 AI 区的 Agent chips 只显示中央 `enabled` 的 Target（原来按 `discovered` 过滤；`discovered` 仍保留为探测结果展示，不再作为任何可选项的过滤源）。
2. **Harvest 源**：「从 Agent 导入」（MCP/Skills）的源下拉只列中央 `enabled` 的 Target；后端 scan/import 命令对未启用 Agent 返回 `AI_HARVEST_AGENT_NOT_ENABLED`。
3. **项目部署 fan-out**：`resolve_project_agents` = 项目启用集 ∩ 中央 `enabled`。中央禁用即从项目 fan-out（MCP/Skill 写入、已添加状态扫描的槽位）排除，即使 `.afm/ai-project.json` 仍存有该 id——存储不清洗，中央禁用期间暂存、重新启用后自动恢复勾选。
4. **项目内发现**：`search_project_skills` 只扫描 workspace 根浅层 + 项目启用集各 Target 的 `projectSkillsDir` 真实配置，不再扫描硬编码公共目录（`.cursor/skills` 等）。

语义收紧：原三层允许「中央关、项目开」——本修订后不再可表达（有意收紧，中央是源头）。「全局开、某项目关」仍可通过项目启用集取消勾选表达。
