<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# File-operations scenario baseline — behavior matrix

This document is the **scenario dimension** of the regression baseline. It answers one question per
cell: *for this operation, in this exact setup, what is the agreed behavior?* It is a spec of
**behaviors to guard**, not a comparison table and not a coverage map.

Three-layer guard model and the STATIC / FE / CARGO legend come from
[ADR 0025](./adr/0025-external-function-regression-baseline.md). The "current implementation vs
Windows Explorer" comparison lives in [`explorer-baseline.md`](./explorer-baseline.md). Which guard
covers which behavior lives in
[`external-behavior-baseline-coverage.md`](./external-behavior-baseline-coverage.md). This file links
to both and does **not** duplicate either.

## How to read this

- **Case id** is stable (`FO-<GRP>-<nnn>`). Later, every use case / test pins to a case id, and the
  coverage ledger references the id — not the prose. Do not renumber; retire a row by marking it
  `SUPERSEDED` and pointing at the replacement.
- **Expected** is the **Windows Explorer** consensus (primary baseline), per the 2026-11-06
  decisions in `explorer-baseline.md §已拍板`.
- **Divergence** names where other mainstream managers (Files, Double Commander / Total Commander,
  Dolphin, Nautilus) behave differently, and what **AFM should** do. Blank means there is no known
  divergence — Explorer's behavior is effectively the consensus.
- **AFM state** uses the legend below. It records what was verified against the code on the audit
  date, not what is merely decided.
- **Guard** names the existing `check-*.mjs` / `*-checks.ts` / cargo test, or `none`.

### Status legend (AFM state)

| Token | Meaning |
| --- | --- |
| `✅ code+test` | Behavior exists in code **and** a baseline test executes it. |
| `🟡 code` | Behavior exists in code; guarded only by a grep/symbol gate, no executed case yet. |
| `🔵 decided` | Semantics agreed in `explorer-baseline.md`; not yet verified in code this pass. |
| `❓ open` | No agreed baseline yet — surfaced by this matrix (see §Open decisions). |
| `⛔ refuse` | By-design rejected (error), aligned with consensus. |

### Guard layer legend (from ADR 0025)

`STATIC` = `scripts/check-*.mjs` symbol gate · `FE` = `scripts/*-checks.ts` pure-logic unit ·
`CARGO` = `#[cfg(test)]` under `src-tauri/src/**` with real temp execution. No e2e.

### References used for the consensus column

Windows File Explorer (primary). Cross-checked against the open-source set for the divergence
notes: **Files** (files-community/Files, WinUI), **Double Commander** / **Total Commander**
(classic dual-pane), **Dolphin** (KDE), **Nautilus / Files** (GNOME). Where they disagree with
Explorer this is called out explicitly rather than silently inherited.

---

## Part 1 — Operation inventory (8 groups)

Only "open a folder and use it" operations are in scope. Work groups, projects, Discovery,
providers, Feature Packs, dual-pane, command palette, open-with-IDE/terminal are product extras and
are **not** file-operations baseline (see `explorer-baseline.md §范围`).

| Group | id prefix | Operations |
| --- | --- | --- |
| 1. Selection & focus | `SEL` | click, ctrl add/remove, shift range, select-all, marquee, arrow keys, type-ahead, esc |
| 2. Clipboard | `CLIP` | copy, cut, paste, paste-target resolution, copy-path |
| 3. Data transfer & drag | `TR` | copy-to, move-to, drag-drop (same/cross volume, Ctrl/Shift/Alt modifiers), OLE in/out |
| 4. Name & structure | `NM` | rename, batch rename numbering, new folder, new file (ShellNew), properties |
| 5. Delete & recycle | `DEL` | recycle delete, permanent delete, non-recyclable volumes, restore/empty |
| 6. Conflict & keep-both | `CF` | replace / skip / keep-both, apply-to-all, remember |
| 7. Undo / redo | `UN` | Ctrl+Z over copy/move/rename/delete/create, ordering, contamination |
| 8. Lifecycle & post-state | `LC` | progress, cancel, error classes, post-op selection & scroll |

---

## Part 2 — Scenario dimensions (the axes)

Every case in Part 3 is a point in this space. An operation is only "bug-free" when its meaningful
cells here are each pinned by a case id.

| Dimension | Values |
| --- | --- |
| **D1 topology** (source vs destination) | same directory (self) · same volume, different dir · cross volume · to UNC/remote · to removable (NTFS/exFAT/FAT32) · to read-only volume · folder into itself/descendant · cut back into same parent · onto volume root |
| **D2 item type** | file · empty dir · deep tree · hidden+system · file symlink · dir symlink · junction · broken link · `.lnk` shortcut · read-only · locked/in-use · NTFS ADS · sparse · hardlink · OneDrive cloud placeholder |
| **D3 name/path** | case-only difference · trailing dot/space · no extension · multi-dot · masked extension (`.txt.exe`) · CJK/emoji/RTL · long path (>260) · reserved names (CON/PRN/AUX/NUL/COM*/LPT*) · invalid chars |
| **D4 selection shape** | single · ctrl multi (across layers) · shift range (sorted view order) · select-all (post-filter) · marquee partial overlap · parent + its child both selected · item vanished externally |
| **D5 conflict** | none · single · multiple · file-over-file · merge dir-into-dir · same name different type · case-only collision · remember + apply-to-all |
| **D6 lifecycle** | instant · large task · cancel mid-transfer · source removed mid-transfer · volume/eject mid-transfer · disk full · file in use · access denied · name too long · AV scan hold · network drop |
| **D7 undo** | single op · reverse of several · redo · undo a partial transfer · undo after external contamination · undo after app restart |
| **D8 post-state** | select new item · keep original selection · select neighbor after delete · undo restores focus · preserve scroll |

**The scenario combinations the user named** map to:
- same-directory copy → `TR` × D1 "same directory (self)" × D2 file/dir × D5 keep-both/replace/skip
- cross-drive copy → `TR` × D1 "cross volume" × D6 "large task / cancel / disk full" × D7 undo across volume

---

## Part 3 — Case matrix

### Group 2 · Clipboard (CLIP)

Clipboard is the **system** clipboard (`CF_HDROP`), verified: `src/state/fileClipboard.ts:2`
("so copy/cut/paste works with File Explorer"); set/get/clear go through
`file_ops_clipboard_set/get/clear`; paste resolves through `transferEntries`
(`fileClipboard.ts:92`). This closes the `explorer-baseline.md §剪贴板` "不一致" row.

| id | precondition | action | expected (Windows) | divergence | AFM state | guard |
| --- | --- | --- | --- | --- | --- | --- |
| CLIP-001 | one file selected | Ctrl+C | entry enters clipboard as *copy*; icon ghost unchanged; interoperable with Explorer paste | Files/DC equivalent | `🟡 code` | `check-file-ops.mjs` |
| CLIP-002 | one file selected | Ctrl+X | entry ghosted; clipboard holds *cut* (`fileClipboard.ts:27` cut-path set) | — | `🟡 code` | `check-file-ops.mjs` |
| CLIP-003 | copied file, current dir empty | Ctrl+V | copy lands; **new item selected** | — | `🔵 decided 2026-11-23` — 粘贴成功后选中新写入项（对齐 Explorer） | see LC-001 |
| CLIP-004 | cut file, paste into other dir | Ctrl+V | move lands, source gone; **new item selected** | — | `🔵 decided 2026-11-23` — 粘贴成功后选中新写入项（对齐 Explorer） | see LC-002 |
| CLIP-005 | paste with **nothing** copied/cut | Ctrl+V | no-op, silent | — | `🟡 code` (`canPaste` guard) | `check-file-ops.mjs` |
| CLIP-006 | copy in AFM | paste into **Explorer** | Explorer shows the item, paste works | requires system clipboard — now true | `🔵 decided`→`🟡 code` | `check-file-ops.mjs` |
| CLIP-007 | copy in Explorer | paste into AFM | AFM reads `CF_HDROP` and pastes | — | `🟡 code` (`syncFromShell`) | `check-file-ops.mjs` |
| CLIP-008 | clipboard cut paths later removed externally, paste | Ctrl+V | graceful error / drop stale cut, not panic | — | `🔵 decided` — 跳过该失效路径，继续其他项；最后 toast 提示「N 项已跳过」 | **decided 2026-11-22** |

**Paste-target resolution** (D1, the axis that causes the most surprise):

| id | selection at paste time | expected target | divergence | AFM state |
| --- | --- | --- | --- | --- |
| CLIP-101 | a folder is selected | paste **into that folder** | Explorer: same; Nautilus: same | `🔵 decided 2026-11-23` — 目标=粘入该文件夹 |
| CLIP-102 | a file is selected | paste into the file's **parent** dir | Explorer: parent | `🔵 decided 2026-11-23` — 目标=该文件的父目录 |
| CLIP-103 | nothing selected | paste into current dir | — | `🟡 code` |
| CLIP-104 | marquee over mixed files+folders | paste into current dir (selection is not a single container) | Explorer: current dir | `🔵 decided 2026-11-23` — 框选混合(非单一容器)→目标=当前目录 |

### Group 3 · Data transfer & drag (TR)

Backend verified in `file_ops/mod.rs`. Key facts: `unique_dest_path` yields `name - Copy` then
`name - Copy (n)` (`mod.rs:147`); `place_dest` special-cases paste-onto-self (`mod.rs:232`); copy
refuses folder-into-itself (`mod.rs:294`); move refuses volume root (`mod.rs:379`) and treats
same-parent cut as no-op (`mod.rs:392`); `total_transfer_bytes` walks with `follow_links(false)`
(`mod.rs:338`). Move/copy safety (staging + verify before source delete) via `safe_io` (AIA-275).

| id | precondition | action | expected (Windows) | divergence | AFM state | guard |
| --- | --- | --- | --- | --- | --- | --- |
| TR-001 | file a.txt, empty other dir same volume | copy → dest | a.txt appears in dest, source intact | — | `✅ code+test` | CARGO `copy_rename_mkdir_roundtrip`; `check-file-ops.mjs` |
| TR-002 | file a.txt, same dir | paste (keep-both) | `a - Copy.txt`, original kept | DC/Total: same suffix convention | `✅ code+test` | CARGO `same_dir_paste_keep_creates_copy` (`mod.rs:754`) |
| TR-003 | dir sub, same dir | paste (keep-both) | `sub - Copy` with contents | — | `✅ code+test` | CARGO `same_dir_paste_keep_creates_copy` |
| TR-004 | file a.txt, same dir | paste (replace) | **no-op success**, no copy made | Explorer: replace self = no-op | `✅ code` (`place_dest` exact==source, `mod.rs:232`) | needs its own CARGO row (see gaps) |
| TR-005 | file a.txt, same dir | paste (skip) | nothing created | — | `✅ code+test` | CARGO `same_dir_paste_keep_creates_copy` (skip branch) |
| TR-006 | dir A | copy A **into** A | error "Cannot copy a folder into itself", A intact | all managers refuse | `✅ code` (`mod.rs:294`) | needs negative CARGO (gaps) |
| TR-007 | dir A, A/sub exists | move A → A/sub (descendant) | refused as into-itself | — | `✅ code` (`is_descendant_or_same`) | needs negative CARGO (gaps) |
| TR-008 | cut file, same parent as target | move to same dir | **no-op success** (no copy, source stays) | Explorer: same; DC: no-op | `✅ code` (`mod.rs:392`) | CARGO implied, needs explicit row |
| TR-009 | file on C: → folder on D: | cross-volume copy | full copy; byte progress shown; source stays | — | `✅ code` (tracked `copy_paths`) | `check-file-ops.mjs`; no cross-volume fixture |
| TR-010 | file on C: → folder on D: | cross-volume **move** | copy to D:, verify, then delete on C:; on any failure source preserved | safe_io staging property | `✅ code` (`safe_io::safe_move_entry`) | CARGO `move_within_temp` (same-vol only) |
| TR-011 | folder tree → other volume | move, mid-transfer power loss/cancel | destination may hold **partial** tree, but **source must NOT be lost** | this is the consensus safety invariant | `✅ code` (staging+verify) | gaps: no crash-mid test |
| TR-012 | file onto existing same-name in dest | copy → conflict | **ask**: Replace / Skip / Keep both (`name-conflict-checks.ts`) | Files: same three; DC: overwrite prompt | `✅ code+test` | `check-name-conflict.mjs`→`name-conflict-checks.ts`; CARGO `replace_overwrites_and_skip_leaves_existing` |
| TR-013 | file onto existing **dir** same name | copy → conflict | conflict type must be file-vs-dir, offer replace (removes dir) / skip / keep-both | Explorer: warns "this replaces the folder" | `🔵 decided` — 专门弹窗，警告类型不同；提供 replace / skip / keep-both | **decided 2026-11-22** |
| TR-014 | many conflicts in one paste | choose Replace on first | "Apply to this only / All" — Explorer offers *Do this for all* | Files: apply-to-all checkbox | `🔵 decided` — apply-to-all 仅本次操作内生效，下次粘贴重新问 | **decided 2026-11-22** |
| TR-015 | copy **onto volume root** as dest | move to `D:\` root | allowed as destination (only *destructive on* root is refused) | — | `🔵 decided 2026-11-23` — 卷根**作为目标目录允许**；仅对卷根本身做破坏性操作（删除/移动根）才拒，同 `refuse_volume_root`，与 TR-016 一致 | gaps |
| TR-016 | move volume **root** itself | move `D:\` | refused (`refuse_volume_root`, `mod.rs:379`) | all refuse | `✅ code` | CARGO `permanent_delete_removes_file_and_refuses_root` (analog) |
| TR-017 | copy `awam://…` | any transfer | refused (provider path is not a disk path) | — | `✅ code+test` | CARGO `rejects_awam_paths` (`mod.rs:936`) |

**Drag-drop modifiers** (D1 × operation): decided in `explorer-baseline.md §拖放` (同盘移动、跨盘复制；
Ctrl 强制复制，Shift 强制移动) — **not yet re-verified in the FE drag code this pass**, so the rows
are `🔵 decided` and must be pinned by a FE matrix or cargo-backed drop test before marked done.

| id | precondition | gesture | expected (Windows) | divergence | AFM state |
| --- | --- | --- | --- | --- | --- |
| TR-101 | drag item within same volume | drop on folder | **move** | Files: move; DC: move | `🔵 decided` (was: "app-internal drop always move" → needs to honor same/cross) |
| TR-102 | drag item to another volume | drop | **copy** | — | `🔵 decided` |
| TR-103 | same volume | drag + **Ctrl** | force **copy** | — | `🔵 decided` |
| TR-104 | cross volume | drag + **Shift** | force **move** | — | `🔵 decided` |
| TR-105 | any | drag + **Alt** | create **shortcut** | Explorer: yes; DC: `.lnk` | `🔵 decided` (explorer-baseline lists shortcut as 缺失) |
| TR-106 | drag file out to Explorer | OLE drop | Explorer copies (default cross-process) | — | `🟡 code` (`file_ops` OLE copy) |
| TR-107 | drag folder onto itself via mouse | drop on own subtree | refused, like TR-006 | — | `🔵 decided 2026-11-23` — 拒绝，复用 TR-006 的 `is_descendant_or_same` |

**Reparse points** (D2) — copy/move semantics are a common bug source:

| id | item | action | expected (Windows) | divergence | AFM state |
| --- | --- | --- | --- | --- | --- |
| TR-201 | broken junction | **delete permanent** | unlink the junction only; target contents (if any) untouched | DC: may recurse (footgun) | `✅ code+test` (`mod.rs:895`) |
| TR-202 | junction → real dir | delete permanent | remove link only, target remains | — | `✅ code+test` (`mod.rs:856`) |
| TR-203 | dir symlink / junction | copy to other dir | 跟随链接，复制实际内容（对齐 Explorer 默认）；右键另设「复制为链接」入口给高级场景 | DC/Linux 默认复制链接本身 | `🔵 decided` — follow on copy; provide "Copy as link" context-menu alt | **decided 2026-11-22** |
| TR-204 | deep tree with internal symlink cycle | copy | must not infinitely recurse (`follow_links(false)` in byte-count; copy path must match) | — | `✅ code` (`mod.rs:338`) verify copy path (gaps) |

### Group 4 · Name & structure (NM)

| id | precondition | action | expected (Windows) | divergence | AFM state | guard |
| --- | --- | --- | --- | --- | --- | --- |
| NM-001 | single file | F2 rename | inline edit, **stem selected, extension not** | — | `🟡 code` | `check-navigator-inline-rename.mjs` |
| NM-002 | rename to existing name | F2 → commit | **Replace / Don't replace(Skip)** conflict dialog (reuse the paste name-conflict UI); rename does **not** auto keep-both | macOS Finder: 仅 OK/Cancel 报错；**AFM 对齐 Explorer** | `🔵 decided 2026-11-23` — 当前后端 `rename_path` 撞名返回 "already exists" error；决定改为返回可识别冲突信号，FE 调用 name-conflict 对话框后以 Replace/Skip 重试 | CARGO `rename_path` 冲突信号 + FE name-conflict 复用；待加 |
| NM-003 | rename to reserved name (CON, NUL) | F2 | rejected (invalid name) | all reject on NTFS | `🔵 decided` — `validate_file_name` 需补拒绝 CON/PRN/AUX/NUL/COM1-9/LPT1-9（不区分大小写、含点后缀如 `CON.txt`） | **decided 2026-11-22; code change needed** |
| NM-004 | rename changing **only case** (a.txt→A.txt) same dir | F2 | must succeed as case change, not "already exists" | case-insensitive FS subtlety | `🔵 decided` — 允许；`rename_path` 的 `dest != src` 在大小写不敏感 FS 上自然通过，需 CARGO 钉住避免被改成大小写敏感比较 | **decided 2026-11-22** |
| NM-005 | multi-select, F2 rename | batch rename | numbering `stem (n).ext` | Explorer 11: numbered names; DC: template rename | `🟡 code` (batch numbering exists) | `check-navigator-inline-rename.mjs` |
| NM-006 | rename with invalid char `:*?"<>|/\` | F2 | rejected "invalid characters" | — | `✅ code+test` | CARGO `validate_name_blocks_separators` |
| NM-007 | name with trailing dot/space | F2 | NTFS strips/rejects trailing dot/space | DC: allows | `🔵 decided` — strip 后继续，静默接受（与 Windows API 行为一致） | **decided 2026-11-22; code change needed** |
| NM-008 | current dir, Ctrl+Shift+N | new folder | "New folder" created, selected, **rename active** | Files: same | `🟡 code` | `check-*.mjs` new-folder |
| NM-009 | current dir | new file (ShellNew) | template file created, selected, rename active | DC: from its own list | `✅ code` (`new_templates.rs`, `file_ops_new_templates`) | new-file-rename gate (H8) |
| NM-010 | target name collides on new | create | "already exists" error | — | `✅ code` (`mod.rs:431`) | CARGO roundtrip |

### Group 5 · Delete & recycle (DEL)

Decided in `explorer-baseline.md §删除`: Delete → recycle if possible, else permanent (confirmed);
Shift+Delete → confirmed permanent; volume root never deleted. UNC cannot recycle (`mod.rs:183`).

| id | precondition | action | expected (Windows) | divergence | AFM state | guard |
| --- | --- | --- | --- | --- | --- | --- |
| DEL-001 | local file | Delete | → Recycle Bin; selectable set updates | — | `✅ code+test` | CARGO `delete_to_recycle_local_file`; `check-recycle-bin.mjs` |
| DEL-002 | local file | Delete, confirm-before-delete **on** (default) | confirmation dialog first | DC: opt-in | `✅ code+test` | `check-delete-confirm.mjs`→`confirm-before-delete-checks.ts` |
| DEL-003 | removable **FAT32/exFAT** volume or UNC (判断依据 = 卷文件系统类型，**非可移动性**；NTFS 可移动盘仍可回收) | Delete | cannot recycle → **permanent delete with confirm** (not silent, not refused) | Explorer: permanent w/ confirm | `🔴 bug → 🔵 decided 2026-11-23` — FE 分流链（`pathsWithoutRecycle`→`confirmPermanentDelete`→`deleteEntries`）已存在，但判定源 `path_can_use_recycle_bin` (`mod.rs:209`) **只挡 UNC(`\\`/`//`)**，本地 FAT32/exFAT（无 `$RECYCLE.BIN`）被误判为可回收 → `trash::delete` 经 Shell 在该卷**静默永久删除、无确认**。决定：判定改为「仅 NTFS → 可回收，其余(含 FAT32/exFAT/UNC) → 不可回收并弹确认」 | 改 `file_ops_can_recycle` 判定 + 新增 CARGO（按 FS 类型，需 temp/卷夹具） |
| DEL-004 | any | Shift+Delete | permanent, after confirm | — | `✅ code` (`delete_permanent_paths`) | needs a Shift+Delete-level gate |
| DEL-005 | volume root | Delete / Shift+Delete | refused | all refuse | `✅ code` (`refuse_volume_root`) | CARGO `..._refuses_root` |
| DEL-006 | folder in Recycle Bin | restore | returns to **original path** (or best match) | — | `🟡 code` (`file_ops_restore`) | undo-restore loopback has no temp-exec test (ledger gap R2) |
| DEL-007 | original path gone at restore | restore | recreated / best-match, not error-only | Explorer recreates path | `🔵 decided` — 自动重建原父目录链（对齐 Explorer） | **decided 2026-11-22** |
| DEL-008 | multi select with mixed recyclable + UNC | Delete | per-item split: recycle where possible, permanent(confirm) for the rest | — | `✅ code (FE-side)` — 同 DEL-003；FE 流程在 `blocked.length` 分支里同时处理 `recyclable` 和 `doomed`，部分失败时 `removePathsFromListings(done)` 回滚已删项 | **verified 2026-11-22** |

### Group 6 · Conflict & keep-both (CF)

`parse_name_conflict` default = **keep-both** (`mod.rs:208`); FE dialog in `name-conflict-checks.ts`
with a short-circuit on a remembered default.

| id | precondition | action | expected (Windows) | divergence | AFM state | guard |
| --- | --- | --- | --- | --- | --- | --- |
| CF-001 | 1 conflict | paste, choose Keep both | `name - Copy.ext` | Explorer same | `✅ code+test` | CARGO `..._skip_...` + name-conflict |
| CF-002 | 1 conflict | Replace | dest content overwritten, staged verify first | — | `✅ code+test` | CARGO `replace_overwrites_and_skip_leaves_existing` |
| CF-003 | N conflicts in one paste | choose Replace once | "Apply to all" state honored for remaining | Explorer: per + all | `🔵 decided` — apply-to-all 仅本次操作内（一次粘贴批次）生效；下次粘贴重新问，与会话/应用无关 | **decided 2026-11-22** |
| CF-004 | conflict | tick "do this for all" and pick Skip | subsequent conflicts auto-skipped | — | `🟡 code` (remembered default short-circuit) | `name-conflict-checks.ts` |
| CF-005 | `Foo.txt` exists, paste `foo.txt` (case-insensitive FS) | keep-both | treated as **collision** → `Foo - Copy.txt` (case-insensitive compare) | FAT/exFAT/NTFS all case-insensitive; Linux FS differs | `🔵 decided` — 视为同名冲突；`unique_dest_path` 的存在性比较必须按 FS 规则（Windows 大小写不敏感） | **decided 2026-11-22** |
| CF-006 | same name, dest is dir, source is file | conflict | file-vs-dir prompt (TR-013) | — | `🔵 decided` — 同 TR-013，专门弹窗 | **decided 2026-11-22** |
| CF-007 | merge dir into dir same name, no inner collision | paste | **merge** contents | Explorer merges; DC merges | `🔵 decided` — 合并内容；safe_io 需支持递归 merge（当前是否 merge 未验证，列入待测） | **decided 2026-11-22** |
| CF-008 | merge dir into dir, inner file collides | paste | recurse conflict handling per inner file | — | `🔵 decided` — 内层冲突走 CF-001~005 递归；不一次性吞掉所有冲突 | **decided 2026-11-22** |

### Group 7 · Undo / redo (UN)

FE undo exists (`file-undo-checks.ts`, `check-file-undo.mjs`). Decided in `explorer-baseline.md`:
Ctrl+Z undoes copy / move / rename / new / recycle-delete.

| id | precondition | action | expected (Windows) | divergence | AFM state | guard |
| --- | --- | --- | --- | --- | --- | --- |
| UN-001 | just copied files in | Ctrl+Z | remove the copies | — | `✅ code` | `check-file-undo.mjs`→`file-undo-checks.ts` |
| UN-002 | just moved files | Ctrl+Z | move back to original location | cross-volume undo restores to source vol | `🟡 code` | needs cross-vol case (gaps) |
| UN-003 | just renamed | Ctrl+Z | original name restored | — | `✅ code` | `check-file-undo.mjs` |
| UN-004 | just recycle-deleted | Ctrl+Z | **restore from Recycle Bin** to original path | — | `🟡 code` | loopback not temp-exec tested (ledger R2) |
| UN-005 | just created folder/file | Ctrl+Z | delete the created item | — | `✅ code` | `check-file-undo.mjs` |
| UN-006 | undo several ops in sequence | Ctrl+Z ×n | reverse order, each correctly undone | — | `✅ code+test` — stack is strict LIFO; FE unit drains copy/create/rename/move/delete last→first | `check-file-undo.mjs`→`file-undo-checks.ts` |
| UN-007 | undo after the app was **closed/reopened** | Ctrl+Z | no undo across restart | — | `✅ code+test` — in-memory only, matches Explorer; grep pins no `localStorage`/`sessionStorage` in `fileUndo/*` | `check-file-undo.mjs` |
| UN-008 | an external program modified the target between op and undo | Ctrl+Z | refuse / warn, do not clobber foreign change | — | `🔵 decided` — 检测后拒绝 undo，toast「项目已被外部修改，无法撤销」；不弹模态 | **decided 2026-11-22** |
| UN-009 | undo a **partial/cancelled** transfer | Ctrl+Z | remove the partial results, source (if not lost) intact | ties to TR-011/LC | `✅ code+test` — cancel rollback is the transfer's teardown, never pushed; Ctrl+Z undoes the previous complete op | `check-file-undo.mjs` |
| UN-010 | redo (Ctrl+Y) after undo | Ctrl+Y | no file-ops redo | DC has redo; AFM will not | `🔵 decided` — **no redo**, matches Explorer | **decided 2026-11-22** |

### Group 8 · Lifecycle & post-state (LC)

Progress event `file-ops-progress` with byte emitter (`mod.rs:500`); no known cancel path in code
read this pass.

| id | precondition | action | expected (Windows) | divergence | AFM state | guard |
| --- | --- | --- | --- | --- | --- | --- |
| LC-001 | copy into empty dir done | auto | **new item selected** | Explorer selects new | `🔵 decided 2026-11-23` — 选中新项（对齐 Explorer；FE 验证待做） | gaps |
| LC-002 | move done | auto | new (moved) item selected | — | `🔵 decided 2026-11-23` — 选中新项（对齐 Explorer；FE 验证待做） | gaps |
| LC-003 | recycle-delete done | auto | selection moves to **neighbor** (Explorer) / keep-set (DC) | **有意差异**：与 Explorer 的「选邻居」不同，AFM 保持选区 | `🔵 decided 2026-11-23` — 保持现有 `delete-keep-selection` 的「保持选区」行为，**不**改为邻居 | `check-delete-keep-selection` |
| LC-004 | large copy running | — | non-blocking UI, byte progress + speed + remaining | — | `🟡 code` (`ProgressEmitter`) | `check-file-ops.mjs` |
| LC-005 | large copy running | cancel | **rollback**: remove partial destination results; source intact (guaranteed by safe_io staging regardless) | Explorer keeps partial; DC keeps; **AFM chooses rollback** | `🔵 decided` — rollback on cancel | **decided 2026-11-22** |
| LC-006 | file **in use** by another process | copy out | copy succeeds (read) | move-in fails: "in use" | `🔵 decided 2026-11-23` — copy 出**成功**；move-in / replace 失败报 **"in use"** 错误类 | gaps |
| LC-007 | **access denied** on one item of multi | transfer | **dialog per failed item: retry / skip / abort**, aligns with Windows Explorer | DC similar; some managers abort whole | `🔵 decided` — per-item dialog (retry/skip/abort) | **decided 2026-11-22** |
| LC-008 | **disk full** on dest volume | transfer | error at the failing item, source intact, partial cleanup state known | — | `🔵 decided 2026-11-23` — 复用 LC-007 已定的**逐项 retry/skip/abort**，不单独发明；源保持安全（LC-005 回滚语义） | gaps |
| LC-009 | **name too long** / path >260 | transfer | error for that item (\\?\ long-path support stance) | DC handles long paths | `🔵 decided` — 启用 `\\?\` 路径处理（Tauri manifest 加 `longPathAware=true` + 路径拼接以 `\\?\` 前缀），支持长路径 | **decided 2026-11-22; code change needed** |
| LC-010 | **OneDrive placeholder** copied | copy | hydrates-on-copy vs copy-dehydrated stance; Explorer hydrates | real divergence | `🔵 decided` — 复制时从云端水合（与 Explorer 一致）；后续可加「不下载复制」高级选项 | **decided 2026-11-22** |
| LC-011 | operation done | auto | scroll position & current sort preserved | — | `🔵 decided 2026-11-23` — 保持滚动位置与当前排序，不跳回顶部 | gaps |

### Group 1 · Selection & focus (SEL)

Verified in `explorer-baseline.md §选择` as 对齐: single, Ctrl, Shift, Ctrl+A, marquee, arrow keys
(with Shift-extend). Matrix pins each as a guardable case.

| id | precondition | action | expected (Windows) | divergence | AFM state | guard |
| --- | --- | --- | --- | --- | --- | --- |
| SEL-001 | list | single click | select that item, clear others | — | `🟡 code` | `check-list-select.mjs` |
| SEL-002 | items | Ctrl+click unselected | add to selection | — | `🟡 code` | `check-list-select.mjs` |
| SEL-003 | items | Ctrl+click selected | remove from selection | — | `🟡 code` | `check-list-select.mjs` |
| SEL-004 | anchor set | Shift+click | range from anchor to target **in current sort/view order** | DC same | `🔵 decided 2026-11-23` — 按**当前排序/视图顺序**计算（已共识，FE 验证待做） | gaps |
| SEL-005 | items | Ctrl+A | select all **visible** (post-filter) | — | `🟡 code` | `check-list-select.mjs` |
| SEL-006 | empty space | marquee drag | rubber-band selects intersecting items | all support | `🟡 code` | `check-list-select.mjs` |
| SEL-007 | marquee over folder A and A/child both visible | marquee | selection may contain parent+child; transfer must not double-process | bug source | `🔵 decided 2026-11-23` — 一次传输对 parent+child **去重**（后代只处理一次；FE reducer / 后端 dedup） | gaps |
| SEL-008 | items | arrow Up/Down, Home/End | move selection single-step / jump | — | `🟡 code` | arrows wired (explorer-baseline) |
| SEL-009 | items | type-ahead (first letters) | jump to matching item | DC quick search differs | `✅ code` | spec `2026-11-18-type-ahead-navigation-design.md` |
| SEL-010 | any selection | Esc | clear selection | Explorer clears / or cancels rename | `🔵 decided` — 三层优先级：rename inline 中 → 取消 rename；有选中 → 清空选中；无选中 → 不消费（交给全局） | **decided 2026-11-22** |
| SEL-011 | select a run, then paste parent+child in one op | transfer | each path processed **once** (dedup descendants) | — | `🔵 decided 2026-11-23` — parent+child **去重**，后代只处理一次 | gaps |

---

## Part 4 — Open decisions surfaced by this matrix

### 已拍板（2026-11-22）

| 差异 | 决定 |
| --- | --- |
| 撤销跨重启 (UN-007) | 不跨重启，in-memory only，对齐 Explorer |
| Redo (UN-010) | 不做文件操作 redo，对齐 Explorer |
| 传输中取消 (LC-005) | **回滚**：清除目标端已完成的部分结果；源端始终安全（staging 保证） |
| 单项失败错误处理 (LC-007) | **弹窗**：retry / skip / abort，对齐 Windows Explorer |
| UNC/不可回收卷 Delete (DEL-003, DEL-008) | **FE 已实现**：`NavigatorView.vue::shortcutDelete` 与 `NavigatorContextMenu.vue::runDelete` 调 `pathsWithoutRecycle` → `confirmPermanentDelete` → 分流 `deleteEntries(false/true)`，混合选择按各项能否回收各自处理，部分失败时回滚。后端 `mod.rs:192` 拒绝分支是 FE 流程被绕过时的兜底防御，正常路径不触发；保留作 defense-in-depth。PR-1 实施 review 中 verified。 |
| 大小写冲突 (CF-005) | 视为同名冲突，进入 replace/skip/keep-both 询问；`unique_dest_path` 存在性比较按 FS 规则（Windows 不敏感） |
| 同名不同类型 (TR-013, CF-006) | 专门弹窗「这会替换文件夹」警告类型不同，提供 replace / skip / keep-both |
| 目录入目录 (CF-007, CF-008) | 合并内容；内层冲突按 CF-001~005 递归处理；safe_io 需支持递归 merge |
| 符号链接 / junction 复制 (TR-203) | 跟随链接复制内容（对齐 Explorer）；右键另设「复制为链接」入口 |
| 保留名 (NM-003) | 拒绝创建 / 重命名为 CON/PRN/AUX/NUL/COM1-9/LPT1-9（不区分大小写，含 `CON.txt` 这种带点后缀） |
| 仅大小写改名 (NM-004) | 允许（与 Windows API 一致），需 CARGO 钉住不被改成大小写敏感比较 |
| 尾点 / 尾空格 (NM-007) | strip 后继续，静默接受（与 Windows API 行为一致） |
| OneDrive 占位符 (LC-010) | 复制时从云端水合（与 Explorer 一致）；后续可加「不下载复制」高级选项 |
| Apply-to-all 范围 (CF-003, TR-014) | 仅本次粘贴操作内生效；下次粘贴重新问，与会话/应用生命周期无关 |
| Cut 路径外部消失 (CLIP-008) | 跳过失效路径，继续其他项，最后 toast「N 项已跳过」；不弹 retry/skip/abort |
| 还原原父目录不存在 (DEL-007) | 自动重建父目录链（对齐 Explorer） |
| Undo 被外部污染 (UN-008) | 检测后拒绝 undo，toast「项目已被外部修改，无法撤销」；不强行执行不静默 skip |
| Undo cancel 的回滚 (UN-009) | cancel + rollback 是传输本身的收尾，**不入 undo 栈**；Ctrl+Z 撤销上一个完整操作 |
| 长路径 >260 (LC-009) | 启用 `\\?\` 路径处理（Tauri manifest `longPathAware=true` + 代码层前缀拼接） |
| Esc 行为边界 (SEL-010) | 三层：rename inline → 取消 rename；有选中 → 清空选中；无选中 → 不消费 |
| 回收卷判定依据 (DEL-003) | **按卷文件系统类型判定**：仅 NTFS 可进回收站；FAT32/exFAT/UNC 弹确认后永久删除。现状 `path_can_use_recycle_bin`(`mod.rs:209`) 只挡 UNC，FAT32/exFAT 被误判可回收→静默永久删除（**BUG，改后端判定，FE 链路自动生效**）。2026-11-23 拍板 |
| 重命名撞名 (NM-002) | 复用 name-conflict 对话框（**Replace / Skip**），对齐 Explorer；后端 `rename_path` 需返回可识别冲突信号而非笼统 error。2026-11-23 拍板 |

### 已拍板（2026-11-23）

| 差异 | 决定 |
| --- | --- |
| 粘贴后选中 (CLIP-003, CLIP-004) | 复制/剪切粘贴成功后 → **选中新写入项**（对齐 Explorer） |
| 粘贴目标=选中文件夹 (CLIP-101) | 选中一个文件夹时粘贴 → 目标=**粘入该文件夹** |
| 粘贴目标=文件父目录 (CLIP-102) | 选中一个文件时粘贴 → 目标=**该文件的父目录** |
| 粘贴目标=框选混合 (CLIP-104) | 框选混合(非单一容器)时粘贴 → 目标=**当前目录** |
| 卷根作为目标 (TR-015) | 卷根**作为目标目录允许**；仅对卷根本身做破坏性操作（删除/移动根）才拒，同 `refuse_volume_root`，与 TR-016 一致 |
| 鼠标拖入自身子树 (TR-107) | 拒绝，复用 TR-006 的 `is_descendant_or_same` |
| 多步 undo 顺序 (UN-006) | **逆序逐项撤销**（标准栈语义） |
| 复制/移动后选中 (LC-001, LC-002) | 完成后**选中新项**（对齐 Explorer；FE 验证待做） |
| 删除后选中 (LC-003) | 保持现有 `delete-keep-selection` 的「保持选区」行为，**不**改为邻居；与 Explorer「选邻居」为**有意差异** |
| 使用中文件错误类 (LC-006) | copy 出**成功**；move-in / replace 失败报 **"in use"** 错误类 |
| 目标盘写满 (LC-008) | 复用 LC-007 已定的**逐项 retry/skip/abort**，不单独发明；源保持安全（LC-005 回滚语义） |
| 操作后滚动/排序 (LC-011) | **保持滚动位置与当前排序**，不跳回顶部 |
| Shift 范围顺序 (SEL-004) | 按**当前排序/视图顺序**计算（已共识，FE 验证待做） |
| parent+child 去重 (SEL-007, SEL-011) | 一次传输对 parent+child **去重**（后代只处理一次；FE reducer / 后端 dedup） |

### 仍需拍板

The 2026-11-06 `explorer-baseline.md §已拍板` already settled clipboard→system, conflict→ask,
drag→same/cross rules, and delete→recycle-else-permanent. This matrix surfaced **new** cells that
were not in that decision list and need a stance before they can be guarded. Grouped by blast
radius.

**High (correctness / data-safety):**
- ~~**DEL-003 / DEL-008**~~ — **VERIFIED 2026-11-22 (PR-1 review)**: FE `shortcutDelete` 和
  `runDelete` 已实现「pathsWithoutRecycle → confirmPermanentDelete → 分流 deleteEntries(false/true)」，
  与 2026-11-06 已拍板一致。矩阵初稿误把后端 `mod.rs:192` 拒绝分支当成与拍板矛盾 —— 实际那是 FE
  流程被绕过时的 defense-in-depth 兜底，正常路径不会触发。无需改后端代码；后续 PR 应加 FE-UNIT
  或更紧的 grep 把 `pathsWithoutRecycle` 在两个入口里的调用钉住（详见 §Part 5）。
- ~~**LC-005**~~ — **DECIDED 2026-11-22**: cancel mid-transfer → **rollback** (remove partial
  destination results). Source safety already guaranteed by safe_io staging.
- ~~**UN-007 / UN-010**~~ — **DECIDED 2026-11-22**: undo is in-memory only (no cross-restart), no
  file-ops redo. Matches Windows Explorer.
- ~~**LC-007**~~ — **DECIDED 2026-11-22**: per-item failure shows a dialog with retry / skip / abort
  options, aligning with Windows Explorer behavior.

**Medium (consensus subtleties):**
- ~~**CF-005**~~ — **DECIDED 2026-11-22**: 大小写不敏感 FS 上视为同名冲突；`unique_dest_path` 存在性比较按 FS 规则。
- ~~**CF-007 / CF-008**~~ — **DECIDED 2026-11-22**: 目录入目录**合并**内容；内层冲突按 CF-001~005 递归处理；safe_io 需支持递归 merge。
- ~~**TR-013 / CF-006**~~ — **DECIDED 2026-11-22**: 同名不同类型专门弹窗警告（"这会替换文件夹"），提供 replace / skip / keep-both。
- ~~**TR-203**~~ — **DECIDED 2026-11-22**: 复制符号链接 / junction 时跟随链接复制内容（对齐 Explorer）；右键另设「复制为链接」入口。
- ~~**NM-003 / NM-004 / NM-007**~~ — **DECIDED 2026-11-22**: 拒绝保留名（CON/PRN/AUX/NUL/COM1-9/LPT1-9，不区分大小写、含点后缀）；允许仅大小写改名；尾点尾空格 strip 后静默接受。`validate_file_name` (`mod.rs:132`) 需补强。
- ~~**LC-010**~~ — **DECIDED 2026-11-22**: OneDrive 占位符复制时从云端水合（对齐 Explorer）；后续可加「不下载复制」高级选项。

**Low (post-state polish + remaining edges):**
- ~~**CF-003 / TR-014**~~ — **DECIDED 2026-11-22**: apply-to-all 仅本次粘贴内生效，下次粘贴重新问。
- ~~**CLIP-008**~~ — **DECIDED 2026-11-22**: cut 路径外部消失时跳过该项，继续其他，最后 toast。
- ~~**DEL-007**~~ — **DECIDED 2026-11-22**: 还原时原父目录不存在则自动重建（对齐 Explorer）。
- ~~**UN-008**~~ — **DECIDED 2026-11-22**: 检测 undo 污染后拒绝并 toast，不强行不静默。
- ~~**UN-009**~~ — **DECIDED 2026-11-22**: cancel + rollback 不入 undo 栈。
- ~~**LC-009**~~ — **DECIDED 2026-11-22**: 启用 `\\?\` 长路径处理（manifest `longPathAware=true`）。
- ~~**SEL-010**~~ — **DECIDED 2026-11-22**: Esc 三层优先级 rename > 清选 > 不消费。
- ~~**LC-001 / LC-002**~~ — **DECIDED 2026-11-23**: 复制/移动完成后选中新项（对齐 Explorer），仅缺 FE 验证。
- ~~**LC-003**~~ — **DECIDED 2026-11-23**: 删除后保持现有 `delete-keep-selection`「保持选区」行为，**不**改为邻居；与 Explorer「选邻居」为有意差异。
- ~~**SEL-004 / SEL-007 / SEL-011**~~ — **DECIDED 2026-11-23**: shift 范围按当前排序/视图顺序计算；一次传输对 parent+child 去重（后代只处理一次），仅缺 FE 验证。

Each open decision becomes, once resolved, either a `explorer-baseline.md` decision-table row (if it
changes a *stance*) or a new/updated case-id here (if it just narrows *expected*), and then a ledger
row.

---

## Part 5 — Migration list for the coverage ledger (not applied this pass)

Per your choice ("写完整基线文档 + 不补测试", "其余先和我决策再一次性完成"), this document does **not**
edit `external-behavior-baseline-coverage.md`. Below are the rows that should be added to its
`§Core file operations` table once the corresponding cases are actually guarded. They are staged
here so the ledger stays the single source of *coverage* truth and this file stays the single source
of *scenario* truth.

| ledger row (behavior) | case id(s) | proposed layer | status |
| --- | --- | --- | --- |
| paste-onto-self replace = no-op | TR-004 | CARGO (add explicit row) | new |
| folder-into-itself refused (copy) | TR-006, TR-007, TR-107 | CARGO negative | new |
| same-parent cut no-op | TR-008 | CARGO | new |
| cross-volume move source-safety (staging) | TR-010, TR-011, LC-005 | CARGO (needs 2-volume fixture / skip on CI) | open — fixture cost |
| same-name-different-type conflict | TR-013, CF-006 | FE + CARGO | new |
| case-only collision keep-both | CF-005 | FE + CARGO | new |
| dir-into-dir merge + inner conflict | CF-007, CF-008 | CARGO | new |
| rename reserved-name / case-only / trailing-dot | NM-003, NM-004, NM-007 | CARGO `validate_file_name` / rename | new |
| UNC/removable Delete → permanent fallback w/ confirm | DEL-003, DEL-008 | FE 已实现（grep + 调用链）；可选 FE-UNIT | **verified 2026-11-22** — `check-file-ops.mjs` 已 grep `confirmPermanentDelete`；本次扩展加 `pathsWithoutRecycle` 在两个入口里的调用钉住 |
| restore to original path when parent gone | DEL-007 | CARGO (heavy, see ledger R2) | **decided — 自动重建父目录链；CARGO 可走 temp 路径，不再依赖 R2 的机器态** |
| undo reverse-order multi-op | UN-006 | FE | guarded 2026-11-23 — `check-file-undo.mjs`→`file-undo-checks.ts` LIFO drain |
| paste-target resolution (folder/file/none) | CLIP-101, CLIP-102, CLIP-104 | FE (pure resolver extraction) | decided 2026-11-23 — 实现待做 |
| select new post-state (copy / move / paste) | LC-001, LC-002, CLIP-003, CLIP-004 | FE | decided 2026-11-23 — 实现待做 |
| delete post-state = keep selection (not neighbor) | LC-003 | FE (`delete-keep-selection`) | decided 2026-11-23 — 保持选区，有意差异，实现待做 |
| descendant dedup in one transfer | SEL-007, SEL-011, TR-006 | FE reducer | decided 2026-11-23 — 实现待做 |
| shift range in sorted view order | SEL-004 | FE | decided 2026-11-23 — 实现待做 |
| drag same/cross/force modifiers | TR-101..TR-105 | FE (grep now) → decide if unitable | open — invoke-fused |
| recycle-bin availability by **filesystem type** | DEL-003 | 改 `path_can_use_recycle_bin`/`file_ops_can_recycle`（仅 NTFS→可回收）+ CARGO | **decided 2026-11-23 — BUG fix, code change required** |
| rename-to-existing → **Replace/Skip dialog** | NM-002 | CARGO(后端返回冲突信号) + FE(复用 name-conflict 对话框) | **decided 2026-11-23 — code change required** |
| apply-to-all per-operation scope | CF-003, TR-014 | FE (dialog state) | new |
| clipboard stale-cut-path drop | CLIP-008 | FE 验证 + 后端 `transferEntries` 跳过分支 | new |
| undo contamination refusal | UN-008 | FE `file-undo-checks.ts` | new |
| cancel rollback excluded from undo stack | UN-009 (ties to LC-005) | FE reducer | guarded 2026-11-23 — `check-file-undo.mjs` asserts paste/drop callers early-return on `!result.ok` before `record*Undo` |
| in-use / disk-full errors reuse per-item retry/skip/abort | LC-006, LC-008 (→ LC-007) | FE + CARGO (error class) | decided 2026-11-23 — 实现待做 |
| long-path support via `\\?\` | LC-009 | CARGO + manifest 断言 | **decided — code change required** |
| Esc semantics (rename > clear > no-op) | SEL-010 | FE (key handler) | new |

Rule reminder from ADR 0025: an `#[ignore]`d or chain-absent gate protects nothing; cross-volume and
restore-pathback cases may need a 2-mount or machine-state fixture — if impractical, keep them as
honest grep with a `定论（不强做）` note like existing R1/R2, rather than fake FE units.

---

## Cross-links

- Behavior-vs-Explorer comparison and the settled decisions: [`explorer-baseline.md`](./explorer-baseline.md)
- Which guard covers which behavior (edit this when you wire cases): [`external-behavior-baseline-coverage.md`](./external-behavior-baseline-coverage.md)
- Baseline mechanism, layers, no-e2e: [ADR 0025](./adr/0025-external-function-regression-baseline.md)
- Process for new/changed user-visible features: `AGENTS.md §Testing new / changed features`
- Implementation source of truth: `src-tauri/src/file_ops/` (`mod.rs`, `safe_io.rs`, `clipboard.rs`,
  `recycle.rs`, `new_templates.rs`, `progress.rs`), `src/state/fileClipboard.ts`.
