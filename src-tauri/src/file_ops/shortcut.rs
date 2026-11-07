// SPDX-License-Identifier: GPL-3.0-or-later
//! Create Windows `.lnk` shortcuts (AIA-240 Link 投放).

use super::{
    display_path, ensure_dir, ensure_existing, err_msg, ok_paths, parse_name_conflict,
    unique_dest_path, FileOpsResult, NameConflict,
};
use std::path::Path;

fn shortcut_file_name(source_name: &str) -> String {
    format!("{source_name} - Shortcut.lnk")
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
        let _ = (source, dest_lnk);
        Err("Shortcuts are only supported on Windows".into())
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
    use super::shortcut_file_name;

    #[test]
    fn shortcut_name_appends_suffix() {
        assert_eq!(shortcut_file_name("Report.pdf"), "Report.pdf - Shortcut.lnk");
        assert_eq!(shortcut_file_name("Photos"), "Photos - Shortcut.lnk");
    }
}
