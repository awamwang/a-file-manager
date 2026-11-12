# AFM ↔ Explorer drag-and-drop interop

Windows-first Hybrid outbound (HTML5 in-window → OLE/`CF_HDROP` on leave) and shell inbound. Related: AIA-240 (COPY|MOVE|LINK), AIA-261/264/266/268/280 (focus).

**Status (FIL-7 MVP):** landed. Outbound uses `src/state/dnd/dragOut.ts` + patched `DoDragDrop`; inbound uses `dnd/paths` + `dnd/surface` (`commitFolderDrop`). Focus policy lives in `drag_focus.rs`. Guard: `npm run test:cross-app-dnd`.

## 8. Outbound focus — drag vs click (AIA-261 → AIA-264 → AIA-266 → AIA-268 → AIA-280)

**Problem.** Dragging a file from a background AFM window onto a foreground drop target (Explorer) must not raise AFM over the target. AIA-261 covered `MA_NOACTIVATE` + OLE prior-FG restore when FG was still the target at `DoDragDrop` start. AIA-264 added press-time prior FG + sticky restore after selection mousedown raised AFM first. AIA-266 deferred background row focus until mouse-up. Dogfood after `2983def` still raised on **press**: WebView2 often keeps `document.hasFocus()` while OS FG is Explorer, so the background-only `preventDefault` gate still allowed `tabindex` focus → host raise/z-order bump. AIA-268 then always `preventDefault` on row mousedown — that **cancelled Chromium HTML5 dragstart** entirely (AIA-280).

**Win32 / Explorer pattern.** Mousedown does **not** bring the app forward or finalize selection. Until mouse-up you are in the drag gesture. Selection + activation happen on mouse-up only if it was a click (not a completed drag). Use `SM_CXDRAG`/`SM_CYDRAG` (or `DragDetect`); never leave the source covering the pre-press foreground for a drag/`DoDragDrop`.

**AFM layers.**

| Layer | Behavior |
|---|---|
| `NavigatorView` rows | **Must not** `preventDefault` on mousedown (blocks `dragstart`). Rows `tabindex="-1"`. Selection/focus on `@click` (mouse-up). `dragstart` may `selectOnly` for payload **without** `focusEntryRow`. |
| `drag_focus.rs` | Background press → `MA_NOACTIVATE` + store prior FG/press point; also arm on `WM_PARENTNOTIFY`/`LBUTTONDOWN` if activate was missed; `WM_ACTIVATE` + poll timer undo WebView raises while LBUTTON down via `SetWindowPos(prior, HWND_TOP)` + `SetForegroundWindow`; past drag threshold → `note_drag_gesture`; mouse-up activates only if still pending (PARENTNOTIFY or timer fallback). |
| `patches/drag` | `resolve_preserve_foreground` prefers press-time prior FG even when AFM is already FG at OLE start; `GiveFeedback`/`QueryContinueDrag` restore it (AIA-264 sticky). |
| `fileDragOut.ts` | No focus APIs; Hybrid leave/edge/blur → `startDrag` only. |

**Manual check.** Place Explorer in front of partially visible AFM → press a file and drag onto Explorer without AFM jumping in front on press; mouse-up without drag still selects + activates; keyboard focus via `focusEntryRow` still works; outbound drag over other windows still works.

**FIL-37 (2026-11):** OLE `DropSource` / drag start·end 与 `drag_focus` 对齐——`SetWindowPos(prior, HWND_TOP, SWP_NOACTIVATE)` + 仅当我们持有 FG 时 `SetForegroundWindow(prior)`；出站 JS 禁止 `setFocus`。纯 click 仍在 mouse-up 激活。

**FIL-39 (2026-11):** Hybrid 仍窗内 HTML5（ADR 0012）；近边 24px + 离开客户区 / blur / 失焦，以及宿主在「拖势成立且 `WindowFromPoint` 不在根 HWND 树」时发 `afm://dnd/ole-promote`，提前进入 OLE。

**FIL-42 (2026-11):** Shell `IDataObject` 出站在 `DoDragDrop` 前设置 `CFSTR_PREFERREDDROPEFFECT`（随 `options.mode`，默认 COPY）并 `IDataObjectAsyncCapability::SetAsyncMode(FALSE)`，保证媒体播放器在 Drop 期间同步 `GetData(CF_HDROP)`；仍无 PotPlayer 私有格式。效果掩码保持 COPY|MOVE|LINK。

**FIL-43 (2026-11):** Press-phase deepen — subclass in-process WebView2 child HWNDs (`Chrome_WidgetWin_*` / Intermediate D3D / RenderWidgetHost) in addition to the main window. Child `WM_MOUSEACTIVATE` → `MA_NOACTIVATE` + early `WM_LBUTTONDOWN` arm of prior FG so Chromium cannot raise the host before sticky restore. Delayed re-scan after setup (`WM_PARENTNOTIFY`/`WM_CREATE` + short find timer). Permanent `WS_EX_NOACTIVATE` on the main window remains forbidden. Rows stay `tabindex=-1` with empty `onRowMouseDown` (no focus/setFocus on press).

**FIL-44 (2026-11):** Dogfood after FIL-43 still raised over target and PotPlayer/Sublime drops were intermittent. Root causes treated as (1) sticky restore too weak vs WebView re-raise, (2) Hybrid promote via JS/IPC arrived after the cursor was already over the target. Fixes: strengthen prior-FG restore with `AttachThreadInput` + place source under prior + timed `LockSetForegroundWindow` while we were wrongly FG; `dnd_arm_outbound_paths` so host leave/near-edge/`WindowFromPoint` promote can `start_files_drag_hwnd` in-process; JS still arms/falls back and pre-warms the drag icon. No per-app formats.

**FIL-45 (2026-11):** Dogfood: Sublime pasted path text (HTML5 `text/plain`/`uri-list`); PotPlayer showed copy-plus but did not play; press always raised AFM. Fixes: (1) row `mousedown` arms outbound paths; host starts OLE at `SM_CXDRAG` **without waiting to leave the window**; `dragstart` `preventDefault` cancels HTML5 outbound; HTML5 payload no longer sets text/uri-list/plain. (2) Temporary `WS_EX_NOACTIVATE` on the main window for the background-press gesture (cleared on mouse-up / after OLE), on top of MA_NOACTIVATE + sticky prior FG.

**FIL-46 (2026-11):** OLE play/open OK; maximized AFM still covered the target on press with no taskbar flash (WebView raises before `WM_MOUSEACTIVATE`). Fixes: enable temp `WS_EX_NOACTIVATE` on `WA_INACTIVE` (not only on press); `WH_MOUSE_LL` arms prior FG before the click is dispatched; `GetAncestor(GA_ROOT)` for OOP WebView hit-testing.

**FIL-47 (2026-11):** Retracted FIL-46 `HWND_TOPMOST` / unconditional `SetForegroundWindow(prior)`. That forced PotPlayer to the front on every click and broke OLE (`DoDragDrop` had no drag feedback / target ignored drops). Correct model: **do not activate AFM**; only if AFM **already stole** FG, hand it back. No TOPMOST on the drop target. Poll restore skipped while OLE is in progress.

**FIL-48 (2026-11):** Tighten further: `SetForegroundWindow(prior)` only when (1) this gesture `had_deferred_activate` (background press), and (2) AFM is currently FG. If another app is already FG → no-op. Foreground-initiated presses clear stale prior via `note_press_origin`. Do not invent prior from `LAST_OTHER_FG` during restore.
