// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod platform;
#[cfg(target_os = "linux")]
#[path = "gtk/mod.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod platform;
pub use platform::start_drag;

#[cfg(windows)]
pub use platform::{
    arm_outbound_paths, clear_outbound_paths, clear_press_state, drag_gesture_active,
    had_deferred_activate, note_deferred_activate, note_drag_gesture, note_press_origin,
    ole_drag_in_progress, press_origin, press_prior_foreground, resolve_preserve_foreground,
    restore_press_prior_foreground, should_suppress_foreground_raise, start_files_drag_hwnd,
    take_outbound_paths, take_pending_activate,
};
