# Skills / MCP 中央仓库与项目分发 — 定稿设计

本文汇总 grilling 定案；术语见根目录 `GLOSSARY.md`，决策见 `docs/adr/0009`–`0011`。

## 目标

面向 Agent、以 Project Workspace 为维度，补齐中央仓库的 **手动导入（Harvest）**、条目 CRUD 展示，以及 **中央 ↔ 项目** 的双向部署入口。不与 ToolPort / Skills Manager 等外部工具对接（仅方案参考）。

## 已有基础（保留）

| 能力 | 现状 |
| --- | --- |
| 中央 MCP 目录 | APPDATA `ai-config.json`；upsert / remove / JSON 文件导入；`afm__*` merge 部署 |
| 中央 Skills 库 | APPDATA（可改 root）；要求 `SKILL.md`；Junction 部署 / 卸载 |
| Agent Target | 内置预设 + 自定义；全局/项目相对路径；`enabled` 控制全局部署扇出 |
| UI | AI 设置：Providers / MCP / Skills / Agents / Harness |

## 增量设计

### 1. Harvest（手动从 Agent 导入）

- 入口：MCP 面板、Skills 面板各自「从 Agent 导入」
- 流程：选 Agent Target → 扫描其**全局** MCP 路径或 Skills 目录 → 多选 → 快照写入中央仓库
- 冲突：同名 id / skill name **跳过不覆盖**
- 不做：自动发现、定时同步、外部工具 API 对接、扫描项目相对路径

### 2. 中央仓库管理（完善）

- MCP：列表展示 + 增删改 + JSON 导入（已有，补齐入口与体验）
- Skills：列表展示 + 导入/删除；可从配置的中央 root 刷新列出
- 每条提供「添加到项目」（见下）

### 3. Agent 启用三层（ADR 0010）

1. `enabled` — 是否参与**全局**部署
2. `defaultInProjects` — 新项目启用集默认是否勾选（首次对齐 `enabled`）
3. 项目 Agent 启用集 — 存在 `.afm/ai-project.json`（ADR 0009），可覆盖，不回写中央

### 4. 项目分发

- 项目部署（MCP 与 Skills）**只**写入该项目启用集中的 Agent Target 的项目相对路径
- 全局部署仍只看 `enabled`
- 惰性初始化：首次打开项目 AI 区或首次项目部署时，若无 `.afm/ai-project.json` 则按 `defaultInProjects` 生成；Core 创建 Workspace **不**预写

**双向入口（同一套部署命令）：**

| 方向 | UI |
| --- | --- |
| 条目 → 项目 | 中央 MCP/Skill 条目：「添加到项目」→ 选 Project（默认 active）→ 按该项目启用集部署 |
| 项目 → 条目 | 项目 AI 区：管理启用集 + 从中央挑选 MCP/Skills 部署/卸载 |

### 5. Skill 部署方式（ADR 0011）

- 默认：Junction
- AI 配置全局选项：可改为「复制」
- 冲突：非 AFM 产物 → 拒绝；AFM 产物 → 允许覆盖更新

## `.afm/ai-project.json` 形状（建议）

```json
{
  "schemaVersion": 1,
  "enabledAgentIds": ["cursor", "claude-code"],
  "updatedAt": 0
}
```

部署记录（可选后续切片）：本版可不强制持久化「已部署哪些条目」——以目标路径实况为准；若 UI 需要「已部署」徽章，可再加 `deployedMcpIds` / `deployedSkillNames`。

## 本版明确不做

- ToolPort / Skills Manager / awam-tool-manager 运行时对接
- 自动发现或后台同步
- 项目私有第二库（不进中央）
- 单次部署弹窗选择 Junction/复制
- Harvest 扫描项目内 MCP/Skills

## 实现切片建议（开工后）

1. Agent Target：`defaultInProjects` + Agents UI
2. `.afm/ai-project.json` 读写 + 项目 AI 区启用集 UI
3. 部署命令改为接受「目标 Agent 子集」（项目用启用集；全局仍用 `enabled`）
4. Harvest：MCP / Skills 扫描 + 导入 UI
5. 双向「添加到项目」入口（绑 active Project）
6. Skill 部署方式偏好（Junction | 复制）+ 复制路径冲突策略
