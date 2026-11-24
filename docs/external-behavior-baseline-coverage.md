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
| cancel/rollback semantics of an in-flight copy/move | CARGO | `mod.rs` `cancelled_copy_short_circuits_before_work`, `rollback_removes_created_copy`, `rollback_moves_back_a_plain_move` | LC-005 (2026-11-24): a cancelled transfer does zero work; rollback deletes the copy it created and moves a plain move back. Same-volume staging only — TR-010/011 two-volume fixture stays a manual todo |
| drag-drop transfer mode (same/cross volume + Ctrl/Shift/Alt) | FE | `check-drop-mode.mjs` → `drop-mode-checks.ts` (`src/state/dnd/mode.ts` `dropTransferMode` executed: same-vol=move, cross-vol=copy, Ctrl=copy, Shift=move, UNC same-server=move / cross-server=copy, Alt=link wins over Ctrl/Shift); surface wiring `check-cross-tab-drop.mjs` | TR-101..TR-105 (2026-11-24): Explorer-style modifier matrix; Alt=link routes to `file_ops_create_shortcuts` via clipboard `mode:"link"` |
| Shift+Delete → confirmed permanent delete | STATIC | `check-file-ops.mjs` (registry `navigator.deletePermanent` `shift+delete` chord + `registerShortcutHandler("navigator.deletePermanent")` in `NavigatorView` + `deletePermanentConfirm` menu/dialog + `confirmPermanentDelete` split); CARGO `delete_permanent_paths` roundtrips | DEL-004 (2026-11-24): wiring pinned in the gate; per-filesystem recycle split shared with DEL-003 |
| vanished cut/move source skipped silently (CLIP-008) | STATIC · CARGO | `check-transfer-error.mjs` (`transferEntries` auto-skips `mode === "move"` + `errorCode === "path_missing"`, tracks `skippedMissing`, toasts `fileOps.skippedMissing` with `{count}`, 6 locales); `mod.rs` `move_missing_source_tags_path_missing_for_clip008` | CLIP-008 (2026-11-24): Explorer-style — a stale cut path is dropped from the paste without a dialog, batch continues, one summary toast; copy of a vanished source keeps the raw localized error (intentional divergence from move) |

## Work groups / projects (external chain)

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| reveal / open in Explorer | STATIC · CARGO | `check-work-groups-open-explorer.mjs`, `check-open-project-folder.mjs`; `open_with` stub records `EXPLORER_LAST_PATH` | WG + Projects entry |
| multi-select → add to Work Group | STATIC · FE | `check-work-group-paths.mjs` → `work-group-paths-checks.ts` (`workGroupPaths.ts`) | pure reducer |
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

## Backend command surfaces

| Domain | Layers | Gate | Notes |
| --- | --- | --- | --- |
| knowledge_service / managed_dependency / extension_host | STATIC | `check-backend-command-surface.mjs` | 24 commands registered + FE command-literal parity + managed install-chain tests intact |
| AI Add-on | STATIC · CARGO | `check-ai-addon-shell.mjs` | pre-existing strong gate (not rebuilt) |
| AI Agent Target 源头闸门（ADR 0010 修订 2026-11：中央 `enabled` 是源头） | STATIC · CARGO | `check-ai-addon-shell.mjs`（`filter((a) => a.enabled)` 三面板断言 + `t.enabled &&` + `AI_HARVEST_AGENT_NOT_ENABLED` + `central: &AiConfigDocument` needles）；`project_ai.rs` `resolve_project_agents_gated_by_central_enabled`；`mod.rs` `project_status_ignores_centrally_disabled_agents`；`harvest.rs` `harvest_scan_rejects_centrally_disabled_agent` / `harvest_import_rejects_candidates_from_disabled_agent`；`deploy.rs` `search_is_driven_by_agent_target_project_skills_dir` | 项目 chips / Harvest 源仅显示中央 enabled；fan-out 与已添加状态扫描排除中央禁用（存储暂存、重启用自动恢复）；项目内发现只扫项目启用集各 Target 的 `projectSkillsDir` 真实配置 |
| AI harvest (multi-agent MCP / Skills import + dedupe) | STATIC · FE | `check-harvest-dedupe.mjs` → `harvest-dedupe-checks.ts` (`harvestDedupe.ts`) | multi-agent scan merged into one candidate set; content-fingerprint detects renamed servers; duplicate reasons surfaced; FE↔BE command-name parity pinned |
| providers / discovery | STATIC · CARGO | `check-provider-*.mjs`, `check-discovery-*.mjs` | |

## Download / install chain

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| managed provision allow-list / SHA / atomic write | CARGO | `managed_dependency/provision.rs` offline `#[test]` (injected `fetch`) | `force_provision_rejects_disallowed_host_before_fetch` etc.; existence pinned by command-surface gate |
| real-HTTP feed download roundtrip | — (ignored by design) | `provision.rs` `#[ignore]` | needs local listener; not in baseline |
| provider download/extract, ext_host install_from_feed (network) | STATIC | `check-provider-download.mjs`, `check-feature-pack-shell.mjs` | real network intentionally not made mandatory |
| provider error classification | STATIC · FE | `check-provider-error-hint.mjs` → `provider-error-message-checks.ts` | order-sensitive |

## Navigation / shell places

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| Network shares in tree (ADR 0026) | STATIC · CARGO | `check-shell-places.mjs` / `check-folder-tree.mjs` (Network root wiring); `navigator/places.rs` `parse_mount_network_filters_to_network_fstypes`, `parse_mount_network_accepts_macos_bare_mount_output` | Windows: Shell Network folder (WS-Discovery) + `net view \\server` for share expansion. macOS/Linux: bare `mount` (macOS rejects `-p`) parsed into mounted smbfs/afpfs/nfs/cifs/webdav volumes, mountpoints with spaces preserved. Unmounted-server Bonjour discovery is deliberately out of scope |

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
| R6 | LC-003 delete post-state wording vs code | **Conflict surfaced, not silently changed**: the 2026-11-23 LC-003 text says "keep the selection, **not** the neighbor", but the shipped and gated `delete-keep-selection` behavior (`nextSelectionAfterRemoval` → `selectOnly(next)`) **moves to the next visible item** (Explorer's neighbor rule). These two statements disagree. Left the code/gate untouched (rewriting an existing passing gate is an Ask-first boundary) and flagged for the user to reconcile: either update the decision text to "select neighbor", or file a follow-up to change `afterShortcutDelete`/`afterDeleteSuccess` + the gate. | this ledger (2026-11-23) + `check-delete-keep-selection.mjs` |
| R7 | LC-005/LC-007 Abort rollback granularity | **Implemented at copy granularity only**: when the per-item dialog is Aborted mid-batch, `transferEntries` deletes the copies created so far (lossless — the source is intact) and leaves completed **moves** in place rather than moving them back, because an auto move-back of already-committed items risks clobbering anything a foreign process wrote at the original path and has no crash-safe primitive. LC-005's "clear partial destination results" is honored for the failed item (already scrubbed by `safe_io` staging) and for copies. Flagged for user: confirm move-batch Abort should (or should not) move back committed items, or route it through the existing undo `moveUndoRecord` reverse (which UN-009 forbids putting on the stack). | this ledger (2026-11-23) + `transfer-error-checks.ts` / `fileClipboard.ts` abort branch |
| R8 | GUI smoke for the per-item dialog + multi-step Ctrl+Z | The dialog against a **real** locked file / full disk and consecutive Ctrl+Z over a mixed copy/move/rename/create/delete sequence need live window interaction (can't run headless; ADR 0025 forbids e2e). The pure dialog state machine + LIFO stack + backend error-class are guarded offline; end-to-end click-through is a manual todo for the user. | this ledger (2026-11-23) |
