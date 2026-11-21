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
| name conflict replace/skip/remember | STATIC · FE | `check-name-conflict.mjs` → `name-conflict-checks.ts` | localStorage shim; short-circuit on remembered default |
| delete → recycle bin | STATIC · FE · CARGO | `check-recycle-bin.mjs` → `recycle-bin-checks.ts`; `mod.rs` trash | |
| delete confirm dialog (default-on) | STATIC · FE | `check-delete-confirm.mjs` → `confirm-before-delete-checks.ts` | `parseConfirmBeforeDelete` |
| rename / prompt dialog | STATIC · FE | `check-prompt-dialog.mjs` → `prompt-dialog-checks.ts` | promise lifecycle, busy-cancel guard |
| inline rename | STATIC · CARGO | `check-navigator-inline-rename.mjs`; `rename_path` | |
| undo / restore | STATIC · FE · CARGO | `check-file-undo.mjs` → `file-undo-checks.ts` | restore loopback has no temp exec test (see gaps) |
| ZIP compress / extract | STATIC · FE · CARGO | `check-archive.mjs` → `archive-checks.ts`; `archive/mod.rs` `rejects_zip_slip`, roundtrip | FE↔BE name parity pinned |

## Work groups / projects (external chain)

| Behavior | Layers | Gate / test | Notes |
| --- | --- | --- | --- |
| reveal / open in Explorer | STATIC · CARGO | `check-work-groups-open-explorer.mjs`, `check-open-project-folder.mjs`; `open_with` stub records `EXPLORER_LAST_PATH` | WG + Projects entry |
| multi-select → add to Work Group | STATIC · FE | `check-work-group-paths.mjs` → `work-group-paths-checks.ts` (`workGroupPaths.ts`) | pure reducer |
| WG rename / delete | CARGO | `work_groups/sync.rs` `rename_work_group_*`, `delete_work_group_*` | pure-list `#[test]` |
| promote / member sync / bind-activate / create / copy / migrate | CARGO (+ menu FE) | `projects/lifecycle.rs`, `workspace.rs`, `work_groups/store.rs` | data layer already guarded; not duplicated |

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

## Known gaps (deliberate or open)

- **paste `transferEntries` self/conflict branch** — reactivity/invoke-fused; only grep. Backend
  copy/move/conflict covered by cargo. A FE unit would need a mock-invoke harness (ADR 0025 says don't).
- **undo restore loopback** — Windows `recycle.rs DoIt`, no temp-exec cargo test (heavy).
- **knowledge vault lock/unlock state machine** — invoke-coupled; command-surface covered, behavior not.
- **reorder pure helper `createListPointerReorder`** — `import { ref } from vue`, never executed;
  optional vue-stub FE unit (W4).
- **shell-icon queue/budget LRU eviction** — needs confirmation the trigger is executed in cargo.
