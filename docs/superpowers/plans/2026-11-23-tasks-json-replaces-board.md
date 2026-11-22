<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# D5 实施计划：用 tasks.json（Backlog.md 字段）取代 board 体系

状态：**待开工**（破坏性重构，需 GUI 冒烟，作为独立任务执行）。
决策来源：2026-11-23 grill-with-docs 复核。三项已拍板：
1. 用 `tasks.json` 取代 `board.json` 作为项目任务权威文件；
2. **不迁移**——旧项目 `board.json` 及其任务直接作废（不写迁移逻辑）；
3. **完全移除** `ProjectsView.vue` 的任务 UI（任务只以文件形式存在，供 AI/手改）。

## tasks.json 契约（Backlog.md 字段语义）
项目创建时生成空文件 `tasks.json`（与 CONTEXT.md / members/ 同级，落在 workspace 目录）：
```jsonc
{ "version": 1,
  "tasks": [
    { "id": "", "title": "", "status": "todo", "priority": "medium",
      "labels": [], "assignee": null, "dependencies": [], "acceptance": [], "notes": "" }
  ] }
```
- status: `todo | in_progress | done | blocked`；priority: `low | medium | high`。
- 首行/注释注明：**与 VS Code 可执行 `tasks.json` 语义不同**（AFM 的 tasks 是待办跟踪，非构建任务）。

## 后端删除 / 改动
- `src-tauri/src/projects/board.rs`：删除 `WorkspaceBoard`/`BoardTask`/`BoardTaskPatch`、四个 `projects_workspace_board_*` 命令与 board 逻辑、相关 `#[cfg(test)]` 测试（或保留文件只留 CONTEXT.md 读写、移除 board 部分）。
- `src-tauri/src/projects/workspace.rs`：`create_workspace_at` 不再写 `board.json`（`WORKSPACE_BOARD_FILE`/`WORKSPACE_BOARD_SCHEMA_VERSION`），改为写空 `tasks.json`；常量 `WORKSPACE_TASKS_FILE = "tasks.json"`。
- `src-tauri/src/projects/types.rs`：移除 `TASK_STATUSES`（backlog/doing/review/done）；如仍被引用，换成 Backlog.md 的 `["todo","in_progress","done","blocked"]`。
- `src-tauri/src/projects/mod.rs` / `lib.rs`：删除 board 命令注册（4 个）。
- `src-tauri/src/projects/legacy_migrate.rs` / `index.rs`：清理对 `board.json` 的引用。

## 前端删除
- `src/views/ProjectsView.vue`：移除 `viewMode`(list|board)、`addTask/moveTask/deleteTask/tasksForStatus/openTaskCount`、board/list 模板段（约 1490–1600）、`tasks: ProjectTask[]` 字段与 `projects_workspace_board_*` 调用、相关 CSS（`.task-list/.task-row/.board`）。
- `ProjectTask`/`WorkspaceBoard` 类型若仅服务此处，一并删。

## i18n（六语 en/zh-CN/zh-TW/ja/de/fr）
- 删除仅服务任务 UI 的 key：`projects.board/addTask/moveTask/deleteTask/openTasks/noOutcomes`（确认无其它引用后再删；保留仍被复用的）。

## gate / coverage ledger
- 删除 board 的 `check-*.mjs`（若有）并从 `package.json` test 链摘除；
- 更新 `check-backend-command-surface.mjs` 的命令清单（去掉 4 个 board 命令）；
- `docs/external-behavior-baseline-coverage.md` 把 board 行换成「项目创建生成空 tasks.json」行。

## 测试与验证（必须全绿）
- 后端 cargo：`create_workspace_at` 后断言存在 `tasks.json`（空、含合法骨架）且**不再**生成 `board.json`；替换原 board 测试。
- `cargo test --lib`（借主仓库 target：`--target-dir <repo>/src-tauri/target`）。
- 前端 `vue-tsc --noEmit -p tsconfig.json` 必须通过。
- `node scripts/check-i18n.mjs`（parity）+ 相关 check gate。
- **GUI app 冒烟**：打开一个项目，确认任务看板/列表已消失、项目目录里有 `tasks.json`、无控制台错误。

## 风险
- 删除既有 AIA-211 board 功能属不可逆；旧项目 `board.json` 数据将不再被读取（按决策作废）。建议 PR 描述里显著标注，避免用户以为丢 bug。
