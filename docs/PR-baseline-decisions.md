# PR：文件操作基线纠正 + 项目功能（D1/D2/D4/D5）+ FE 行为簇 + 传输错误/undo

分支 `feat/baseline-decisions` → `master`（基线 `7749fb99`）。19 个提交，53 文件，+1872 / −1027。**全部本地未 push**；合并前需人工 GUI 冒烟 + `file_ops/mod.rs` 合并复核（见「风险与合并注意」）。

来源：2026-11-23 与需求方 grill 复核 `docs/file-ops-scenario-baseline.md` 后逐项拍板的改动。

---

## 主题 1 · 文件操作基线纠正（D1 / D2）
**D1 — 回收站可用性按文件系统判定**（`fix(file-ops): DEL-003`，`843ab704`）
- 修真实数据安全 bug：`path_can_use_recycle_bin` 旧版只挡 UNC，导致 **FAT32/exFAT 可移动盘**（无 `$RECYCLE.BIN`）被判「可回收」→ `trash::delete` 经 Shell **静默永久删除、无任何确认**。
- 改为：`platform/windows.rs` 新增纯判定 `filesystem_supports_recycle_bin`（仅 NTFS→可回收）+ `volume_supports_recycle_bin`（`GetVolumeInformationW` 取 FS 名）；`platform/mod.rs` 中立入口；`file_ops` 委托。FE 永久删除确认链自动生效。
- 测试：`only_ntfs_recycles` / `fat_and_exfat_do_not_recycle` / `unknown_filesystem_defers_to_trash`；`cargo test --lib recycle` 全绿。

**D2 — 重命名撞名走冲突对话框**（`feat(file-ops): NM-002`，`8cf8f075` 后端 + `daee363a` 前端）
- 后端 `rename_path` 加 `replace` 标志（覆盖已存在**文件**；目标为目录仍拒绝），命令 `file_ops_rename` 暴露可选 `replace`（向后兼容）。
- 前端内联改名撞名复用 `resolveNameConflictChoice`（Replace/Skip），并在重命名前 `resetNameConflictBatch()` 防止套用上次粘贴记忆。
- 测试：`rename_replace_overwrites_existing_file_only_when_allowed`、`rename_case_only...`；gate `check-navigator-inline-rename` 断言接线；vue-tsc 过。

## 主题 2 · 产品独有功能（D4 / D5）
**D4 — 从工作组创建项目前先预览确认**（`feat(projects)`，`0a38c362`）
- `WorkGroupsView.promoteToProject` 与 `ProjectsView.createProjectFromWorkGroup` 执行前加 `openConfirmDialog`，说明将创建目录 + 生成 `CONTEXT.md`/`tasks.json`/skill·MCP，且**原工作组保留**。
- 「保留源 WG」与「从工作组创建项目」措辞后端/六语此前已具备；本次补预览确认这一步。新增六语 `promotePreviewMessage`。
- 测试：`promote_creates_bound_project_keeping_wg`（后端保留 WG）；gate `check-wg-promote-jump` 断言 confirm 接线。

**D5 — `tasks.json`(Backlog.md 字段) 取代 `board` 任务体系**（`feat(projects)`，`0c027953`）⚠️ 破坏性
- 后端删 `projects/board.rs`（`WorkspaceBoard`/`BoardTask` + `projects_workspace_board_get/add/move/delete` 4 命令）；`workspace.rs::create_workspace_at` 改写空 `tasks.json`（`{version,tasks:[]}`）；`lib.rs` 去命令注册；前端 `ProjectsView.vue` **完全移除**任务 UI（−332 行）。
- 决策：**不迁移**，旧项目 `board.json` 作废（不写迁移逻辑）。
- 测试：`light_create_writes_only_core_files` 精确断言新项目 = `CONTEXT.md/members/project.json/tasks.json` 且 `!board.json`、tasks 空合法。`cargo test --lib projects::workspace` 13 passed。

## 主题 3 · 文件操作 FE 行为簇（粘贴目标 / 选中新项 / Shift 顺序 / 父子去重）
- `bebf970d` 粘贴目标按选区解析（选中目录→其内、文件→父目录、框选/无→当前目录）→ 抽纯函数 `state/pasteTarget.ts` + `paste-target-checks.ts` + `check-paste-target.mjs`。
- `61d8acfc` 粘贴/复制/移动后选中新项 + 保持滚动/排序 → `check-paste-post-state.mjs`。
- `4beb0753` Shift 范围按当前视图(排序/过滤)顺序 → `check-selection-view-order.mjs`。
- `787c602e` 传输源 parent+child 去重（FE reducer）→ `state/transferDedup.ts` + `transfer-dedup-checks.ts` + gate。
- 删除后选区（LC-003）**R6 拍板 2026-11-24：选邻居**。代码与 `check-delete-keep-selection.mjs`（历史名）本就实现 Explorer 邻居规则（`nextSelectionAfterRemoval`：后继→末尾回退→清空）；早期「保持选区」措辞系文档误写，已纠正。

## 主题 4 · 传输错误处理 + undo 语义
- `e005e23f` 传输单项失败 **逐项 Retry/Skip/Abort** 弹窗 + 错误分类：后端 `file_ops/mod.rs`（+in-use/访问/磁盘满分类），前端 `state/transferError.ts` + `TransferErrorDialogHost.vue` + `transfer-error-checks.ts` + `check-transfer-error.mjs`（六语）。Abort 走 LC-005 回滚、源安全由 safe_io staging 保证。
- `3fa8e242` undo 栈语义补强：多步 **逆序逐项撤销**、内存不跨重启(UN-007)、取消回滚不入栈(UN-009) 断言。

## 主题 5 · 基线文档 / 覆盖率 / QA 归档（docs）
- `b0bbaa73`/`02d1fc82`：`file-ops-scenario-baseline.md` + `explorer-baseline.md` 落档复核纠正与 14 项行为批量拍板（`❓ open` 清零）。
- `4cff9dcc`/`8e4f19a0`/`d1c0d52f`：coverage ledger 与基线文档同步到实际实现（含 D5 后去 board stale）。
- `45287153` D5 计划；`6e88b8de` GUI 冒烟清单(27)；`639c17fe` SM↔测试映射表。

---

## 验证汇总
- `cargo test --lib`：**489 passed**（唯一 `clipboard::windows_roundtrip_write` 在无头会话因无桌面剪贴板 fail，本轮未改该文件、正常桌面绿）。
- `vue-tsc --noEmit` rc=0；`node check-i18n.mjs` rc=0（locale parity）；`check-backend-command-surface` / `ipc-parity` rc=0（命令面 FE↔BE 一致）；新增/改动 gate 全 rc=0。

## 风险与合并注意
1. **`src-tauri/src/file_ops/mod.rs` 合并冲突**：本 PR 改它约 +210 行（D1/D2/错误簇）；`master` 工作区另有他人未提交的「传输 cancel/rollback」WIP（`copy_paths_cancellable` 等）也改同一文件 → 合并需人工按 hunk 复核。`projects/*`（D5 大改）同理注意。
2. **D5 不可逆**：移除 AIA-211 board 功能与任务 UI、旧 `board.json` 作废。PR 评审需向用户显著说明。
3. **GUI 冒烟未完成**：交互观感 + 真实介质（exFAT/NTFS/UNC/锁定文件/满盘/重启）待人工，见 `docs/qa/gui-smoke-checklist.md`（本轮暂缓）。
4. **未 push**：19 提交均本地；合并/推送需另行授权。
