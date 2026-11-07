// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::{CursorPosition, DragItem, DragResult, Image, Options};

use std::{
    ffi::c_void,
    iter::once,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering},
    sync::Once,
};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::{GetObjectW, BITMAP},
        System::Com::*,
        System::Memory::*,
        System::Ole::{DoDragDrop, OleInitialize},
        System::Ole::{
            IDropSource, IDropSource_Impl, CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK,
            DROPEFFECT_MOVE,
        },
        System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS},
        UI::{
            Shell::{
                BHID_DataObject, CLSID_DragDropHelper, Common, IDragSourceHelper, IShellItemArray,
                SHCreateDataObject, SHCreateShellItemArrayFromIDLists, DROPFILES, SHDRAGIMAGE,
            },
            WindowsAndMessaging::{
                GetAncestor, GetCursorPos, GetForegroundWindow, IsChild, SetForegroundWindow,
                GA_ROOT,
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

#[implement(IDataObject)]
struct DataObject {
    files: Vec<PathBuf>,
    inner_shell_obj: IDataObject,
}

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

/// AIA-261: if OLE / WebView re-activates us mid-drag, put the prior foreground back
/// so the drop target under the cursor stays usable (Explorer parity).
fn restore_preserved_foreground(our_root: HWND, preserve_fg: Option<HWND>) {
    let Some(prev) = preserve_fg else {
        return;
    };
    if prev.0 == 0 {
        return;
    }
    unsafe {
        let fg = GetForegroundWindow();
        if hwnd_is_ours(our_root, fg) {
            let _ = SetForegroundWindow(prev);
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

impl DataObject {
    // This will be used for sharing text between applications
    #[allow(dead_code)]
    fn new(files: Vec<PathBuf>) -> Self {
        unsafe {
            Self {
                files,
                inner_shell_obj: SHCreateDataObject(None, None, None).unwrap(),
            }
        }
    }

    fn is_supported_format(pformatetc: *const FORMATETC) -> bool {
        if let Some(format_etc) = unsafe { pformatetc.as_ref() } {
            !(format_etc.tymed as i32 != TYMED_HGLOBAL.0
                || format_etc.cfFormat != CF_HDROP.0
                || format_etc.dwAspect != DVASPECT_CONTENT.0)
        } else {
            false
        }
    }

    fn clone_drop_hglobal(&self) -> Result<HGLOBAL> {
        let mut buffer = Vec::new();
        for path in &self.files {
            let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(once(0)).collect();
            buffer.extend(wide_path);
        }
        buffer.push(0);
        let size = std::mem::size_of::<DROPFILES>() + buffer.len() * 2;
        let handle = get_hglobal(size, buffer)?;
        Ok(handle)
    }
}

#[allow(non_snake_case)]
impl IDataObject_Impl for DataObject {
    fn GetData(&self, pformatetc: *const FORMATETC) -> Result<STGMEDIUM> {
        unsafe {
            if Self::is_supported_format(pformatetc) {
                Ok(STGMEDIUM {
                    tymed: TYMED_HGLOBAL.0 as u32,
                    u: STGMEDIUM_0 {
                        hGlobal: self.clone_drop_hglobal()?,
                    },
                    pUnkForRelease: std::mem::ManuallyDrop::new(None),
                })
            } else {
                self.inner_shell_obj.GetData(pformatetc)
            }
        }
    }

    fn GetDataHere(&self, _pformatetc: *const FORMATETC, _pmedium: *mut STGMEDIUM) -> Result<()> {
        Err(Error::new(DV_E_FORMATETC, HSTRING::new()))
    }

    fn QueryGetData(&self, pformatetc: *const FORMATETC) -> HRESULT {
        unsafe {
            if Self::is_supported_format(pformatetc) {
                S_OK
            } else {
                self.inner_shell_obj.QueryGetData(pformatetc)
            }
        }
    }

    fn GetCanonicalFormatEtc(
        &self,
        _pformatectin: *const FORMATETC,
        pformatetcout: *mut FORMATETC,
    ) -> HRESULT {
        unsafe { (*pformatetcout).ptd = std::ptr::null_mut() };
        E_NOTIMPL
    }

    fn SetData(
        &self,
        pformatetc: *const FORMATETC,
        pmedium: *const STGMEDIUM,
        frelease: BOOL,
    ) -> Result<()> {
        unsafe { self.inner_shell_obj.SetData(pformatetc, pmedium, frelease) }
    }

    fn EnumFormatEtc(&self, _dwdirection: u32) -> Result<IEnumFORMATETC> {
        Err(Error::new(E_NOTIMPL, HSTRING::new()))
    }

    fn DAdvise(
        &self,
        _pformatetc: *const FORMATETC,
        _advf: u32,
        _padvsink: Option<&IAdviseSink>,
    ) -> Result<u32> {
        Err(Error::new(OLE_E_ADVISENOTSUPPORTED, HSTRING::new()))
    }

    fn DUnadvise(&self, _dwconnection: u32) -> Result<()> {
        Err(Error::new(OLE_E_ADVISENOTSUPPORTED, HSTRING::new()))
    }

    fn EnumDAdvise(&self) -> Result<IEnumSTATDATA> {
        Err(Error::new(OLE_E_ADVISENOTSUPPORTED, HSTRING::new()))
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
        let our_hwnd = HWND(w.hwnd.get());
        let our_root = root_hwnd(our_hwnd);
        // AIA-261/264: prefer press-time prior FG (mousedown may have already raised AFM),
        // else the FG at OLE start when Hybrid left/blur moved focus to the drop target.
        let preserve_fg = unsafe {
            let fg = GetForegroundWindow();
            resolve_preserve_foreground(our_root.0, fg.0).map(HWND)
        };

        match item {
            DragItem::Files(files) => {
                init_ole();
                unsafe {
                    #[allow(static_mut_refs)]
                    if let Err(e) = &OLE_RESULT {
                        return Err(e.clone().into());
                    }
                }

                let mut paths = Vec::new();
                for f in files {
                    paths.push(dunce::canonicalize(f)?);
                }

                let data_object: IDataObject = get_file_data_object(&paths).unwrap();
                let drop_source: IDropSource = DropSource::new(our_root, preserve_fg).into();

                unsafe {
                    if let Some(drag_image) = get_drag_image(image) {
                        if let Ok(helper) =
                            create_instance::<IDragSourceHelper>(&CLSID_DragDropHelper)
                        {
                            let _ = helper.InitializeFromBitmap(&drag_image, &data_object);
                        }
                    }

                    let mut out_dropeffect = DROPEFFECT::default();
                    // AIA-240: Explorer-parity — offer copy|move|link; target + modifiers choose.
                    // `options.mode` kept for API compat but mask always includes all three.
                    let _ = options.mode;
                    let effect = DROPEFFECT_COPY | DROPEFFECT_MOVE | DROPEFFECT_LINK;

                    if let Some(prev) = preserve_fg {
                        let _ = SetForegroundWindow(prev);
                    }

                    // OLE consumes the gesture — never click-activate on the trailing mouse-up.
                    note_drag_gesture();
                    OLE_DRAG_IN_PROGRESS.store(true, Ordering::SeqCst);
                    let drop_result =
                        DoDragDrop(&data_object, &drop_source, effect, &mut out_dropeffect);
                    OLE_DRAG_IN_PROGRESS.store(false, Ordering::SeqCst);
                    clear_press_state();

                    // Keep drop-target foreground after a background-started OLE drag.
                    if let Some(prev) = preserve_fg {
                        let _ = SetForegroundWindow(prev);
                    }

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

fn get_hglobal(size: usize, buffer: Vec<u16>) -> Result<HGLOBAL> {
    let handle = unsafe { GlobalAlloc(GMEM_FIXED, size).unwrap() };
    let ptr = unsafe { GlobalLock(handle) };

    let header = ptr as *mut DROPFILES;
    unsafe {
        (*header).pFiles = std::mem::size_of::<DROPFILES>() as u32;
        (*header).fWide = BOOL(1);
        std::ptr::copy(
            buffer.as_ptr() as *const c_void,
            ptr.add(std::mem::size_of::<DROPFILES>()),
            buffer.len() * 2,
        );
        GlobalUnlock(handle)
    }?;
    Ok(handle)
}

pub fn create_instance<T: Interface + ComInterface>(clsid: &GUID) -> Result<T> {
    unsafe { CoCreateInstance(clsid, None, CLSCTX_ALL) }
}

fn get_file_data_object(paths: &[PathBuf]) -> Option<IDataObject> {
    unsafe {
        let shell_item_array = get_shell_item_array(paths).unwrap();
        shell_item_array.BindToHandler(None, &BHID_DataObject).ok()
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

#[cfg(test)]
mod tests {
    use super::*;
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
}
