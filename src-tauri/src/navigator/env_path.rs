// SPDX-License-Identifier: GPL-3.0-or-later
//! Expand %VAR% in browse paths (address bar).

#[cfg(windows)]
pub fn expand_env_path(input: &str) -> String {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows::core::PCWSTR;
    use windows::Win32::System::Environment::ExpandEnvironmentStringsW;

    let wide: Vec<u16> = std::ffi::OsStr::new(input)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let needed = ExpandEnvironmentStringsW(PCWSTR(wide.as_ptr()), None);
        if needed == 0 {
            return input.to_string();
        }
        let mut buf = vec![0u16; needed as usize];
        let written = ExpandEnvironmentStringsW(PCWSTR(wide.as_ptr()), Some(&mut buf));
        if written == 0 {
            return input.to_string();
        }
        // written includes trailing NUL
        let len = (written as usize).saturating_sub(1);
        std::ffi::OsString::from_wide(&buf[..len])
            .to_string_lossy()
            .into_owned()
    }
}

#[cfg(not(windows))]
pub fn expand_env_path(input: &str) -> String {
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn expands_userprofile() {
        let expanded = expand_env_path(r"%USERPROFILE%\Documents");
        assert!(!expanded.contains('%'), "{expanded}");
        assert!(expanded.to_lowercase().contains("documents"));
    }

    #[test]
    fn leaves_plain_paths() {
        assert_eq!(expand_env_path(r"C:\Temp"), r"C:\Temp");
    }
}
