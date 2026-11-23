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
| paste self / conflict decision branch | STATIC | `check-file-ops.mjs`, `check-list-select.mjs` | `transferEntries` is invoke/taskCenter-fused → grep only (intentional; see gaps) |
| name conflict replace/skip + apply-to-all | STATIC · FE | `check-name-conflict.mjs` → `name-conflict-checks.ts` | CF-003/TR-014 (2026-11-22): "Apply to all" is **per-batch**, not persisted; gate asserts absence of `localStorage`/`afm.nameConflictDefault` + `resetNameConflictBatch()` in `transferEntries` |
| file↔folder type conflict warning | STATIC · FE · CARGO | `check-file-ops.mjs` (parity `file_ops_name_conflict_info`); `name-conflict-checks.ts` (mismatch flag); `mod.rs` `name_conflict_info_detects_type_mismatch` | TR-013/CF-006 (2026-11-22): folder-into-folder merge is **not** a mismatch |
| dir-into-dir merge + inner conflict recursion | CARGO | `mod.rs` `copy_dir_into_existing_dir_merges_no_inner_collision`, `..._handles_inner_collisions`, `move_dir_into_existing_dir_empties_source`, `move_merge_with_skip_preserves_source_when_not_empty` | CF-007/CF-008 (2026-11-22): merge / replace / skip / keep-both inner branches; move removes emptied source only |
| case-insensitive collision + case-only rename | CARGO | `mod.rs` `unique_dest_treats_case_only_difference_as_collision`, `rename_case_only_succeeds_but_real_collision_fails` | CF-005 (keep-both) + NM-004 (case rename) — Windows-only FS contract |
| reserved-name reject + trailing dot/space strip | CARGO | `mod.rs` `validate_name_blocks_windows_reserved`, `validate_name_strips_trailing_dot_and_space` | NM-003 / NM-007 (2026-11-22) |
| rename collision → Replace/Skip dialog | STATIC · CARGO | `check-navigator-inline-rename.mjs` (asserts `resolveNameConflictChoice` + `renameEntry(...,true)`); `mod.rs` `rename_replace_overwrites_existing_file_only_when_allowed` | NM-002 (2026-11-23): inline rename reuses the name-conflict dialog; Replace overwrites a file, never a directory |
| UNC/removable Delete → permanent fallback | STATIC · CARGO | `check-file-ops.mjs` (`pathsWithoutRecycle` + `confirmPermanentDelete` on both `shortcutDelete` and `runDelete`); `platform/windows.rs` `filesystem_supports_recycle_bin` | DEL-003/DEL-008 — availability now keys off the **filesystem** (NTFS-only recycles) not just UNC (2026-11-23 fix: FAT32/exFAT were silently erased); FE split grep-only, FS decision cargo-tested |
| delete → recycle bin | STATIC · FE · CARGO | `check-recycle-bin.mjs` → `recycle-bin-checks.ts`; `mod.rs` trash | |
| delete confirm dialog (default-on) | STATIC · FE | `check-delete-confirm.mjs` → `confirm-before-delete-checks.ts` | `parseConfirmBeforeDelete` |
| rename / prompt dialog | STATIC · FE | `check-prompt-dialog.mjs` → `prompt-dialog-checks.ts` | promise lifecycle, busy-cancel guard |
| inline rename | STATIC · CARGO | `check-navigator-inline-rename.mjs`; `rename_path` | |
| undo / restore | STATIC · FE · CARGO | `check-file-undo.mjs` → `file-undo-checks.ts` | restore loopback has no temp exec test (see gaps) |
| ZIP compress / extract | STATIC · FE · CARGO | `check-archive.mjs` → `archive-checks.ts`; `archive/mod.rs` `rejects_zip_slip`, roundtrip | FE↔BE name parity pinned |
| paste-target resolution (folder/file/mixed/none) | STATIC · FE | `check-paste-target.mjs` → `paste-target-checks.ts` (`src/state/pasteTarget.ts`); `NavigatorView.shortcutPaste` wires `resolvePasteTarget` over the selection | CLIP-101/102/104 (2026-11-23): single folder→into it, single file→its parent, mixed/none→current dir |
| select new items after paste (keyboard + context menu) | STATIC | `check-paste-post-state.mjs`; `revealItemsInNavigator(result.paths)` in `shortcutPaste` and `NavigatorContextMenu.runPaste` | CLIP-003/004, LC-001/002 (2026-11-23): Explorer-style reveal-and-select the transfer result |
| post-op scroll & sort preserved (same-dir refresh) | STATIC | `check-paste-post-state.mjs`; list `scrollTop` reset is keyed to `watch(currentPath)` (navigation) only, sort lives in module-level `sortColumn`/`sortDirection` refs that survive `refreshListing` | LC-011 (2026-11-23): same-directory copy/move/paste/delete keeps scroll + current sort |
| shift range in current view order | STATIC | `check-selection-view-order.mjs`; `selectRange(orderedPaths.value, …)` where `orderedPaths = visibleEntries.map` (sorted+filtered) | SEL-004 (2026-11-23): ordering arithmetic itself executed by `list-select-checks.ts` |
| parent+child source dedup in one transfer | STATIC · FE | `check-transfer-dedup.mjs` → `transfer-dedup-checks.ts` (`src/state/transferDedup.ts`); `transferEntries` runs `dedupeDescendantSources(sources)` | SEL-007/011 (2026-11-23): a folder and its descendant are processed once; folder-into-itself *target* stays a backend refusal (TR-006 CARGO) |

## Work groups / projects (external chain)

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| reveal / open in Explorer | STATIC · CARGO | `check-work-groups-open-explorer.mjs`, `check-open-project-folder.mjs`; `open_with` stub records `EXPLORER_LAST_PATH` | WG + Projects entry |
| multi-select → add to Work Group | STATIC · FE | `check-work-group-paths.mjs` → `work-group-paths-checks.ts` (`workGroupPaths.ts`) | pure reducer |
| WG rename / delete | CARGO | `work_groups/sync.rs` `rename_work_group_*`, `delete_work_group_*` | pure-list `#[test]` |
| promote / member sync / bind-activate / create / copy / migrate | CARGO (+ menu FE) | `projects/lifecycle.rs`, `workspace.rs`, `work_groups/store.rs` | data layer already guarded; not duplicated |
| project create writes an empty `tasks.json` skeleton (D5: replaces the retired `board.json`; the 5 `projects_workspace_board_*` commands + task UI are removed, board data **not** migrated) | STATIC · CARGO | `check-projects-ops-cleanup.mjs` (pins `projects_workspace_create` present) + `workspace.rs` `light_create_writes_only_core_files` (asserts `tasks.json` exists, `version`/empty `tasks`, and **no** `board.json`) | Tasks are now a plain file for AI / manual edit; no UI. Copy carries `tasks.json` verbatim (`lifecycle.rs`); legacy `projects.json` tasks are dropped (no migration). GUI smoke: confirm the board/list task UI is gone in `ProjectsView` and a created project dir has `tasks.json`. |
| one-time example presets (WG + 2 Projects — project 1 bound, project 2 unbound; home-dir tab; no re-seed after delete) | STATIC · CARGO | `check-example-presets.mjs`; `examples.rs` `seed_examples` (`seeds_work_group_and_two_projects_with_home_tab`, `marker_prevents_reseed_after_user_deletes_presets`, `partial_run_is_repaired_without_duplicating`) | startup wiring primary-only + non-fatal; marker file blocks resurrection |
| rename-to-make-it-yours guidance steps (WG + Projects tour) | STATIC | `check-example-presets.mjs`, `check-help-guidance.mjs`, `check-i18n.mjs` | anchors on detail rename title; registry keys in 6 locales; registry key count 55 |

## Help / onboarding

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| L2 hint bound to its own page; switching routes hides the source hint (incl. keep-alive Navigator → WorkGroups) | STATIC · FE | `check-hint-route-scope.mjs` → `hint-route-scope-checks.ts` (`src/help/hintRouteScope.ts`); `check-help-guidance.mjs` | 5x5 route matrix + Navigator→WorkGroups transition; `/help` never carries a page hint; seen-once, opt-in tour, `hintEnabled`/`tourEnabled` unchanged |

## Backend command surfaces

| Domain | Layers | Gate | Notes |
| --- | --- | --- | --- |
| knowledge_service / managed_dependency / extension_host | STATIC | `check-backend-command-surface.mjs` | 24 commands registered + FE command-literal parity + managed install-chain tests intact |
| AI Add-on | STATIC · CARGO | `check-ai-addon-shell.mjs` | pre-existing strong gate (not rebuilt) |
| providers / discovery | STATIC · CARGO | `check-provider-*.mjs`, `check-discovery-*.mjs` | |

## Download / install chain

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| managed provision allow-list / SHA / atomic write | CARGO | `managed_dependency/provision.rs` offline `#[test]` (injected `fetch`) | `force_provision_rejects_disallowed_host_before_fetch` etc.; existence pinned by command-surface gate |
| real-HTTP feed download roundtrip | — (ignored by design) | `provision.rs` `#[ignore]` | needs local listener; not in baseline |
| provider download/extract, ext_host install_from_feed (network) | STATIC | `check-provider-download.mjs`, `check-feature-pack-shell.mjs` | real network intentionally not made mandatory |
| provider error classification | STATIC · FE | `check-provider-error-hint.mjs` → `provider-error-message-checks.ts` | order-sensitive |

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
