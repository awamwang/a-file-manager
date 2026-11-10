# Agent 启用的三层：`enabled` / `defaultInProjects` / 项目启用集

全局部署只看 Agent Target 的 `enabled`。另增 `defaultInProjects`，用于首次生成某项目的「项目 Agent 启用集」时的默认勾选（缺省时与当时的 `enabled` 对齐）。项目启用集可按 Project Workspace 覆盖，且不回写中央。项目范围的 MCP/Skills 部署只写入该启用集中的 Target。

曾考虑用单一 `enabled` 同时控制全局与项目：无法表达「全局开、某项目关」或反过来。曾考虑每项目强制手选而无中央默认：新建项目成本高。
