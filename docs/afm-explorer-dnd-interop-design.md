# AFM ↔ Explorer drag-and-drop interop

Windows-first Hybrid outbound (HTML5 in-window → OLE/`CF_HDROP` on leave) and shell inbound. Related: AIA-240 (COPY|MOVE|LINK), AIA-261/264/266 (focus).

## 8. Outbound focus — drag vs click (AIA-261 → AIA-264 → AIA-266)

**Problem.** Dragging a file from a background AFM window onto a foreground drop target (Explorer) must not raise AFM over the target. AIA-261 covered `MA_NOACTIVATE` + OLE prior-FG restore when FG was still the target at `DoDragDrop` start. AIA-264 added press-time prior FG + sticky restore after selection mousedown raised AFM first. Dogfood after `23194fb` still raised on **press**: WebView focused `tabindex=0` rows on mousedown (select/activate-first), so the press left the drag-gesture path.

**Win32 / Explorer pattern.** Mousedown does **not** bring the app forward or finalize selection. Until mouse-up you are in the drag gesture. Selection + activation happen on mouse-up only if it was a click (not a completed drag). Use `SM_CXDRAG`/`SM_CYDRAG` (or `DragDetect`); never leave the source covering the pre-press foreground for a drag/`DoDragDrop`.

**AFM layers.**

| Layer | Behavior |
|---|---|
| `NavigatorView` rows | Background press → `preventDefault` on mousedown (no tabindex focus / raise). Selection stays on `@click` (mouse-up). `dragstart` may `selectOnly` for payload **without** `focusEntryRow`. Foreground keeps snappy focus-on-press. |
| `drag_focus.rs` | Background press → `MA_NOACTIVATE` + store prior FG/press point; also arm on `WM_PARENTNOTIFY`/`LBUTTONDOWN` if activate was missed; `WM_ACTIVATE` + poll timer undo WebView raises while LBUTTON down; past drag threshold → `note_drag_gesture`; mouse-up activates only if still pending (PARENTNOTIFY or timer fallback). |
| `patches/drag` | `resolve_preserve_foreground` prefers press-time prior FG even when AFM is already FG at OLE start; `GiveFeedback`/`QueryContinueDrag` restore it (AIA-264 sticky). |
| `fileDragOut.ts` | No focus APIs; Hybrid leave/edge/blur → `startDrag` only. |

**Manual check.** Place Explorer in front of partially visible AFM → press a file and drag onto Explorer without AFM jumping in front on press; mouse-up without drag still selects + activates; foreground click-select stays snappy and outbound drag over other windows still works.
