// SPDX-License-Identifier: GPL-3.0-or-later
//! Win11 Snap Layouts for the custom Fluent titlebar.
//!
//! Approach (documented): `tauri-plugin-frame` snap overlay — a transparent native
//! child HWND over the maximize button that answers `WM_NCHITTEST` with `HTMAXBUTTON`.
//! WebView2 otherwise swallows hit-testing, so subclassing the parent alone cannot
//! surface Snap Layouts. See Microsoft Learn “Support snap layouts for desktop apps
//! on Windows 11” and tauri-plugin-frame / Zbrooklyn frameless Snap notes.
//!
//! Geometry must match CSS: `--fluent-titlebar-height` and `.win-btn` width.
//!
//! AIA-80 caption ownership:
//! - Fluent Vue draws minimize / maximize / close (`frame-tb-*`) and handles clicks
//!   via `@tauri-apps/api` (`core:window:allow-*` capabilities).
//! - Plugin `auto_titlebar(false)` still evals titlebar.js/controls.js on
//!   `frame-page-load`. A boot sentinel in `index.html` (`#afm-frame-boot`) prevents
//!   titlebar.js from injecting a fixed full-width drag overlay that stole min/close
//!   hits before Vue mounted. `withGlobalTauri` lets controls.js exit cleanly once
//!   it sees existing `frame-tb-*` stubs.

/// Logical titlebar height — keep in sync with `--fluent-titlebar-height` (40px).
pub const TITLEBAR_HEIGHT_PX: u32 = 40;

/// Logical caption button width — keep in sync with `.win-btn` (46px).
pub const BUTTON_WIDTH_PX: u32 = 46;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_geometry_matches_fluent_chrome_contract() {
        assert_eq!(TITLEBAR_HEIGHT_PX, 40);
        assert_eq!(BUTTON_WIDTH_PX, 46);
    }
}
