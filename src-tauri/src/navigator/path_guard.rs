// SPDX-License-Identifier: GPL-3.0-or-later

/// Kernel Navigator accepts classic OS / UNC paths only (ADR 0001).
/// `awam://` and other virtual URI schemes are extension-layer, not Kernel browse.
pub fn assert_os_path(path: &str) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Path must not be empty".to_string());
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("awam://") {
        return Err(
            "navigator_list rejects awam:// paths; Kernel browse uses native OS paths only"
                .to_string(),
        );
    }

    if looks_like_uri_scheme(&lower) {
        return Err(format!(
            "navigator_list rejects URI scheme paths for Kernel browse: {trimmed}"
        ));
    }

    Ok(())
}

fn looks_like_uri_scheme(lower: &str) -> bool {
    if let Some(idx) = lower.find("://") {
        let before = &lower[..idx];
        return !is_windows_drive_letter(before);
    }
    false
}

fn is_windows_drive_letter(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 1 && bytes[0].is_ascii_alphabetic()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_windows_drive_and_unc() {
        assert!(assert_os_path(r"G:\Projects").is_ok());
        assert!(assert_os_path("G:/Projects").is_ok());
        assert!(assert_os_path(r"\\server\share").is_ok());
    }

    #[test]
    fn rejects_awam_and_empty() {
        assert!(assert_os_path("awam://local/foo").is_err());
        assert!(assert_os_path("AWAM://knowledge/x").is_err());
        assert!(assert_os_path("").is_err());
        assert!(assert_os_path("   ").is_err());
    }

    #[test]
    fn rejects_other_uri_schemes() {
        assert!(assert_os_path("file:///C:/temp").is_err());
        assert!(assert_os_path("http://example.com").is_err());
    }
}
