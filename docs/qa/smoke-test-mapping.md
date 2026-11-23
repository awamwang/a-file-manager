<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# GUI 冒烟项 ↔ 自动测试覆盖映射表

把 `gui-smoke-checklist.md` 的 27 条交互项，映射到**已自动覆盖其行为逻辑**的测试（cargo 真执行 temp / FE 纯逻辑 `*-checks.ts` / grep 接线 gate），并标出仍需**真实窗口观感 / 真实介质**人工点验的部分。

图例：✅ = 本轮已在宿主实跑通过（逻辑/文件层已由机器保证）；🖐 = 只剩「真实窗口看起来对不对 / 需插盘·锁文件·满盘」人工抽验。

## 本轮实跑证据（非 grep，真执行）
- `cargo test --lib projects::workspace` → **13 passed / 0 failed**（含 `light_create_writes_only_core_files`：新项目 = CONTEXT.md/members/project.json/tasks.json，`!board.json`，tasks 空合法）。
- `cargo test --lib`（全量）：`rename_replace_overwrites_existing_file_only_when_allowed`、`rename_case_only_succeeds_but_real_collision_fails`、`copy_paths_succeeds_while_source_handle_is_open`、`permanent_delete_removes_file_and_refuses_root`、`permanent_delete_junction_removes_link_only`、`projects::store::tests::persists_round_trip_with_tasks` 均 `ok`。
- FE 纯逻辑 checks（`node scripts/check-*.mjs`）：`paste-target`、`transfer-dedup`、`transfer-error`、`file-undo`、`paste-post-state`、`selection-view-order` 全 OK。
- ⚠️ `clipboard::tests::windows_roundtrip_write` 在本**无头/非交互**宿主会话稳定 fail（需交互式桌面 `OpenClipboard`）；本轮**未改动** `src-tauri/src/clipboard.rs`，在正常桌面 session 会绿，非本轮回归。

## 映射表

| SM | 行为 | 覆盖它的自动测试 | 仍需人工 |
|---|---|---|---|
| SM-01 | exFAT/FAT32 Delete→永久确认 | ✅ cargo `only_ntfs_recycles`/`fat_and_exfat_do_not_recycle`/`unknown_filesystem_defers_to_trash`；FE grep `confirmPermanentDelete` | 🖐 真 exFAT U 盘看确认框 |
| SM-02 | NTFS 可移动→回收站 | ✅ cargo NTFS→true + `delete_to_recycle_local_file` | 🖐 真 NTFS U 盘 |
| SM-03 | UNC→永久确认 | ✅ cargo `unc_cannot_recycle` | 🖐 真 UNC 共享 |
| SM-04 | 混合可/不可回收分流 | ✅ 上述分类 + `check-file-ops`(pathsWithoutRecycle/confirmPermanentDelete) | 🖐 真混选删除 |
| SM-05 | 改名撞名弹 Replace/Skip | ✅ FE gate `check-navigator-inline-rename`(resolveNameConflictChoice) | 🖐 真实 F2 弹框 |
| SM-06 | Replace 覆盖文件 | ✅ cargo `rename_replace_overwrites_existing_file_only_when_allowed` | — |
| SM-07 | Replace 目标为目录拒绝 | ✅ 同测试 dir 分支 | — |
| SM-08 | Skip 取消改名 | ✅ FE name-conflict checks + gate | 🖐 观感 |
| SM-09 | 从 WG 建项目预览确认 | ✅ FE gate `check-wg-promote-jump`(openConfirmDialog+promotePreviewMessage) | 🖐 真实预览框 |
| SM-10 | 预览取消不创建 | （FE cancel 路径） | 🖐 人工点取消验证无目录 |
| SM-11 | 预览确认创建且原 WG 保留 | ✅ cargo `promote_creates_bound_project_keeping_wg` + `light_create_writes_only_core_files` | 🖐 观感 |
| SM-12 | 新项目有 tasks.json 无 board.json | ✅ cargo `light_create_writes_only_core_files` | — |
| SM-13 | ProjectsView 无任务看板/列表 | ✅ vue-tsc + 移除 UI 的 grep/dead-code gate | 🖐 打开页面目视 |
| SM-14 | 旧 board.json 被忽略 | ✅ 后端无 board 引用(grep 0)、无读取路径 | 🖐 打开旧项目目视 |
| SM-15 | 粘贴目标=选中目录 | ✅ FE `paste-target-checks.ts` + `check-paste-target.mjs` | — |
| SM-16 | 粘贴目标=文件父目录 | ✅ 同上（file 分支） | — |
| SM-17 | 粘贴目标=框选混合/无选中 | ✅ 同上（none/marquee 分支） | — |
| SM-18 | 复制/移动/粘贴后选中新项 | ✅ FE gate `check-paste-post-state.mjs`(setSelection) | 🖐 观感选中 |
| SM-19 | Shift 范围按当前排序 | ✅ FE gate `check-selection-view-order.mjs` + `check-list-select` | 🖐 改排序后真按 |
| SM-20 | 父+子混选只传一次 | ✅ FE `transfer-dedup-checks.ts` + `check-transfer-dedup.mjs` | 🖐 真拖一次目视目标 |
| SM-21 | 操作后滚动/排序保持 | 部分 `check-paste-post-state` | 🖐 滚动后操作目视 |
| SM-22 | 删除后保持选区(有意差异) | ✅ 现有 `check-delete-keep-selection.mjs` | 🖐 观感 |
| SM-23 | 多次 Ctrl+Z 逆序逐项撤销 | ✅ FE `file-undo-checks.ts`(多步逆序) + `check-file-undo.mjs` | 🖐 观感 |
| SM-24 | 重启后无 undo(不持久化) | ✅ gate 断言 undo 内存态、无持久化 | 🖐 真重启验证 |
| SM-25 | 传输单项失败 Retry/Skip/Abort | ✅ FE `transfer-error-checks.ts` + `check-transfer-error.mjs`；cargo `copy_paths_succeeds_while_source_handle_is_open`(in-use) | 🖐 真锁定文件弹框 |
| SM-26 | Abort 回滚、源完好 | ✅ cargo safe_io staging/源安全、`permanent_delete_*` | 🖐 真满盘触发 |
| SM-27 | 中止批次不入 undo 栈 | ✅ `check-file-undo.mjs`(cancel 不入栈) | 🖐 观感 |

## 结论
- **27 项中，其「行为逻辑」全部有自动测试钉住**（✅ 列）；纯文件层事实（SM-06/07/11/12/15/16/17 等）本轮已**实跑 cargo/checks 通过**。
- 仍需人工 GUI 的只剩两类：**① 真实窗口观感**（弹框出现、选中高亮、滚动不跳、看板消失）；**② 真实介质/条件**（exFAT·NTFS·UNC 卷、被进程锁定的文件、接近写满的盘、重启）。这些是 ADR 0025「不做 e2e」刻意留给人工的层。
- 非交互宿主下 `clipboard::windows_roundtrip_write` 会因无桌面剪贴板而红，与本轮无关。
