# AFM ↔ Explorer drag-and-drop interop

Windows-first Hybrid outbound (HTML5 in-window → OLE/`CF_HDROP` on leave) and shell inbound. Related: AIA-240 (COPY|MOVE|LINK), AIA-261/264 (focus).

## 8. Outbound focus — drag vs click (AIA-261 → AIA-264)

**Problem.** Dragging a file from a background AFM window onto a foreground drop target (Explorer) must not raise AFM over the target. AIA-261 covered `MA_NOACTIVATE` + OLE prior-FG restore when FG was still the target at `DoDragDrop` start. Users still saw raises because **selection mousedown activates first** (WebView/Chromium), so by HTML5/OLE start AFM was already FG and `preserve_fg` was `None`.

**Win32 pattern.** Explorer-style: answer `WM_MOUSEACTIVATE` with `MA_NOACTIVATE` when another top-level is FG; use `SM_CXDRAG`/`SM_CYDRAG` (or `DragDetect`) to tell drag from click; activate on mouse-up only for a plain click; never leave the source covering the pre-press foreground for a drag/`DoDragDrop`.

**AFM layers.**

| Layer | Behavior |
|---|---|
| `drag_focus.rs` | Background press → `MA_NOACTIVATE` + store prior FG/press point; `WM_ACTIVATE` + poll timer undo WebView raises while LBUTTON down; past drag threshold → `note_drag_gesture`; mouse-up activates only if still pending (no gesture/OLE). |
| `patches/drag` | `resolve_preserve_foreground` prefers press-time prior FG even when AFM is already FG at OLE start; `GiveFeedback`/`QueryContinueDrag` restore it. |
| `fileDragOut.ts` | No focus APIs; Hybrid leave/edge/blur → `startDrag` only. |

**Manual check.** Place Explorer in front of partially visible AFM → drag a file onto Explorer without AFM jumping in front; click (no drag) a background AFM window still activates on mouse-up.
