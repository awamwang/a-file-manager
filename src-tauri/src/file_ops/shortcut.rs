// SPDX-License-Identifier: GPL-3.0-or-later
//! Copy-as-link: Windows `.lnk` shortcuts (AIA-240 / TR-203); on Unix a plain
//! symbolic link so the context entry is not a guaranteed failure on macOS.

use super::{
    display_path, ensure_dir, ensure_existing, err_msg, ok_paths, parse_name_conflict,
    unique_dest_path, FileOpsResult, NameConflict,
};
use std::path::Path;

/// TR-203: the link's file name. Windows creates `.lnk` shortcuts; Unix creates
/// a symlink with a `.link` suffix so it stays distinguishable from its source
/// (same-directory paste must not collide) without pretending to be a `.lnk`.
#[cfg(windows)]
fn shortcut_file_name(source_name: &str) -> String {
    format!("{source_name} - Shortcut.lnk")
}

#[cfg(not(windows))]
fn shortcut_file_name(source_name: &str) -> String {
    format!("{source_name} - Shortcut.link")
}

fn create_one_shortcut(source: &Path, dest_lnk: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::core::{Interface, PCWSTR};
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED,
        };
        use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

        unsafe {
            let coinit = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let needs_uninit = coinit.is_ok();
            let result = (|| -> windows::core::Result<()> {
                let shell_link: IShellLinkW =
                    CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
                let source_wide: Vec<u16> = source
                    .as_os_str()
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                shell_link.SetPath(PCWSTR(source_wide.as_ptr()))?;
                if let Some(parent) = source.parent() {
                    let parent_wide: Vec<u16> = parent
                        .as_os_str()
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect();
                    shell_link.SetWorkingDirectory(PCWSTR(parent_wide.as_ptr()))?;
                }
                let persist: IPersistFile = shell_link.cast()?;
                let dest_wide: Vec<u16> = dest_lnk
                    .as_os_str()
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                persist.Save(PCWSTR(dest_wide.as_ptr()), true)?;
                Ok(())
            })();
            if needs_uninit {
                CoUninitialize();
            }
            result.map_err(|e| e.to_string())
        }
    }
    #[cfg(not(windows))]
    {
        // TR-203 macOS 降级：符号链接指向源文件/abs 路径，与项目成员/skill 的
        // Unix 链接约定一致（projects/members.rs、ai/deploy.rs）。canonicalize
        // 保证移动链接文件时目标仍有效。
        let absolute = std::fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
        std::os::unix::fs::symlink(&absolute, dest_lnk)
            .map_err(|err| format!("Failed to create link: {err}"))
    }
}

/// Read the stored target path of an existing `.lnk` shortcut (link preview).
/// Returns the raw stored target (empty string when the shortcut has no file target).
pub fn read_shortcut_target(lnk_path: &Path) -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::core::{Interface, PCWSTR};
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED, STGM_READ,
        };
        use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

        unsafe {
            let coinit = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let needs_uninit = coinit.is_ok();
            let result = (|| -> windows::core::Result<String> {
                let shell_link: IShellLinkW =
                    CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
                let persist: IPersistFile = shell_link.cast()?;
                let lnk_wide: Vec<u16> = lnk_path
                    .as_os_str()
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                persist.Load(PCWSTR(lnk_wide.as_ptr()), STGM_READ)?;
                let mut target_buf = [0u16; 1024];
                shell_link.GetPath(&mut target_buf, std::ptr::null_mut(), 0)?;
                let len = target_buf
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(target_buf.len());
                Ok(String::from_utf16_lossy(&target_buf[..len]))
            })();
            if needs_uninit {
                CoUninitialize();
            }
            result.map_err(|e| e.to_string())
        }
    }
    #[cfg(not(windows))]
    {
        // Unix symlink: read_link gives the same "where does this point" answer
        // that IShellLink::GetPath gives for a Windows .lnk.
        std::fs::read_link(lnk_path)
            .map(|target| target.to_string_lossy().into_owned())
            .map_err(|err| format!("Failed to read link: {err}"))
    }
}

pub fn create_shortcuts(
    sources: &[String],
    dest_dir: &str,
    conflict: NameConflict,
) -> Result<Vec<String>, String> {
    let dest = ensure_dir(dest_dir)?;
    let mut created = Vec::new();
    for source in sources {
        let src = ensure_existing(source)?;
        let base = src
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("Invalid source name: {source}"))?;
        let link_name = shortcut_file_name(base);
        let mut dest_path = dest.join(&link_name);
        if dest_path.exists() {
            match conflict {
                NameConflict::Replace => {
                    let _ = std::fs::remove_file(&dest_path);
                }
                NameConflict::Skip => continue,
                NameConflict::KeepBoth => {
                    dest_path = unique_dest_path(&dest, &link_name);
                }
            }
        }
        create_one_shortcut(&src, &dest_path)?;
        created.push(display_path(&dest_path));
    }
    Ok(created)
}

#[tauri::command]
pub fn file_ops_create_shortcuts(
    sources: Vec<String>,
    dest_dir: String,
    conflict: Option<String>,
) -> FileOpsResult {
    match create_shortcuts(
        &sources,
        &dest_dir,
        parse_name_conflict(conflict.as_deref()),
    ) {
        Ok(paths) => ok_paths(paths),
        Err(error) => err_msg(error),
    }
}

#[cfg(test)]
mod tests {
    use super::{create_shortcuts, read_shortcut_target, shortcut_file_name};
    use crate::file_ops::NameConflict;
    use std::fs;

    #[cfg(windows)]
    #[test]
    fn shortcut_name_appends_suffix() {
        assert_eq!(shortcut_file_name("Report.pdf"), "Report.pdf - Shortcut.lnk");
        assert_eq!(shortcut_file_name("Photos"), "Photos - Shortcut.lnk");
    }

    #[cfg(not(windows))]
    #[test]
    fn shortcut_name_appends_link_suffix() {
        assert_eq!(shortcut_file_name("Report.pdf"), "Report.pdf - Shortcut.link");
        assert_eq!(shortcut_file_name("Photos"), "Photos - Shortcut.link");
    }

    /// TR-203 macOS 降级：复制为链接必须产出真正可读的符号链接，且 link
    /// preview 能读出目标 —— 不是必然失败的 .lnk 存根。
    #[cfg(not(windows))]
    #[test]
    fn unix_copy_as_link_creates_readable_symlink() {
        let root = std::env::temp_dir().join(format!(
            "afm-tr203-link-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let src_dir = root.join("src");
        let dest_dir = root.join("dest");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dest_dir).unwrap();
        let source = src_dir.join("Report.pdf");
        fs::write(&source, b"payload").unwrap();

        let created = create_shortcuts(
            &[source.to_string_lossy().into_owned()],
            &dest_dir.to_string_lossy(),
            NameConflict::KeepBoth,
        )
        .unwrap();
        assert_eq!(created.len(), 1);
        let link = dest_dir.join("Report.pdf - Shortcut.link");
        assert!(link.exists(), "symlink must be created");

        let meta = fs::symlink_metadata(&link).unwrap();
        assert!(meta.file_type().is_symlink(), "entry must be a symlink");
        // 链接内容可读 —— 目标文件未因创建链接被改动。
        assert_eq!(fs::read(&link).unwrap(), b"payload");
        let target = read_shortcut_target(&link).unwrap();
        assert!(
            target.ends_with("Report.pdf"),
            "link preview must expose target, got {target}"
        );
        // 源文件必须保持原样（链接不是移动）。
        assert_eq!(fs::read(&source).unwrap(), b"payload");

        let _ = fs::remove_dir_all(&root);
    }
}
