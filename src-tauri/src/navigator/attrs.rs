// SPDX-License-Identifier: GPL-3.0-or-later
//! Read / set Windows file attributes (readonly + hidden).

use super::path_guard::assert_os_path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EntryAttributes {
    pub readonly: bool,
    pub hidden: bool,
}

#[cfg(windows)]
const FILE_ATTRIBUTE_READONLY: u32 = 0x1;
#[cfg(windows)]
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;

#[cfg(windows)]
pub fn read_attributes(path: &str) -> Result<EntryAttributes, String> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetFileAttributesW;

    assert_os_path(path)?;
    let wide: Vec<u16> = path
        .trim()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let attrs = GetFileAttributesW(PCWSTR(wide.as_ptr()));
        if attrs == u32::MAX {
            return Err(format!("GetFileAttributesW failed for {path}"));
        }
        Ok(EntryAttributes {
            readonly: attrs & FILE_ATTRIBUTE_READONLY != 0,
            hidden: attrs & FILE_ATTRIBUTE_HIDDEN != 0,
        })
    }
}

#[cfg(windows)]
pub fn set_attributes(path: &str, next: EntryAttributes) -> Result<EntryAttributes, String> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        GetFileAttributesW, SetFileAttributesW, FILE_FLAGS_AND_ATTRIBUTES,
    };

    assert_os_path(path)?;
    let wide: Vec<u16> = path
        .trim()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let current = GetFileAttributesW(PCWSTR(wide.as_ptr()));
        if current == u32::MAX {
            return Err(format!("GetFileAttributesW failed for {path}"));
        }
        let attrs = merge_attribute_bits(current, &next);
        SetFileAttributesW(PCWSTR(wide.as_ptr()), FILE_FLAGS_AND_ATTRIBUTES(attrs))
            .map_err(|err| format!("SetFileAttributesW failed: {err}"))?;
    }
    read_attributes(path)
}

#[cfg(not(windows))]
pub fn read_attributes(path: &str) -> Result<EntryAttributes, String> {
    use std::fs;
    assert_os_path(path)?;
    let meta = fs::metadata(path.trim()).map_err(|err| format!("metadata failed: {err}"))?;
    Ok(EntryAttributes {
        readonly: meta.permissions().readonly(),
        hidden: false,
    })
}

#[cfg(not(windows))]
pub fn set_attributes(path: &str, next: EntryAttributes) -> Result<EntryAttributes, String> {
    use std::fs;
    assert_os_path(path)?;
    let meta = fs::metadata(path.trim()).map_err(|err| format!("metadata failed: {err}"))?;
    let mut perms = meta.permissions();
    perms.set_readonly(next.readonly);
    fs::set_permissions(path.trim(), perms).map_err(|err| format!("set_permissions failed: {err}"))?;
    let _ = next.hidden;
    read_attributes(path)
}

/// Pure helper for tests: merge attribute flags without touching the filesystem.
pub fn merge_attribute_bits(current: u32, next: &EntryAttributes) -> u32 {
    let mut attrs = current;
    const READONLY: u32 = 0x1;
    const HIDDEN: u32 = 0x2;
    if next.readonly {
        attrs |= READONLY;
    } else {
        attrs &= !READONLY;
    }
    if next.hidden {
        attrs |= HIDDEN;
    } else {
        attrs &= !HIDDEN;
    }
    attrs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_toggles_readonly_and_hidden() {
        let next = EntryAttributes {
            readonly: true,
            hidden: true,
        };
        assert_eq!(merge_attribute_bits(0, &next), 0x3);
        assert_eq!(
            merge_attribute_bits(
                0x3,
                &EntryAttributes {
                    readonly: false,
                    hidden: true,
                }
            ),
            0x2
        );
        assert_eq!(
            merge_attribute_bits(
                0x23,
                &EntryAttributes {
                    readonly: false,
                    hidden: false,
                }
            ),
            0x20
        );
    }

    #[cfg(windows)]
    #[test]
    fn set_attributes_roundtrip_on_temp_file() {
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let path = std::env::temp_dir().join(format!("afm-attrs-{stamp}.txt"));
        fs::write(&path, b"attrs").unwrap();
        let path_str = path.to_string_lossy().to_string();
        let applied = set_attributes(
            &path_str,
            EntryAttributes {
                readonly: true,
                hidden: true,
            },
        )
        .unwrap();
        assert!(applied.readonly);
        assert!(applied.hidden);
        let cleared = set_attributes(
            &path_str,
            EntryAttributes {
                readonly: false,
                hidden: false,
            },
        )
        .unwrap();
        assert!(!cleared.readonly);
        assert!(!cleared.hidden);
        let _ = fs::remove_file(&path);
    }
}
