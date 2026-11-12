// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::{CursorPosition, DragItem, DragMode, DragResult, Image, Options};

use std::{
    ffi::c_void,
    iter::once,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering},
    sync::{Mutex, Once, OnceLock},
};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::{GetObjectW, BITMAP},
        System::Com::*,
        System::DataExchange::RegisterClipboardFormatW,
        System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE},
        System::Ole::{DoDragDrop, OleInitialize, ReleaseStgMedium},
        System::Ole::{
            IDropSource, IDropSource_Impl, CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK,
            DROPEFFECT_MOVE,
        },
        System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS},
        System::Threading::{AttachThreadInput, GetCurrentThreadId},
        UI::{
            Shell::{
                BHID_DataObject, CLSID_DragDropHelper, Common, IDataObjectAsyncCapability,
                IDragSourceHelper, IShellItemArray, SHCreateShellItemArrayFromIDLists,
                CFSTR_PREFERREDDROPEFFECT, CFSTR_SHELLIDLIST, SHDRAGIMAGE,
            },
            WindowsAndMessaging::{
                GetAncestor, GetCursorPos, GetForegroundWindow, GetWindowThreadProcessId,
                IsChild, LockSetForegroundWindow, SetForegroundWindow, SetWindowPos, GA_ROOT,
                HWND_TOP, LSFW_LOCK, LSFW_UNLOCK, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
                SWP_SHOWWINDOW,
            },
        },
    },
};

/// AIA-261/264: true while `DoDragDrop` is on the stack (Hybrid / plugin outbound).
static OLE_DRAG_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Host deferred activation after background `MA_NOACTIVATE` (cleared on drag / OLE / take).
static PENDING_BACKGROUND_ACTIVATE: AtomicBool = AtomicBool::new(false);

/// Press that crossed `SM_CXDRAG`/`SM_CYDRAG` (or entered OLE) — do not activate on mouse-up.
static DRAG_GESTURE: AtomicBool = AtomicBool::new(false);

/// Foreground HWND (as isize) at the background press that deferred activation.
static PRESS_PRIOR_FG: AtomicIsize = AtomicIsize::new(0);

/// Screen coordinates of that press (for drag-threshold detection).
static PRESS_X: AtomicI32 = AtomicI32::new(0);
static PRESS_Y: AtomicI32 = AtomicI32::new(0);

/// FIL-44: 已对前台切换上锁（后台拖出期间阻止 WebView 再抢 FG）。
static FG_LOCKED: AtomicBool = AtomicBool::new(false);

/// FIL-44: 本轮 arm 是否已有人启动 OLE（宿主直启与 JS startDrag 互斥）。
static OUTBOUND_CLAIMED: AtomicBool = AtomicBool::new(false);

/// FIL-44: Hybrid 出站路径（JS arm → 宿主近边/离窗直启 OLE，避免 IPC 竞态）。
fn outbound_paths_slot() -> &'static Mutex<Vec<PathBuf>> {
    static SLOT: OnceLock<Mutex<Vec<PathBuf>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(Vec::new()))
}

/// Whether a Windows OLE outbound drag is currently inside `DoDragDrop`.
pub fn ole_drag_in_progress() -> bool {
    OLE_DRAG_IN_PROGRESS.load(Ordering::SeqCst)
}

/// Record background `WM_MOUSEACTIVATE` → defer activation; remember prior FG + press point.
pub fn note_deferred_activate(prior_fg: isize, press_x: i32, press_y: i32) {
    PRESS_PRIOR_FG.store(prior_fg, Ordering::SeqCst);
    PRESS_X.store(press_x, Ordering::SeqCst);
    PRESS_Y.store(press_y, Ordering::SeqCst);
    DRAG_GESTURE.store(false, Ordering::SeqCst);
    PENDING_BACKGROUND_ACTIVATE.store(true, Ordering::SeqCst);
}

/// 记录按下原点（前台窗也可用于拖势阈值），不推迟激活、不改 prior FG。
pub fn note_press_origin(press_x: i32, press_y: i32) {
    PRESS_X.store(press_x, Ordering::SeqCst);
    PRESS_Y.store(press_y, Ordering::SeqCst);
}

/// Take-and-clear deferred activation; `false` once a drag gesture / OLE started.
pub fn take_pending_activate() -> bool {
    PENDING_BACKGROUND_ACTIVATE.swap(false, Ordering::SeqCst)
}

/// Mark the press as a drag (past system drag threshold or OLE). Suppresses click-activate.
pub fn note_drag_gesture() {
    DRAG_GESTURE.store(true, Ordering::SeqCst);
    PENDING_BACKGROUND_ACTIVATE.store(false, Ordering::SeqCst);
}

pub fn drag_gesture_active() -> bool {
    DRAG_GESTURE.load(Ordering::SeqCst)
}

/// Prior foreground captured at deferred-activate time (`0` if none).
pub fn press_prior_foreground() -> isize {
    PRESS_PRIOR_FG.load(Ordering::SeqCst)
}

pub fn press_origin() -> (i32, i32) {
    (
        PRESS_X.load(Ordering::SeqCst),
        PRESS_Y.load(Ordering::SeqCst),
    )
}

/// True while we must keep the pre-press foreground on top (pending click deferral or drag).
pub fn should_suppress_foreground_raise() -> bool {
    PENDING_BACKGROUND_ACTIVATE.load(Ordering::SeqCst)
        || DRAG_GESTURE.load(Ordering::SeqCst)
        || OLE_DRAG_IN_PROGRESS.load(Ordering::SeqCst)
}

/// End of the press (mouse-up) or after OLE finishes consuming the gesture.
pub fn clear_press_state() {
    unlock_foreground();
    PENDING_BACKGROUND_ACTIVATE.store(false, Ordering::SeqCst);
    DRAG_GESTURE.store(false, Ordering::SeqCst);
    PRESS_PRIOR_FG.store(0, Ordering::SeqCst);
}

/// Resolve which HWND to keep above AFM for the OLE lifetime.
/// Prefers the press-time prior FG (AIA-264: mousedown may have already raised us).
pub fn resolve_preserve_foreground(our_root: isize, current_fg: isize) -> Option<isize> {
    let prior = PRESS_PRIOR_FG.load(Ordering::SeqCst);
    if prior != 0 && prior != our_root {
        return Some(prior);
    }
    if current_fg == 0 || current_fg == our_root {
        None
    } else {
        Some(current_fg)
    }
}

/// FIL-44: JS `armNativeFileDragOut` 登记路径，供宿主离窗直启 OLE。
pub fn arm_outbound_paths(paths: Vec<PathBuf>) {
    let cleaned: Vec<PathBuf> = paths
        .into_iter()
        .filter(|p| !p.as_os_str().is_empty())
        .collect();
    OUTBOUND_CLAIMED.store(false, Ordering::SeqCst);
    if let Ok(mut slot) = outbound_paths_slot().lock() {
        *slot = cleaned;
    }
}

pub fn clear_outbound_paths() {
    if let Ok(mut slot) = outbound_paths_slot().lock() {
        slot.clear();
    }
}

/// 取出并清空已登记路径（宿主直启 OLE 用）。
pub fn take_outbound_paths() -> Vec<PathBuf> {
    match outbound_paths_slot().lock() {
        Ok(mut slot) => std::mem::take(&mut *slot),
        Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
    }
}

fn lock_foreground() {
    if FG_LOCKED.swap(true, Ordering::SeqCst) {
        return;
    }
    unsafe {
        let _ = LockSetForegroundWindow(LSFW_LOCK);
    }
}

fn unlock_foreground() {
    if !FG_LOCKED.swap(false, Ordering::SeqCst) {
        return;
    }
    unsafe {
        let _ = LockSetForegroundWindow(LSFW_UNLOCK);
    }
}

/// 用 press-time prior FG 做一次强化恢复（供宿主 timer / OLE 入口共用）。
pub fn restore_press_prior_foreground(our_root: isize) {
    let prior = PRESS_PRIOR_FG.load(Ordering::SeqCst);
    if prior == 0 || prior == our_root {
        return;
    }
    restore_preserved_foreground(HWND(our_root), Some(HWND(prior)));
}

mod image;

static mut OLE_RESULT: Result<()> = Ok(());
static OLE_UNINITIALIZE: Once = Once::new();
fn init_ole() {
    OLE_UNINITIALIZE.call_once(|| {
        unsafe {
            OLE_RESULT = OleInitialize(Some(std::ptr::null_mut()));
        }
        // I guess we never deinitialize for now?
        // OleUninitialize
    });
}

// FIL-38: 手写 IDataObject（EnumFormatEtc=E_NOTIMPL、仅 CF_HDROP）已移除。
// Files 出站只走 Shell `BHID_DataObject`（见 get_file_data_object）。

#[implement(IDropSource)]
struct DropSource {
    /// Root HWND of the AFM / drag-source window.
    our_root: HWND,
    /// Foreground window to keep on top when OLE started in the background
    /// (Hybrid leave/blur). `None` when we were already foreground.
    preserve_fg: Option<HWND>,
}

#[implement(IDropSource)]
struct DummyDropSource(());

impl DropSource {
    fn new(our_root: HWND, preserve_fg: Option<HWND>) -> Self {
        Self {
            our_root,
            preserve_fg,
        }
    }
}

fn hwnd_is_ours(our_root: HWND, candidate: HWND) -> bool {
    if candidate.0 == 0 || our_root.0 == 0 {
        return false;
    }
    if candidate == our_root {
        return true;
    }
    unsafe { IsChild(our_root, candidate).as_bool() }
}

fn root_hwnd(hwnd: HWND) -> HWND {
    unsafe {
        let root = GetAncestor(hwnd, GA_ROOT);
        if root.0 == 0 {
            hwnd
        } else {
            root
        }
    }
}

/// FIL-37/44/47: 仅当我们**已经抢到前台**时，把 prior 还回去并把自身压到其下。
/// 禁止 HWND_TOPMOST / 无条件 SetForegroundWindow(prior)——那会把播放器强拉前台并打坏 DoDragDrop。
/// Explorer 做法是源窗不激活，而不是去置顶目标窗。
fn restore_preserved_foreground(our_root: HWND, preserve_fg: Option<HWND>) {
    let Some(prev) = preserve_fg else {
        return;
    };
    if prev.0 == 0 || hwnd_is_ours(our_root, prev) {
        return;
    }
    unsafe {
        let fg = GetForegroundWindow();
        // 我们不是 FG：不要动 prior（它本就该在前台）；乱 SetForegroundWindow 会打坏 OLE。
        if !hwnd_is_ours(our_root, fg) {
            return;
        }
        lock_foreground();

        let mut fg_pid = 0u32;
        let fg_tid = GetWindowThreadProcessId(fg, Some(&mut fg_pid));
        let mut prev_pid = 0u32;
        let prev_tid = GetWindowThreadProcessId(prev, Some(&mut prev_pid));
        let cur_tid = GetCurrentThreadId();
        let attached_fg = fg_tid != 0
            && fg_tid != cur_tid
            && AttachThreadInput(cur_tid, fg_tid, TRUE).as_bool();
        let attached_prev = prev_tid != 0
            && prev_tid != cur_tid
            && prev_tid != fg_tid
            && AttachThreadInput(cur_tid, prev_tid, TRUE).as_bool();

        let _ = SetWindowPos(
            prev,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
        );
        let _ = SetForegroundWindow(prev);
        if our_root.0 != 0 {
            let _ = SetWindowPos(
                our_root,
                prev,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }

        if attached_prev {
            let _ = AttachThreadInput(cur_tid, prev_tid, FALSE);
        }
        if attached_fg {
            let _ = AttachThreadInput(cur_tid, fg_tid, FALSE);
        }
    }
}

#[allow(non_snake_case)]
impl IDropSource_Impl for DropSource {
    fn QueryContinueDrag(&self, fescapepressed: BOOL, grfkeystate: MODIFIERKEYS_FLAGS) -> HRESULT {
        restore_preserved_foreground(self.our_root, self.preserve_fg);
        if fescapepressed.as_bool() {
            DRAGDROP_S_CANCEL
        } else if (grfkeystate & MK_LBUTTON) == MODIFIERKEYS_FLAGS(0) {
            DRAGDROP_S_DROP
        } else {
            S_OK
        }
    }

    fn GiveFeedback(&self, _dweffect: DROPEFFECT) -> HRESULT {
        restore_preserved_foreground(self.our_root, self.preserve_fg);
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

impl DummyDropSource {
    fn new() -> Self {
        Self(())
    }
}

#[allow(non_snake_case)]
impl IDropSource_Impl for DummyDropSource {
    fn QueryContinueDrag(&self, fescapepressed: BOOL, grfkeystate: MODIFIERKEYS_FLAGS) -> HRESULT {
        if fescapepressed.as_bool() || (grfkeystate & MK_LBUTTON) == MODIFIERKEYS_FLAGS(0) {
            DRAGDROP_S_CANCEL
        } else {
            S_OK
        }
    }

    fn GiveFeedback(&self, _dweffect: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

pub fn start_drag<W: HasWindowHandle, F: Fn(DragResult, CursorPosition) + Send + 'static>(
    handle: &W,
    item: DragItem,
    image: Image,
    on_drop_callback: F,
    options: Options,
) -> crate::Result<()> {
    if let Ok(RawWindowHandle::Win32(w)) = handle.window_handle().map(|h| h.as_raw()) {
        let our_hwnd = HWND(w.hwnd.get() as isize);
        match item {
            DragItem::Files(files) => {
                let mut paths = Vec::new();
                for f in files {
                    paths.push(dunce::canonicalize(f)?);
                }
                let result = run_files_ole_drag(our_hwnd, &paths, Some(image), options.mode)?;
                match result {
                    Some(DragResult::Dropped) => {
                        let mut pt = POINT { x: 0, y: 0 };
                        unsafe {
                            GetCursorPos(&mut pt)?;
                        }
                        on_drop_callback(DragResult::Dropped, CursorPosition { x: pt.x, y: pt.y });
                    }
                    Some(DragResult::Cancel) => {
                        let mut pt = POINT { x: 0, y: 0 };
                        unsafe {
                            GetCursorPos(&mut pt)?;
                        }
                        on_drop_callback(DragResult::Cancel, CursorPosition { x: pt.x, y: pt.y });
                    }
                    None => {}
                }
            }
            DragItem::Data { .. } => {
                init_ole();
                unsafe {
                    #[allow(static_mut_refs)]
                    if let Err(e) = &OLE_RESULT {
                        return Err(e.clone().into());
                    }
                }

                let paths = vec![dunce::canonicalize("./")?];

                let data_object: IDataObject = get_file_data_object(&paths).unwrap();
                let drop_source: IDropSource = DummyDropSource::new().into();

                unsafe {
                    if let Some(drag_image) = get_drag_image(image) {
                        if let Ok(helper) =
                            create_instance::<IDragSourceHelper>(&CLSID_DragDropHelper)
                        {
                            let _ = helper.InitializeFromBitmap(&drag_image, &data_object);
                        }
                    }

                    let mut out_dropeffect = DROPEFFECT::default();
                    note_drag_gesture();
                    OLE_DRAG_IN_PROGRESS.store(true, Ordering::SeqCst);
                    let drop_result = DoDragDrop(
                        &data_object,
                        &drop_source,
                        DROPEFFECT_COPY,
                        &mut out_dropeffect,
                    );
                    OLE_DRAG_IN_PROGRESS.store(false, Ordering::SeqCst);
                    clear_press_state();
                    let mut pt = POINT { x: 0, y: 0 };
                    GetCursorPos(&mut pt)?;
                    if drop_result == DRAGDROP_S_DROP {
                        on_drop_callback(DragResult::Dropped, CursorPosition { x: pt.x, y: pt.y });
                    } else {
                        // DRAGDROP_S_CANCEL
                        on_drop_callback(DragResult::Cancel, CursorPosition { x: pt.x, y: pt.y });
                    }
                }
            }
        }
        Ok(())
    } else {
        Err(crate::Error::UnsupportedWindowHandle)
    }
}

/// FIL-44: 宿主在离窗/近边时用根 HWND 直启 OLE（跳过 JS→IPC 往返）。
pub fn start_files_drag_hwnd(hwnd: isize, files: Vec<PathBuf>) -> crate::Result<()> {
    let _ = run_files_ole_drag(HWND(hwnd), &files, None, DragMode::Copy)?;
    Ok(())
}

/// 执行 Files 出站 `DoDragDrop`。若已在 OLE 中则返回 `Ok(None)`（幂等）。
fn run_files_ole_drag(
    our_hwnd: HWND,
    files: &[PathBuf],
    image: Option<Image>,
    mode: DragMode,
) -> crate::Result<Option<DragResult>> {
    if files.is_empty() {
        return Ok(None);
    }
    if OLE_DRAG_IN_PROGRESS.load(Ordering::SeqCst) {
        return Ok(None);
    }
    // 宿主直启与 JS startDrag 可能先后到达；每轮 arm 只允许一次 DoDragDrop。
    if OUTBOUND_CLAIMED.swap(true, Ordering::SeqCst) {
        return Ok(None);
    }

    init_ole();
    unsafe {
        #[allow(static_mut_refs)]
        if let Err(e) = &OLE_RESULT {
            return Err(e.clone().into());
        }
    }

    let our_root = root_hwnd(our_hwnd);
    let preserve_fg = unsafe {
        let fg = GetForegroundWindow();
        resolve_preserve_foreground(our_root.0, fg.0).map(HWND)
    };

    let mut paths = Vec::with_capacity(files.len());
    for f in files {
        paths.push(dunce::canonicalize(f)?);
    }

    let data_object: IDataObject = get_file_data_object(&paths).ok_or(crate::Error::FailedToStartDrag)?;
    // FIL-42: PreferredDropEffect + sync GetData，使播放器 Drop 时拿到与 Explorer 同等路径数据。
    let preferred = preferred_effect_for_mode(mode);
    prepare_shell_file_data_object(&data_object, preferred);
    debug_assert!(
        shell_file_formats_gettable(&data_object),
        "FIL-38: Shell IDataObject must expose GetData-able CF_HDROP + Shell IDList"
    );
    debug_assert!(
        preferred_drop_effect_is(&data_object, preferred.0),
        "FIL-42: Preferred DropEffect must be GetData-able"
    );
    let drop_source: IDropSource = DropSource::new(our_root, preserve_fg).into();

    let drop_result = unsafe {
        if let Some(img) = image {
            if let Some(drag_image) = get_drag_image(img) {
                if let Ok(helper) = create_instance::<IDragSourceHelper>(&CLSID_DragDropHelper) {
                    let _ = helper.InitializeFromBitmap(&drag_image, &data_object);
                }
            }
        }

        let mut out_dropeffect = DROPEFFECT::default();
        // AIA-240: Explorer-parity — offer copy|move|link; target + modifiers choose.
        // FIL-42: PreferredDropEffect（由 options.mode）引导非壳目标走 copy/open，而非空 MOVE。
        let effect = DROPEFFECT_COPY | DROPEFFECT_MOVE | DROPEFFECT_LINK;

        restore_preserved_foreground(our_root, preserve_fg);

        // OLE consumes the gesture — never click-activate on the trailing mouse-up.
        note_drag_gesture();
        clear_outbound_paths();
        OLE_DRAG_IN_PROGRESS.store(true, Ordering::SeqCst);
        let drop_result = DoDragDrop(&data_object, &drop_source, effect, &mut out_dropeffect);
        OLE_DRAG_IN_PROGRESS.store(false, Ordering::SeqCst);
        clear_press_state();

        // Keep drop-target foreground after a background-started OLE drag.
        restore_preserved_foreground(our_root, preserve_fg);
        drop_result
    };

    if drop_result == DRAGDROP_S_DROP {
        Ok(Some(DragResult::Dropped))
    } else {
        Ok(Some(DragResult::Cancel))
    }
}

fn get_drag_image(image: Image) -> Option<SHDRAGIMAGE> {
    let hbitmap = match image {
        Image::Raw(bytes) => image::read_bytes_to_hbitmap(&bytes).ok(),
        Image::File(path) => image::read_path_to_hbitmap(&path).ok(),
    };
    hbitmap.map(|hbitmap| unsafe {
        // get image size
        let mut bitmap: BITMAP = BITMAP::default();
        let (width, height) = if 0
            == GetObjectW(
                hbitmap,
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut bitmap as *mut BITMAP as *mut c_void),
            ) {
            (128, 128)
        } else {
            (bitmap.bmWidth, bitmap.bmHeight)
        };

        SHDRAGIMAGE {
            sizeDragImage: SIZE {
                cx: width,
                cy: height,
            },
            ptOffset: POINT { x: 0, y: 0 },
            hbmpDragImage: hbitmap,
            crColorKey: COLORREF(0x00000000),
        }
    })
}

pub fn create_instance<T: Interface + ComInterface>(clsid: &GUID) -> Result<T> {
    unsafe { CoCreateInstance(clsid, None, CLSCTX_ALL) }
}

/// Shell 文件数据对象：`SHCreateShellItemArrayFromIDLists` → `BHID_DataObject`。
/// 出站 Files 唯一路径（FIL-38）；勿再引入手写 IDataObject。
fn get_file_data_object(paths: &[PathBuf]) -> Option<IDataObject> {
    unsafe {
        let shell_item_array = get_shell_item_array(paths).unwrap();
        shell_item_array.BindToHandler(None, &BHID_DataObject).ok()
    }
}

fn preferred_effect_for_mode(mode: DragMode) -> DROPEFFECT {
    match mode {
        DragMode::Move => DROPEFFECT_MOVE,
        DragMode::Copy => DROPEFFECT_COPY,
    }
}

/// FIL-42：对齐 Explorer 出站给媒体播放器可用的 Drop 合同。
/// - Preferred DropEffect：标明 copy/open 偏好（默认 copy），避免目标只认 MOVE 语义却无法打开路径。
/// - 关闭 `IDataObjectAsyncCapability`：强制 Drop 期间同步 `GetData`，避免 DoDragDrop 返回后源释放对象导致播放器拿到空/失效数据。
fn prepare_shell_file_data_object(obj: &IDataObject, preferred: DROPEFFECT) {
    let _ = set_preferred_drop_effect(obj, preferred.0);
    disable_shell_async_capability(obj);
}

fn preferred_drop_effect_format() -> u16 {
    unsafe { RegisterClipboardFormatW(CFSTR_PREFERREDDROPEFFECT) as u16 }
}

fn set_preferred_drop_effect(obj: &IDataObject, effect: u32) -> Result<()> {
    let cf = preferred_drop_effect_format();
    if cf == 0 {
        return Err(Error::from(E_FAIL));
    }
    unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE, std::mem::size_of::<u32>())?;
        let ptr = GlobalLock(handle);
        if ptr.is_null() {
            let _ = GlobalFree(handle);
            return Err(Error::from(E_OUTOFMEMORY));
        }
        std::ptr::write(ptr as *mut u32, effect);
        let _ = GlobalUnlock(handle);
        let fmt = hglobal_format_etc(cf);
        let medium = STGMEDIUM {
            tymed: TYMED_HGLOBAL.0 as u32,
            u: STGMEDIUM_0 { hGlobal: handle },
            pUnkForRelease: std::mem::ManuallyDrop::new(None),
        };
        // frelease=TRUE：Shell 对象接管 HGLOBAL。
        if let Err(err) = obj.SetData(&fmt, &medium, TRUE) {
            let _ = GlobalFree(handle);
            return Err(err);
        }
    }
    Ok(())
}

fn disable_shell_async_capability(obj: &IDataObject) {
    unsafe {
        if let Ok(async_cap) = obj.cast::<IDataObjectAsyncCapability>() {
            let _ = async_cap.SetAsyncMode(FALSE);
        }
    }
}

fn preferred_drop_effect_is(obj: &IDataObject, expected: u32) -> bool {
    read_preferred_drop_effect(obj) == Some(expected)
}

fn read_preferred_drop_effect(obj: &IDataObject) -> Option<u32> {
    let cf = preferred_drop_effect_format();
    if cf == 0 {
        return None;
    }
    let fmt = hglobal_format_etc(cf);
    unsafe {
        if obj.QueryGetData(&fmt) != S_OK {
            return None;
        }
        match obj.GetData(&fmt) {
            Ok(mut medium) => {
                let mut effect = None;
                if medium.tymed == TYMED_HGLOBAL.0 as u32 {
                    let hglobal = medium.u.hGlobal;
                    let size = GlobalSize(hglobal);
                    let ptr = GlobalLock(hglobal);
                    if !ptr.is_null() && size >= 4 {
                        let bytes = std::slice::from_raw_parts(ptr as *const u8, 4);
                        effect = Some(u32::from_le_bytes([
                            bytes[0], bytes[1], bytes[2], bytes[3],
                        ]));
                    }
                    let _ = GlobalUnlock(hglobal);
                }
                ReleaseStgMedium(&mut medium);
                effect
            }
            Err(_) => None,
        }
    }
}

fn get_shell_item_array(paths: &[PathBuf]) -> Option<IShellItemArray> {
    unsafe {
        let list: Vec<*const Common::ITEMIDLIST> = paths
            .iter()
            .map(|path| get_file_item_id(path).cast_const())
            .collect();
        SHCreateShellItemArrayFromIDLists(&list).ok()
    }
}

fn get_file_item_id(path: &Path) -> *mut Common::ITEMIDLIST {
    unsafe {
        let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(once(0)).collect();
        windows::Win32::UI::Shell::ILCreateFromPathW(PCWSTR::from_raw(wide_path.as_ptr()))
    }
}

fn shell_idlist_clipboard_format() -> u16 {
    unsafe { RegisterClipboardFormatW(CFSTR_SHELLIDLIST) as u16 }
}

fn hglobal_format_etc(cf_format: u16) -> FORMATETC {
    FORMATETC {
        cfFormat: cf_format,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
}

/// FIL-38：确认 Shell IDataObject 对 CF_HDROP 与 Shell IDList 可 QueryGetData/GetData。
fn data_object_format_gettable(obj: &IDataObject, cf_format: u16) -> bool {
    let fmt = hglobal_format_etc(cf_format);
    unsafe {
        if obj.QueryGetData(&fmt) != S_OK {
            return false;
        }
        match obj.GetData(&fmt) {
            Ok(mut medium) => {
                ReleaseStgMedium(&mut medium);
                true
            }
            Err(_) => false,
        }
    }
}

fn shell_file_formats_gettable(obj: &IDataObject) -> bool {
    let idlist = shell_idlist_clipboard_format();
    if idlist == 0 {
        return false;
    }
    data_object_format_gettable(obj, CF_HDROP.0)
        && data_object_format_gettable(obj, idlist)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Mutex;

    static LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn press_prior_drag_gesture_and_resolve() {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_press_state();
        // Background press remembered Explorer (0x100); by OLE start we are FG (0x200).
        note_deferred_activate(0x100, 10, 20);
        assert_eq!(press_prior_foreground(), 0x100);
        assert_eq!(press_origin(), (10, 20));
        assert_eq!(resolve_preserve_foreground(0x200, 0x200), Some(0x100));

        assert!(PENDING_BACKGROUND_ACTIVATE.load(Ordering::SeqCst));
        note_drag_gesture();
        assert!(!take_pending_activate());
        assert!(drag_gesture_active());
        assert!(should_suppress_foreground_raise());
        clear_press_state();
        assert!(!should_suppress_foreground_raise());

        assert_eq!(resolve_preserve_foreground(0x200, 0x100), Some(0x100));
        assert_eq!(resolve_preserve_foreground(0x200, 0x200), None);
        assert_eq!(resolve_preserve_foreground(0x200, 0), None);
    }

    #[test]
    fn note_press_origin_does_not_defer_activate() {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_press_state();
        note_press_origin(3, 4);
        assert_eq!(press_origin(), (3, 4));
        assert_eq!(press_prior_foreground(), 0);
        assert!(!PENDING_BACKGROUND_ACTIVATE.load(Ordering::SeqCst));
        clear_press_state();
    }

    #[test]
    fn outbound_paths_arm_take_and_clear() {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_outbound_paths();
        arm_outbound_paths(vec![
            PathBuf::from(r"C:\media\a.mp4"),
            PathBuf::from(""),
            PathBuf::from(r"C:\docs\b.txt"),
        ]);
        assert!(!OUTBOUND_CLAIMED.load(Ordering::SeqCst));
        let taken = take_outbound_paths();
        assert_eq!(taken.len(), 2);
        assert!(take_outbound_paths().is_empty());
        arm_outbound_paths(vec![PathBuf::from(r"C:\x")]);
        clear_outbound_paths();
        assert!(take_outbound_paths().is_empty());
        assert!(
            !OUTBOUND_CLAIMED.swap(true, Ordering::SeqCst),
            "fresh arm must allow one OLE claim"
        );
        assert!(
            OUTBOUND_CLAIMED.swap(true, Ordering::SeqCst),
            "second claim in same arm must be rejected"
        );
        arm_outbound_paths(vec![PathBuf::from(r"C:\y")]);
        assert!(!OUTBOUND_CLAIMED.load(Ordering::SeqCst));
        clear_outbound_paths();
    }

    #[test]
    fn shell_data_object_exposes_hdrop_and_shell_idlist() {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        init_ole();
        unsafe {
            #[allow(static_mut_refs)]
            if let Err(e) = &OLE_RESULT {
                panic!("OleInitialize failed: {e}");
            }
        }

        let dir = std::env::temp_dir().join("afm-fil38-dnd-audit");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("sample.txt");
        {
            let mut f = std::fs::File::create(&file).expect("create temp file");
            f.write_all(b"fil-38").expect("write temp file");
        }
        let path = dunce::canonicalize(&file).expect("canonicalize");
        let obj = get_file_data_object(&[path]).expect("Shell IDataObject");
        assert!(
            shell_file_formats_gettable(&obj),
            "CF_HDROP + CFSTR_SHELLIDLIST must be QueryGetData/GetData-able"
        );
        // FIL-42：同一对象上准备 PreferredDropEffect + 同步 GetData。
        prepare_shell_file_data_object(&obj, DROPEFFECT_COPY);
        assert!(
            shell_file_formats_gettable(&obj),
            "prepare must keep CF_HDROP + Shell IDList GetData-able"
        );
        assert_eq!(
            read_preferred_drop_effect(&obj),
            Some(DROPEFFECT_COPY.0),
            "Preferred DropEffect must be COPY for open/play targets"
        );
        unsafe {
            if let Ok(async_cap) = obj.cast::<IDataObjectAsyncCapability>() {
                let mode = async_cap.GetAsyncMode().expect("GetAsyncMode");
                assert!(
                    !mode.as_bool(),
                    "FIL-42: async capability must be disabled so Drop GetData stays sync"
                );
            }
        }
        let _ = std::fs::remove_file(&file);
        let _ = std::fs::remove_dir(&dir);
    }
}
