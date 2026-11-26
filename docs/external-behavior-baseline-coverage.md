# External-behavior regression baseline — coverage ledger

Living ledger for [ADR 0025](./adr/0025-external-function-regression-baseline.md). It records,
per user-visible behavior, **which baseline layer guards it and where**. When you add or change a
user-visible feature, follow §Testing new / changed features in [AGENTS.md](../AGENTS.md), then add
or refresh the row here. This is a coverage map, not a spec — product semantics live in the other
ADRs.

Legend — each behavior is guarded by one or more layers:
- **STATIC** — `scripts/check-*.mjs` symbol/grep gate, wired into the aggregated `npm test` chain.
- **FE** — `scripts/*-checks.ts` pure-logic behavior unit, `spawn`ed by a `.mjs`.
- **CARGO** — `#[cfg(test)]` under `src-tauri/src/**`, executed by `npm run test:rust`.

Rule reminders: no e2e; `#[ignore]` ≠ coverage; FE↔BE command-name parity is mandatory; an
`#[ignore]`d or chain-absent gate protects nothing.

## Core file operations

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| copy / cut / paste (data move) | STATIC · CARGO | `check-file-ops.mjs`; `file_ops/mod.rs` `copy_paths` / move roundtrips (`safe_io.rs`) | Real temp execution in cargo |
| new file / folder → new item selected | STATIC | `check-select-new-file-folder.mjs` | 4 surfaces (NavigatorView shortcuts + NavigatorContextMenu actions); same create-result path feeds undo + selection, then scrolls into view |
| paste self / conflict decision branch | STATIC | `check-file-ops.mjs`, `check-list-select.mjs` | `transferEntries` is invoke/taskCenter-fused → grep only (intentional; see gaps) |
| name conflict replace/skip + apply-to-all | STATIC · FE | `check-name-conflict.mjs` → `name-conflict-checks.ts` | CF-003/TR-014 (2026-11-22): "Apply to all" is **per-batch**, not persisted; gate asserts absence of `localStorage`/`afm.nameConflictDefault` + `resetNameConflictBatch()` in `transferEntries` |
| file↔folder type conflict warning | STATIC · FE · CARGO | `check-file-ops.mjs` (parity `file_ops_name_conflict_info`); `name-conflict-checks.ts` (mismatch flag); `mod.rs` `name_conflict_info_detects_type_mismatch` | TR-013/CF-006 (2026-11-22): folder-into-folder merge is **not** a mismatch |
| dir-into-dir merge + inner conflict recursion | CARGO | `mod.rs` `copy_dir_into_existing_dir_merges_no_inner_collision`, `..._handles_inner_collisions`, `move_dir_into_existing_dir_empties_source`, `move_merge_with_skip_preserves_source_when_not_empty` | CF-007/CF-008 (2026-11-22): merge / replace / skip / keep-both inner branches; move removes emptied source only |
| case-insensitive collision + case-only rename | CARGO | `mod.rs` `unique_dest_treats_case_only_difference_as_collision`, `rename_case_only_succeeds_but_real_collision_fails` (Windows), `rename_case_only_uses_inode_identity_on_unix` (macOS/Linux) | CF-005 (keep-both) + NM-004 (case rename). Windows uses case-folded path equality; Unix uses **(device, inode)** identity so it is correct on both case-sensitive and case-insensitive APFS/HFS+ volumes — no static `case_insensitive_paths` assumption |
| reserved-name reject + trailing dot/space strip | CARGO | `mod.rs` `validate_name_blocks_windows_reserved`, `validate_name_strips_trailing_dot_and_space` (both `#[cfg(windows)]`); `validate_name_keeps_posix_legal_names` (`#[cfg(not(windows))]`) | NM-003 / NM-007 (2026-11-22). Win32 rules (reserved names, trailing dot/space strip, Win32 illegal chars) are Windows-only: APFS/HFS+ accepts `CON`, `a.`, `a:b` and keeps them verbatim; POSIX validation rejects only `/`, `.` / `..` and empty |
| rename collision → Replace/Skip dialog | STATIC · CARGO | `check-navigator-inline-rename.mjs` (asserts `resolveNameConflictChoice` + `renameEntry(...,true)`); `mod.rs` `rename_replace_overwrites_existing_file_only_when_allowed` | NM-002 (2026-11-23): inline rename reuses the name-conflict dialog; Replace overwrites a file, never a directory |
| UNC/removable Delete → permanent fallback | STATIC · CARGO | `check-file-ops.mjs` (`pathsWithoutRecycle` + `confirmPermanentDelete` on both `shortcutDelete` and `runDelete`); `platform/windows.rs` `filesystem_supports_recycle_bin` | DEL-003/DEL-008 — availability now keys off the **filesystem** (NTFS-only recycles) not just UNC (2026-11-23 fix: FAT32/exFAT were silently erased); FE split grep-only, FS decision cargo-tested |
| delete → recycle bin | STATIC · FE · CARGO | `check-recycle-bin.mjs` → `recycle-bin-checks.ts`; `mod.rs` trash | |
| delete confirm dialog (default-on) | STATIC · FE | `check-delete-confirm.mjs` → `confirm-before-delete-checks.ts` | `parseConfirmBeforeDelete` |
| rename / prompt dialog | STATIC · FE | `check-prompt-dialog.mjs` → `prompt-dialog-checks.ts` | promise lifecycle, busy-cancel guard |
| inline rename | STATIC · CARGO | `check-navigator-inline-rename.mjs`; `rename_path` | |
| undo / restore | STATIC · FE · CARGO | `check-file-undo.mjs` → `file-undo-checks.ts` | restore loopback has no temp exec test (see gaps) |
| undo reverse-order multi-op (Ctrl+Z ×n) | FE | `check-file-undo.mjs` → `file-undo-checks.ts` (push copy/create/rename/move/delete, pop `delete→move→rename→create→copy`) | UN-006 (2026-11-23): stack is strict LIFO; each pop reveals the immediately-earlier record so the next Ctrl+Z keeps unwinding |
| undo does not persist across restart | STATIC | `check-file-undo.mjs` (`fileUndo/index.ts` + `stack.ts` must contain no `localStorage`/`sessionStorage`) | UN-007 (2026-11-22): in-memory only, matches Explorer |
| cancel/rollback excluded from undo stack | STATIC | `check-file-undo.mjs` (paste + drop callers `return` on `!result.ok` before `record*Undo`) | UN-009 (2026-11-22): an aborted batch is the transfer's own teardown; Ctrl+Z undoes the previous **complete** op, not the rollback |
| ZIP compress / extract | STATIC · FE · CARGO | `check-archive.mjs` → `archive-checks.ts`; `archive/mod.rs` `rejects_zip_slip`, roundtrip | FE↔BE name parity pinned |
| paste-target resolution (folder/file/mixed/none) | STATIC · FE | `check-paste-target.mjs` → `paste-target-checks.ts` (`src/state/pasteTarget.ts`); `NavigatorView.shortcutPaste` wires `resolvePasteTarget` over the selection | CLIP-101/102/104 (2026-11-23): single folder→into it, single file→its parent, mixed/none→current dir |
| select new items after paste (keyboard + context menu) | STATIC | `check-paste-post-state.mjs`; `revealItemsInNavigator(result.paths)` in `shortcutPaste` and `NavigatorContextMenu.runPaste` | CLIP-003/004, LC-001/002 (2026-11-23): Explorer-style reveal-and-select the transfer result |
| post-op scroll & sort preserved (same-dir refresh) | STATIC | `check-paste-post-state.mjs`; list `scrollTop` reset is keyed to `watch(currentPath)` (navigation) only, sort lives in module-level `sortColumn`/`sortDirection` refs that survive `refreshListing` | LC-011 (2026-11-23): same-directory copy/move/paste/delete keeps scroll + current sort |
| shift range in current view order | STATIC | `check-selection-view-order.mjs`; `selectRange(orderedPaths.value, …)` where `orderedPaths = visibleEntries.map` (sorted+filtered) | SEL-004 (2026-11-23): ordering arithmetic itself executed by `list-select-checks.ts` |
| parent+child source dedup in one transfer | STATIC · FE | `check-transfer-dedup.mjs` → `transfer-dedup-checks.ts` (`src/state/transferDedup.ts`); `transferEntries` runs `dedupeDescendantSources(sources)` | SEL-007/011 (2026-11-23): a folder and its descendant are processed once; folder-into-itself *target* stays a backend refusal (TR-006 CARGO) |
| per-item transfer error dialog (Retry / Skip / Abort) | STATIC · FE | `check-transfer-error.mjs` → `transfer-error-checks.ts` (`src/state/transferError.ts`); `TransferErrorDialogHost.vue` mounted in `AppShell.vue`; `transferEntries` loops per-item on a retryable class | LC-007 (2026-11-22): one failed item asks Retry/Skip/Abort instead of killing the batch. Dialog is per-item, never persisted (CLIP-008-style skip is a separate path). GUI smoke (open dialog against a real locked/full-disk item) is a manual todo |
| per-item error class FE↔BE parity | STATIC | `check-transfer-error.mjs` (asserts `transferError.ts` retryable set == backend `ERROR_CLASS_IN_USE/_ACCESS_DENIED/_DISK_FULL`); `fileClipboard.ts` `FileOpsResult.errorClass` | `in-use` / `access-denied` / `disk-full`; a hard refusal (folder-into-itself, volume root, validation) has no class → batch aborts unchanged |
| error-class classification (locale-stable) | CARGO | `mod.rs` `classify_transfer_error_maps_windows_codes` (`#[cfg(windows)]`), `classify_transfer_error_maps_posix_errnos` (`#[cfg(not(windows))]`), `classify_transfer_error_leaves_hard_refusals_uncategorized`, `err_msg_populates_per_item_error_class` | Platform-specific OS codes from `io::Error` Display: Windows 32/33/36→in-use, 5/13→access-denied, 39/112→disk-full; POSIX errno 16/26→in-use, 1/13→access-denied, 28/69→disk-full. Unknown codes must **fall through** to English fragments (a premature `return None` disabled the dialog on macOS); refusals stay unclassified |
| copy-out of an in-use file succeeds | CARGO | `mod.rs` `copy_paths_succeeds_while_source_handle_is_open` (#[cfg(windows)]) | LC-006 (2026-11-23): read-only copy is not blocked by the in-use lock; source stays intact. Move-in / replace onto an in-use target fails as `in-use` via the same classification path |
| disk-full → per-item dialog, source intact | STATIC · FE · CARGO | `check-transfer-error.mjs` (disk-full class retryable); `mod.rs` `classify_transfer_error_maps_windows_codes` / `..._maps_posix_errnos` | LC-008 (2026-11-23): reuses the LC-007 dialog; failed item's partial dest already scrubbed by `safe_io` staging, source preserved |
| cancel/abort rollback of completed copies | STATIC · FE | `check-transfer-error.mjs` (`transferEntries` aborts copy batch via `deleteEntries(transferred, true)`); `check-file-undo.mjs` (undo recorded only after `!result.ok` return) | LC-005 (2026-11-22) + UN-009: an aborted copy drops the copies made this batch, source stays; the rollback is never pushed onto the undo stack. Move-abort deliberately leaves completed moves (see Known gaps / decision) |
| cancel/rollback semantics of an in-flight copy/move | CARGO | `mod.rs` `cancelled_copy_short_circuits_before_work`, `rollback_removes_created_copy`, `rollback_moves_back_a_plain_move`; `#[ignore]` real two-volume fixtures `real_vol2_cross_volume_move_large_file` (TR-010) / `real_vol2_cancel_mid_move_keeps_sources_and_rolls_back` (TR-011) via `AFM_TEST_VOL2=<mount>` | LC-005 (2026-11-24): a cancelled transfer does zero work; rollback deletes the copy it created and moves a plain move back. TR-010/011 real two-volume runs verified 2026-11-24 against an attached VHD (`cargo test --lib real_vol2 -- --ignored`): 64 MB cross-volume move source-gone/dest-verified; mid-transfer cancel keeps all sources and leaves no destination residue |
| drag-drop transfer mode (same/cross volume + Ctrl/Shift/Alt) | FE | `check-drop-mode.mjs` → `drop-mode-checks.ts` (`src/state/dnd/mode.ts` `dropTransferMode` executed: same-vol=move, cross-vol=copy, Ctrl=copy, Shift=move, UNC same-server=move / cross-server=copy, Alt=link wins over Ctrl/Shift); surface wiring `check-cross-tab-drop.mjs` | TR-101..TR-105 (2026-11-24): Explorer-style modifier matrix; Alt=link routes to `file_ops_create_shortcuts` via clipboard `mode:"link"` |
| Shift+Delete → confirmed permanent delete | STATIC | `check-file-ops.mjs` (registry `navigator.deletePermanent` `shift+delete` chord + `registerShortcutHandler("navigator.deletePermanent")` in `NavigatorView` + `deletePermanentConfirm` menu/dialog + `confirmPermanentDelete` split); CARGO `delete_permanent_paths` roundtrips | DEL-004 (2026-11-24): wiring pinned in the gate; per-filesystem recycle split shared with DEL-003 |
| vanished cut/move source skipped silently (CLIP-008) | STATIC · CARGO | `check-transfer-error.mjs` (`transferEntries` auto-skips `mode === "move"` + `errorCode === "path_missing"`, tracks `skippedMissing`, toasts `fileOps.skippedMissing` with `{count}`, 6 locales); `mod.rs` `move_missing_source_tags_path_missing_for_clip008` | CLIP-008 (2026-11-24): Explorer-style — a stale cut path is dropped from the paste without a dialog, batch continues, one summary toast; copy of a vanished source keeps the raw localized error (intentional divergence from move) |
| undo refused on externally contaminated target (UN-008) | STATIC · FE · CARGO | `check-file-undo.mjs` (copy/create records capture `file_ops_fingerprints` at record time, re-check at undo time; refusal toasts `fileOps.undoContaminated` 6 locales and pops the record) → `file-undo-checks.ts` (`isContaminated` branches: no baseline / vanished path / size / mtime drift); `mod.rs` `fingerprint_changes_when_file_is_rewritten` | UN-008 (2026-11-24): destructive undos (copy/create) refuse with a localized toast instead of clobbering foreign edits; the popped record keeps the LIFO stack unwinding (UN-006); rename/move/delete undos stay unchecked (non-destructive) |
| restore recreates a vanished original parent chain (DEL-007) | STATIC · CARGO | `check-file-undo.mjs` (DEL-007 needle: batch restore script recreates the parent via `New-Item -ItemType Directory -Force`); `recycle.rs` `restore_script_recreates_missing_parent_chain` (script extracted to `restore_wanted_script` for testability); `#[ignore]` `real_recycle_restore_recreates_deleted_parent_dir` (real-Recycle-Bin roundtrip, long-path fixture) | DEL-007 (2026-11-24): Explorer parity — the PowerShell batch restore rebuilds the missing original dir chain after the Restore verb is found and before `DoIt()`. Real-bin roundtrip auto-verified 2026-11-24 (`cargo test --lib real_recycle -- --ignored --nocapture`): delete → remove parent chain → restore recreates the chain with content intact |
| Esc three-layer semantics: cancel rename > clear selection > pass through (SEL-010) | STATIC · FE | `check-esc-semantics.mjs` (inline-rename input `.stop` Esc, list Escape branch routes `escSemantics`, clears selection and consumes) → `esc-semantics-checks.ts` (`escSemantics` truth table); `check-selection-view-order.mjs` needle re-aligned to the grouped-view `orderedPaths` | SEL-010 (2026-11-24): Esc in the list clears the selection instead of falling through; with nothing selected the key is left for global handlers; rename input keeps priority via `.stop` |
 | long-path (>260 char) transfer/rename support (LC-009) | STATIC · CARGO | `check-long-path.mjs` (replacement manifest declares `longPathAware=true`, keeps asInvoker + Common-Controls); `mod.rs` `long_path_ops_roundtrip` (\\?\-extended deep-path copy/rename/move roundtrip) | LC-009 (2026-11-24): the shipped exe opts into long paths via the manifest; ops round-trip extended-form paths; display-form >260 paths additionally require system `LongPathsEnabled` — GUI smoke item |
| perf-baseline self-drive launch mode S1/S5 (`--afm-perf-baseline` / `AFM_PERF_BASELINE=1`) | STATIC | `check-perf-baseline.mjs` (FE↔BE parity `perf_baseline_enabled`/`perf_baseline_report` + `generate_handler` registration, `detect_flag` wired in `run()` and bypassing the single-instance early return, runner wires `startupMarks`/`setSpanRecording`/S5 tab sampling/`app_quit`, boot wiring in `main.ts`, chain + `test:perf-baseline`) | Diagnostic dev-build launch mode (ADR 0025 scope-exempt): auto-collects the S1 chain + S5 two-tab round-trip and reports `[AFM_BASELINE]` lines to the launching terminal, then exits. Behavior contract held by the gate's wiring/parity needles; no user-visible feature semantics beyond the diagnostic flow |
| self/descendant paste & move refusals + no-op reports (TR-004/006/007/008/107) | CARGO | `mod.rs` `paste_onto_self_replace_is_noop_success`, `copy_folder_into_itself_refused`, `copy_folder_into_descendant_refused`, `move_folder_into_descendant_refused`, `same_parent_move_is_noop_success` | TR-004/006/007/008/107 (2026-11-24): replace-self and same-parent move are **reported** no-ops (path returned for post-op selection, no physical copy); folder-into-itself/-descendant refusals are explicit negatives for copy and move alike |
| reparse-point copy follows the link; "Copy as link" advanced entry (TR-203) | STATIC · CARGO | `check-context-menu.mjs` TR-203 block (model `copyAsLink` item, handler sets the app-local `link` clipboard, link never synced to CF_HDROP, paste skips the system refresh); `mod.rs` `copy_junction_follows_link_content` (`#[cfg(windows)]`, mklink /J) | TR-203 (2026-11-24): default copy follows a junction/symlink and copies real content (Explorer parity); "Copy as link" routes paste to `transferEntries mode:"link"` → `file_ops_create_shortcuts`; DC/Linux copy-the-link-itself is the intentional divergence |

## Work groups / projects (external chain)

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| reveal / open in Explorer | STATIC · CARGO | `check-work-groups-open-explorer.mjs`, `check-open-project-folder.mjs`; `open_with` stub records `EXPLORER_LAST_PATH` | WG + Projects entry |
| multi-select → add to Work Group | STATIC · FE | `check-work-group-paths.mjs` → `work-group-paths-checks.ts` (`workGroupPaths.ts`) | pure reducer |
| promote one open tab → fixed member (WG detail row-level ↑) | STATIC | `check-wg-tab-to-members.mjs` (row-level ↑ with the `arrowUp` glyph; only folder tabs; rows that are already members are disabled with an explanatory label; the action calls the **additive** `work_groups_add_member` and never the overwrite path; FE↔BE command-name parity; 6-locale `workGroups.addTabToMembers` / `addedTabToMembers` / `alreadyMember`) | 2026-11-26: the "Currently open tabs" section gains a per-row ↑ that **appends** that one tab as a member — the pre-existing bottom action ("Open tabs → member config") still *replaces* the whole member list, so the two coexist with different semantics. Invoke-fused, no dialog → grep gate is the sanctioned layer (ADR 0025). GUI smoke: ↑ adds exactly one member, the row then reads "Already a member" and stays disabled |
| WG rename / delete | CARGO | `work_groups/sync.rs` `rename_work_group_*`, `delete_work_group_*` | pure-list `#[test]` |
| promote / member sync / bind-activate / create / copy / migrate | CARGO (+ menu FE) | `projects/lifecycle.rs`, `workspace.rs`, `work_groups/store.rs` | data layer already guarded; not duplicated |
| **D4 — create-project-from-WG preview + confirm**: both entries (`WorkGroupsView.promoteToProject`, `ProjectsView.createProjectFromWorkGroup`) open a `ConfirmDialog` describing the on-disk project folder (CONTEXT.md / tasks.json / skill·MCP) to be created and that the **source Work Group is kept**, before invoking; cancel is a no-op. The "keep source WG" backend semantics pre-date this round and stay guarded by the jump assertion. | STATIC | `check-wg-promote-jump.mjs` (asserts both views include `openConfirmDialog` **and** `promotePreviewMessage`); 6-locale `workGroups.promotePreviewMessage` (`check-i18n.mjs`) | D4 (2026-11-23): pre-execution confirmation is invoke-fused dialog wiring → grep gate is the sanctioned layer (ADR 0025). GUI smoke (cancel creates nothing; confirm leaves the original WG in place) is a manual todo — see `docs/qa/gui-smoke-checklist.md`. |
| project create writes an empty `tasks.json` skeleton (D5: replaces the retired `board.json`; the 5 `projects_workspace_board_*` commands + task UI are removed, board data **not** migrated) | STATIC · CARGO | `check-projects-ops-cleanup.mjs` (pins `projects_workspace_create` present) + `workspace.rs` `light_create_writes_only_core_files` (asserts `tasks.json` exists, `version`/empty `tasks`, and **no** `board.json`) | Tasks are now a plain file for AI / manual edit; no UI. Copy carries `tasks.json` verbatim (`lifecycle.rs`); legacy `projects.json` tasks are dropped (no migration). GUI smoke: confirm the board/list task UI is gone in `ProjectsView` and a created project dir has `tasks.json`. |
| one-time example presets (WG + 2 Projects — project 1 bound, project 2 unbound; home-dir tab; no re-seed after delete) | STATIC · CARGO | `check-example-presets.mjs`; `examples.rs` `seed_examples` (`seeds_work_group_and_two_projects_with_home_tab`, `marker_prevents_reseed_after_user_deletes_presets`, `partial_run_is_repaired_without_duplicating`) | startup wiring primary-only + non-fatal; marker file blocks resurrection |
| rename-to-make-it-yours guidance steps (WG + Projects tour) | STATIC | `check-example-presets.mjs`, `check-help-guidance.mjs`, `check-i18n.mjs` | anchors on detail rename title; registry keys in 6 locales; registry key count 55 |

## Help / onboarding

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| L2 hint bound to its own page; switching routes hides the source hint (incl. keep-alive Navigator → WorkGroups) | STATIC · FE | `check-hint-route-scope.mjs` → `hint-route-scope-checks.ts` (`src/help/hintRouteScope.ts`); `check-help-guidance.mjs` | 5x5 route matrix + Navigator→WorkGroups transition; `/help` never carries a page hint; seen-once, opt-in tour, `hintEnabled`/`tourEnabled` unchanged |

## Shortcut hint / chrome focus

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| floating shortcut hint never pops from cold-start focus (macOS reveal hands DOM focus to the first control), but still shows on hover and on Tab focus after real input | STATIC · FE | `check-action-tip.mjs` → `action-tip-checks.ts` (`src/shortcuts/actionTip.ts` `createActionTipInteractionGate`) | gate is per-mount and idempotent; only `pointerdown`/`keydown` mark interaction — `scroll`/`wheel` merely dismiss so startup scroll restoration cannot unlock it; hover stays immediate |
| TR-203 Copy as link works on macOS via Unix symlink (`.lnk` on Windows), with platform-correct name and notice | STATIC · FE · CARGO | `check-context-menu.mjs` → `link-paste-name-checks.ts` (`fileClipboard.ts::linkPasteName`); `file_ops/shortcut.rs` `shortcut_name_appends_link_suffix` + `unix_copy_as_link_creates_readable_symlink` | non-Windows `create_one_shortcut` = `std::os::unix::fs::symlink` to canonicalized target; `read_shortcut_target` = `read_link` (link preview works); `.link` suffix keeps same-dir paste distinct; `copyAsLinkDoneUnix` in all 6 locales |

## Backend command surfaces

| Domain | Layers | Gate | Notes |
| --- | --- | --- | --- |
| knowledge_service / managed_dependency / extension_host | STATIC | `check-backend-command-surface.mjs` | 24 commands registered + FE command-literal parity + managed install-chain tests intact |
| managed knowledge vault（保险库）通道注入：`[vault:…]` 密钥引用在托管 awam.knowledge 扩展里可 reveal/copy | STATIC | `check-knowledge-vault-env.mjs` | 2026-11-25：pack.json `KM_SECRET_SCRIPT/AGENT_TOKEN/SECRET_ID` → `${afm:settings:power.awamKnowledgeSecret*}` 占位、settings defaults + schema v16 迁移、6 locale 字段标签全链路断言；缺一环 = 托管服务 `/vault/*` 503 `vault_not_configured`（密钥处理报错、无法解密）。密钥值只落用户 settings，不进 pack.json |
| AI Add-on | STATIC · CARGO | `check-ai-addon-shell.mjs` | pre-existing strong gate (not rebuilt) |
| AI Agent Target 源头闸门（ADR 0010 修订 2026-11：中央 `enabled` 是源头） | STATIC · CARGO | `check-ai-addon-shell.mjs`（`filter((a) => a.enabled)` 三面板断言 + `t.enabled &&` + `AI_HARVEST_AGENT_NOT_ENABLED` + `central: &AiConfigDocument` needles）；`project_ai.rs` `resolve_project_agents_gated_by_central_enabled`；`mod.rs` `project_status_ignores_centrally_disabled_agents`；`harvest.rs` `harvest_scan_rejects_centrally_disabled_agent` / `harvest_import_rejects_candidates_from_disabled_agent`；`deploy.rs` `search_is_driven_by_agent_target_project_skills_dir` | 项目 chips / Harvest 源仅显示中央 enabled；fan-out 与已添加状态扫描排除中央禁用（存储暂存、重启用自动恢复）；项目内发现只扫项目启用集各 Target 的 `projectSkillsDir` 真实配置 |
| AI harvest (multi-agent MCP / Skills import + dedupe) | STATIC · FE | `check-harvest-dedupe.mjs` → `harvest-dedupe-checks.ts` (`harvestDedupe.ts`) | multi-agent scan merged into one candidate set; content-fingerprint detects renamed servers; duplicate reasons surfaced; FE↔BE command-name parity pinned |
| AI Agent Target 路径槽扩展 + AI 目录跳转（ADR 0028：sessions/rules 槽 schema v3 + 只读跳转层） | STATIC · CARGO | `check-ai-agent-dirs.mjs`（schema v3 / `BUILTIN_EXTRA_PATHS` / `resolve_global_slot_path` / `ai_agent_target_open_path` / `ai_project_ai_dirs` FE↔BE 名字对齐 / 六 locale key 齐）；`agents.rs` `migrates_v2_seeds_extra_paths_once_and_keeps_user_values` / `resolves_global_slot_paths_with_expansion_and_errors`；`project_ai.rs` `project_ai_dirs_lists_existing_dirs_readonly` | v2→v3 一次性迁移补种（空才种、用户清空不回种）；全局路径槽后端展开 token 后经共享 open_with 打开；项目 AI 目录只读检测（enabled_agent_ids，无 load_or_init 副作用） |
| providers / discovery | STATIC · CARGO | `check-provider-*.mjs`, `check-discovery-*.mjs` | |

## Download / install chain

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| managed provision allow-list / SHA / atomic write | CARGO | `managed_dependency/provision.rs` offline `#[test]` (injected `fetch`) | `force_provision_rejects_disallowed_host_before_fetch` etc.; existence pinned by command-surface gate |
| real-HTTP feed download roundtrip | — (ignored by design) | `provision.rs` `#[ignore]` | needs local listener; not in baseline |
| provider download/extract, ext_host install_from_feed (network) | STATIC | `check-provider-download.mjs`, `check-feature-pack-shell.mjs` | real network intentionally not made mandatory |
| provider error classification | STATIC · FE | `check-provider-error-hint.mjs` → `provider-error-message-checks.ts` | order-sensitive |
| three Feature Packs ship native macOS sidecars (ADR 0022) | STATIC | `check-mac-pack-release.mjs` | 2026-11-24: 三平台矩阵（Windows / macos-aarch64 / macos-x86_64）各建三个 pack；任一失败阻断 merge 并保留 draft（禁 `continue-on-error` / `always()`）；产物 ad-hoc 签名、按 `(packId, platform)` 入 feed。本机 x86_64 实测三产物均为 Mach-O + adhoc，manifest 三条 `macos-x86_64` |
| winget 社区源清单生成（Awam.AFileManager，ADR 0031） | STATIC · FE | `check-winget-channel.mjs` | 2026-11-27: `package-winget.mjs` 导出纯函数（目录小写首级 / defaultLocale 区分 / SHA 校验 / 拒预发布版本）真执行断言 + 脚本静态针；`winget validate` 成功判定解析输出文本（succeeded 也非零退出）；v0.3.0 冒烟通过（4 文件 validate succeeded） |
| ad-hoc macOS install clears Gatekeeper quarantine (7.28) | STATIC + local rehearsal | `check-ci-macos.mjs` (installer / DMG / Release asset / help text); `scripts/install-macos.sh` exercised on a real app copy in a temp `--apps-dir` | macOS 15+ removed the Finder right-click → Open override, so the zero-Developer-ID path ships `install-macos.sh` inside the DMG and as a Release asset: it ditto-copies the ad-hoc-signed app to /Applications, moves the old version to Trash, removes `com.apple.quarantine`, re-verifies the signature and opens the app. Quarantine propagation from both ZIP and DMG was measured; pre-flagging a ZIP as approved is overwritten on extraction |
| self-hosted Homebrew tap cask for ad-hoc builds (7.29) | STATIC + local render | `check-ci-macos.mjs` 2h (cask template / sync script / workflow token / README + help); `scripts/sync-homebrew-tap.mjs` rendered a real v0.3.0 cask from Release asset digests, `ruby -c` OK | Official `homebrew/cask` rejects ad-hoc builds since Homebrew 5/2026-09; self-hosted `awamwang/homebrew-tap` uses Homebrew 7 `postflight_steps` + `run "/usr/bin/xattr"` to clear quarantine. Release workflow derives per-arch sha256 from asset `digest` and PUTs the cask via Contents API using a PAT secret; `finalize` blocks undraft if the tap job fails |

## Navigation / shell places

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| Network shares in tree (ADR 0026) | STATIC · CARGO | `check-shell-places.mjs` / `check-folder-tree.mjs` (Network root wiring); `navigator/places.rs` `parse_mount_network_filters_to_network_fstypes`, `parse_mount_network_accepts_macos_bare_mount_output` | Windows: Shell Network folder (WS-Discovery) + `net view \\server` for share expansion. macOS/Linux: bare `mount` (macOS rejects `-p`) parsed into mounted smbfs/afpfs/nfs/cifs/webdav volumes, mountpoints with spaces preserved. Unmounted-server Bonjour discovery is deliberately out of scope |

## Navigator view (sort / group)

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| 排序依据/分组依据下拉 + 空白右键子菜单 + 组折叠（ADR 0030） | STATIC · FE · CARGO | `check-navigator-sort-group.mjs` → `navigator-sort-group-checks.ts`（四字段分桶、「#」/unknown 垫底不随方向翻转、文件夹单独组置顶、折叠只剩组头、切字段默认方向、settings 水合、`toggleSort` 语义真执行 + 六语言 i18n 针）；`check-view-layout.mjs` / `check-list-virtual.mjs` 针同步；`migrate.rs` v16→v17 4 tests | 选字段保持当前方向；折叠会话级记忆；中文首字母桶走 zh collation 二分；四键持久化，legacy `core.navigatorGroupByType` 仅留迁移，UI 隐藏 |

## Navigator view (drill-through)

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| 穿透视图：把当前目录的子文件夹摊平进同一列表，深度可调（ADR 0031） | STATIC · FE · CARGO | `check-navigator-drill.mjs` → `navigator-drill-checks.ts`（深度档位收敛、树内判定、排序键=根相对路径、写入落点=穿透根 真执行 + 六语言 i18n 针）；`navigator/drill.rs` 6 tests（深度 1/2 语义、深度 0 不遍历、相对层级、隐藏项、拒 awam://）；`migrate.rs` v17→v18 2 tests | 资源管理器的显示模式（左侧导航不变）；入口＝工具条「图标 + chevron」弹层（关闭 / 1…5 层）；穿透根与当前位置是两个概念，树内双击＝树内定位，走出树则自动关闭；名称按根相对路径排，普通视图等价；新建/粘贴落点=穿透根；上限 2000 行 / 20000 次访问，超出截断横幅；档位持久化（`core.navigatorDrillDepth`，v18），开关会话级 |

## Knowledge capability

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| Knowledge doc "Show in AFM" → AFM-internal reveal: the note's folder opens as a **new tab at the end of the current open group**, the `.md` is selected, and no system file manager is launched (ADR 0029) | STATIC | `check-knowledge-reveal-in-navigator.mjs` | dialog closes first (z-index 1050 overlay would cover the navigator), then `revealItemsInNavigator([path], { appendTab: true })` → `newNavigatorTab` → `openStateWithNewTab` (append + activate); `revealItemInDir` must stay out of the dialog; 6 locales on `knowledgeView.showInNavigator`, the retired `knowledgeView.openInExplorer` key and its macOS Finder override are gone |

## Wiring hygiene

| Item | State |
| --- | --- |
| Orphan gates (defined but not in `test` chain) | Cleared to 0 (H8: `perf-probe`, `new-file-rename`, `properties-attrs`, `provider-torrent-open`, `delete-keep-selection` wired) |

## Process / console hygiene

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| no console window on helper spawn (where.exe / rclone / IDE / open_with / `net use`) | STATIC | `check-no-console-flash.mjs` | AIA-177; 2026-11-21 added `net use` (navigator/places.rs) — bare `Command::new("cmd")` made Win11 default-terminal handoff launch Windows Terminal (and its persisted window) on every app start |

## Known gaps (deliberate or open)

- **paste `transferEntries` self/conflict branch** — reactivity/invoke-fused; only grep. Backend
  copy/move/conflict covered by cargo. A FE unit would need a mock-invoke harness (ADR 0025 says don't).
- **undo restore loopback** — Windows `recycle.rs DoIt`, no temp-exec cargo test (heavy).
- **knowledge vault lock/unlock state machine** — invoke-coupled; command-surface covered, behavior not.
- **reorder pure helper `createListPointerReorder`** — `import { ref } from vue`, never executed;
  optional vue-stub FE unit (W4).
- ~~**shell-icon queue/budget LRU eviction**~~ — CLOSED (8eba40f): `scripts/shell-icons-state-checks.ts`
  imports and executes the real `src/state/shellIcons.ts` cache/queue (entry cap, byte budget,
  FIL-68 pinning, soft-vs-hard pin priority, FIL-50 pending dedup); both `check-shell-icon-*.mjs`
  gates spawn it, so coverage is no longer mirror-simulation only.

## Decision records (why a gap or a pin stays as it is)

| # | Subject | Decision | Where recorded |
| --- | --- | --- | --- |
| R1 | paste `transferEntries` self/conflict branch | Not tested as a FE unit: needs a mock-`invoke` harness, which ADR 0025 forbids; backend copy/move/conflict already executed by cargo | ADR 0025 「定论（不强做）」 |
| R2 | undo restore loopback | Deferred: restore calls the Windows Recycle Bin COM `DoIt`, so a temp-exec cargo test cannot run offline (heavy, machine-state dependent) | ADR 0025 「定论（不强做）」 |
| R3 | knowledge vault lock/unlock state machine | Command surface + IPC name parity pinned (`check-backend-command-surface.mjs`, `check-ipc-parity.mjs`); state machine left as grep because it is `invoke`-coupled | ADR 0025 「定论（不强做）」 |
| R4 | reorder helper `createListPointerReorder` (W4) | Optional vue-stub FE unit, low priority: pure but pulls `import { ref } from "vue"`, never executed today | ADR 0025 + this ledger |
| R5 | "Close all tabs" (tab context menu) | **Removed as a feature**: menu entry, `onCloseAllTabsFromContext` handler and the 6-locale `chrome.closeAllTabs` keys are gone. Gates therefore assert the entry **must not reappear**, and deliberately no longer require `navigatorBrowse.closeAllNavigatorTabs` / `model.openStateAfterCloseAllGroups`, which are currently unreferenced dead API. `openStateAfterCloseAllGroups` keeps real behavior coverage via `scripts/open-state-checks.ts`, so dropping the grep pin loses no executed coverage and unblocks deleting the dead function. | this ledger (2026-11-22) + `check-tab-copy-path.mjs` comment |
| R6 | LC-003 delete post-state wording vs code | **Resolved 2026-11-24: select neighbor (用户拍板)**. The shipped code and gate were already the Explorer neighbor rule (`nextSelectionAfterRemoval`: successor → last-item fallback → null on empty dir); the 2026-11-23 "keep the selection" wording was a doc error and is corrected across `file-ops-scenario-baseline.md` / `PR-baseline-decisions.md`. No code change. Gate keeps its historical `check-delete-keep-selection.mjs` name but asserts the neighbor rule. | this ledger (2026-11-24) + `check-delete-keep-selection.mjs` |
| R7 | LC-005/LC-007 Abort rollback granularity | **Resolved 2026-11-24: keep current behavior (用户拍板「保存现状」)**. On Abort, `transferEntries` deletes the copies created so far (lossless) and deliberately leaves completed **moves** in place — an auto move-back of already-committed items risks clobbering foreign writes at the original path and has no crash-safe primitive. The failed item's partial dest is scrubbed by `safe_io` staging. No code change. | this ledger (2026-11-24) + `transfer-error-checks.ts` / `fileClipboard.ts` abort branch |
| R8 | GUI smoke for the per-item dialog + multi-step Ctrl+Z | The dialog against a **real** locked file / full disk and consecutive Ctrl+Z over a mixed copy/move/rename/create/delete sequence need live window interaction (can't run headless; ADR 0025 forbids e2e). The pure dialog state machine + LIFO stack + backend error-class are guarded offline; end-to-end click-through is a manual todo for the user. | this ledger (2026-11-23) |
