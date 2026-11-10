# 项目 AI 状态与 Core `project.json` 分离

Project Workspace 的 MCP/Skills 部署状态与「项目 Agent 启用集」写在工作区内 AI Add-on 自管文件（`.afm/ai-project.json`），不写入 Core 的 `project.json`。这样保持轻量创建与 ADR 0007/0008 的 Core 边界；AI 状态可惰性初始化，删除/忽略 `.afm` 不影响 Core 元数据。

曾考虑写入 `project.json` 或仅存 APPDATA 映射：前者污染 Core，后者使工作区无法随目录携带启用集。
