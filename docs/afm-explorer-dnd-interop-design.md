# AFM ↔ Explorer drag-and-drop interop

Windows-first Hybrid outbound (HTML5 in-window → OLE/`CF_HDROP` on leave) and shell inbound. Related: AIA-240 (COPY|MOVE|LINK), AIA-261/264/266/268/280 (focus).

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

**If dogfood still fails:** subclass WebView child HWND and/or carefully try `LockSetForegroundWindow` — ask TZ before either.
