# Agent Target 路径槽扩展与 AI 目录跳转层

Agent Target 在 Skills / MCP 之外补充三组只读路径槽：`globalSessionsDir`（全局会话/转录目录）、`globalRulesPath`（全局规则/记忆文件或目录）、`projectRulesDir`（项目相对规则目录）。另提供两层跳转能力：单个 Target 的全局路径一键在系统资源管理器打开（`ai_agent_target_open_path`），以及项目工作区内已存在 AI 目录的只读检测（`ai_project_ai_dirs`）。

## 动机

会话与规则目录是排查 Agent 行为、备份与迁移时最常被访问的位置，但藏在 `%USERPROFILE%` 深处，用户只能手工拼路径。Skills / MCP 已有路径槽与部署管线，会话与规则没有——先补「看得见、跳得过去」，不引入写入。

## 决策

1. **schema v3，加字段不改旧字段**：`AgentTarget` 新增三个字符串字段，serde 带默认值，旧配置直接解析。v2→v3 迁移只做一件事：对内置条目按补充预设表种入三个新槽（空才种，非空不覆盖）；迁移一次性完成（按 `schema_version < 3` 门控），用户清空的值不会被回种。
2. **补充预设表与 60 条内置定义分离**：`BUILTIN_EXTRA_PATHS` 独立 const 表（id → sessions/rules/project rules），`builtin_target` 与迁移各自查表。避免为 3 个新槽改动全部内置定义条目；未收录的 Agent 三个槽为空，用户可自行补填。
3. **只读跳转，不做部署**：`ai_agent_target_open_path(id, slot)` 后端展开 token（`%USERPROFILE%`/`~`）后经共享 open_with 通路打开——目录走资源管理器、文件走系统默认打开。不做任何写入/清理；会话目录含隐私，本期不接入 Discovery 全局搜索。
4. **项目 AI 目录只读检测**：`ai_project_ai_dirs(workspace)` 汇总 `.afm` + 该项目启用 Agent 的 `projectSkillsDir` / `projectRulesDir` 真实配置（项目启用集 ∩ 中央 enabled，ADR 0010 修订），只返回磁盘上存在的目录。前端提供「在资源管理器中打开」，与现有 `open_path_default` 同通路。
5. **错误码**：未知槽位 `AI_AGENT_SLOT_UNKNOWN`、路径未配置 `AI_AGENT_PATH_EMPTY`、路径不存在 `AI_AGENT_PATH_NOT_FOUND`。

曾考虑把会话目录纳入 Discovery 搜索：隐私风险与索引成本高，先不做。曾考虑直接编辑 60 条内置定义：改动面大且每次新增槽位都要重写一遍，改用旁挂补充表。
