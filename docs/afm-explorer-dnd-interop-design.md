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

**If dogfood still fails (FIL-37 upgrade gate — ask TZ):**
- **WebView child HWND subclass** — can catch secondary raises on the WebView2 input path; couples host/child lifetime and widens crash surface.
- **`LockSetForegroundWindow`** — only works when already FG; can block other apps' foreground switches; use as a timed last resort only.
- Prefer matrix dogfood (see Project store `docs/cross-app-dnd-target-matrix.md`) before either.
