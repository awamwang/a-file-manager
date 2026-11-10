// SPDX-License-Identifier: GPL-3.0-or-later

use super::path_guard::assert_os_path;
use serde::Serialize;
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NavigatorEntry {
    pub name: String,
    pub path: String,
    /// `"dir"` or `"file"` (symlinks reported by target kind when possible).
    pub kind: String,
    pub size: u64,
    /// Last write time as Unix epoch milliseconds (when the FS reports it).
    pub modified_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NavigatorListing {
    pub path: String,
    pub entries: Vec<NavigatorEntry>,
    pub dir_count: u64,
    pub file_count: u64,
}

pub(crate) fn entry_is_hidden(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 2;
        metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}

pub fn list_directory(path: &str, show_hidden: bool) -> Result<NavigatorListing, String> {
    assert_os_path(path)?;

    let requested = PathBuf::from(path.trim());
    // C4: if the path itself is a Workspace member junction (or any dir reparse),
    // navigate/list under the real target so tabs never stay on the junction path.
    let root = crate::projects::members::resolve_reparse_to_target(&requested)
        .unwrap_or(requested.clone());
    if !root.exists() {
        return Err(format!("Path does not exist: {path}"));
    }
    if !root.is_dir() {
        return Err(format!("Path is not a directory: {path}"));
    }

    let read_dir = fs::read_dir(&root).map_err(|err| format!("Failed to read directory: {err}"))?;

    let mut entries: Vec<NavigatorEntry> = Vec::new();
    for item in read_dir {
        let entry = match item {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let entry_path = entry.path();
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        if !show_hidden && entry_is_hidden(&metadata) {
            continue;
        }

        let is_dir = metadata.is_dir();
        let kind = if is_dir { "dir" } else { "file" }.to_string();
        let size = if metadata.is_file() { metadata.len() } else { 0 };
        let name = entry.file_name().to_string_lossy().to_string();
        let modified_ms = metadata.modified().ok().and_then(system_time_ms);

        // Directory junctions/symlinks: expose the real target as `path` so
        // Navigator tabs open the original folder (AIA-210 / C4).
        let open_path = if is_dir {
            crate::projects::members::resolve_reparse_to_target(&entry_path)
                .unwrap_or(entry_path.clone())
        } else {
            entry_path.clone()
        };

        entries.push(NavigatorEntry {
            name,
            path: display_path(&open_path),
            kind,
            size,
            modified_ms,
        });
    }

    entries.sort_by(compare_entries);

    let dir_count = entries.iter().filter(|e| e.kind == "dir").count() as u64;
    let file_count = entries.len() as u64 - dir_count;

    Ok(NavigatorListing {
        path: display_path(&root),
        entries,
        dir_count,
        file_count,
    })
}

pub fn default_path() -> Result<String, String> {
    if let Ok(home) = std::env::var("USERPROFILE") {
        if !home.trim().is_empty() && Path::new(&home).is_dir() {
            return Ok(display_path(Path::new(&home)));
        }
    }
    if Path::new(r"C:\").is_dir() {
        return Ok(r"C:\".to_string());
    }
    Err("Could not resolve a default browse path".to_string())
}

pub fn parent_path(path: &str) -> Result<Option<String>, String> {
    assert_os_path(path)?;
    let current = PathBuf::from(path.trim());
    match current.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => Ok(Some(display_path(parent))),
        _ => Ok(None),
    }
}

pub(crate) fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}

pub(crate) fn system_time_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}

fn compare_entries(left: &NavigatorEntry, right: &NavigatorEntry) -> Ordering {
    match (left.kind.as_str(), right.kind.as_str()) {
        ("dir", "file") => Ordering::Less,
        ("file", "dir") => Ordering::Greater,
        _ => left.name.to_ascii_lowercase().cmp(&right.name.to_ascii_lowercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    /// Manual perf probe: `AFM_PERF_DIR=<large-dir> cargo test --lib navigator::list::tests::bench_list_large_directory -- --nocapture`
    #[test]
    fn bench_list_large_directory() {
        let Ok(path) = std::env::var("AFM_PERF_DIR") else {
            return;
        };
        let _ = list_directory(&path, true).unwrap();
        let mut times = Vec::new();
        for _ in 0..5 {
            let t0 = std::time::Instant::now();
            let listing = list_directory(&path, true).unwrap();
            times.push(t0.elapsed().as_secs_f64() * 1000.0);
            assert!(listing.entries.len() >= 1000, "unexpected small dir");
        }
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let listing = list_directory(&path, true).unwrap();
        let n = listing.entries.len();
        let mut serde_times = Vec::new();
        for _ in 0..5 {
            let t0 = std::time::Instant::now();
            let payload = serde_json::to_string(&listing).expect("serialize listing");
            serde_times.push(t0.elapsed().as_secs_f64() * 1000.0);
            assert!(!payload.is_empty());
        }
        serde_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        eprintln!(
            "AFM_PERF list_directory n={n} samples_ms={times:?} median_ms={:.1}",
            times[times.len() / 2]
        );
        eprintln!(
            "AFM_PERF list_serde_json n={n} samples_ms={serde_times:?} median_ms={:.1}",
            serde_times[serde_times.len() / 2]
        );
    }

    #[test]
    fn lists_temp_directory_entries() {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-nav-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::create_dir(dir.join("alpha-dir")).unwrap();
        let mut file = File::create(dir.join("beta.txt")).unwrap();
        write!(file, "hi").unwrap();

        let listing = list_directory(&dir.to_string_lossy(), true).unwrap();
        assert_eq!(listing.dir_count, 1);
        assert_eq!(listing.file_count, 1);
        assert_eq!(listing.entries[0].kind, "dir");
        assert_eq!(listing.entries[0].name, "alpha-dir");
        assert_eq!(listing.entries[1].name, "beta.txt");
        assert_eq!(listing.entries[1].kind, "file");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_awam_in_list() {
        let err = list_directory("awam://local/C:/", false).unwrap_err();
        assert!(err.contains("awam://"));
    }

    #[test]
    fn default_path_is_os_directory() {
        let path = default_path().unwrap();
        assert!(Path::new(&path).is_dir());
        assert!(!path.to_ascii_lowercase().starts_with("awam://"));
    }

    #[test]
    fn parent_of_nested_path() {
        let parent = parent_path(r"C:\Windows\System32").unwrap();
        assert_eq!(parent.as_deref(), Some(r"C:\Windows"));
    }

    #[test]
    fn parent_of_drive_roots_is_none() {
        for p in [r"D:\", r"D:/", r"D:", r"C:\", r"G:\", r"\\server\share"] {
            let got = parent_path(p).unwrap();
            assert!(got.is_none(), "{p} should have no FS parent, got {got:?}");
        }
        assert_eq!(parent_path(r"D:\Projects").unwrap().as_deref(), Some(r"D:\"));
    }

    #[cfg(windows)]
    #[test]
    fn directory_junction_entry_exposes_real_target_path() {
        use std::process::Command;

        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-nav-junc-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let real = dir.join("real");
        let members = dir.join("members");
        fs::create_dir_all(&real).unwrap();
        fs::create_dir_all(&members).unwrap();
        fs::write(real.join("marker.txt"), "x").unwrap();
        let link = members.join("alias");
        let real_s = fs::canonicalize(&real)
            .unwrap()
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string();
        let status = Command::new("cmd")
            .args([
                "/C",
                "mklink",
                "/J",
                &link.to_string_lossy(),
                &real_s,
            ])
            .status()
            .unwrap();
        assert!(status.success());

        let listing = list_directory(&members.to_string_lossy(), true).unwrap();
        assert_eq!(listing.entries.len(), 1);
        assert_eq!(listing.entries[0].name, "alias");
        let open = PathBuf::from(&listing.entries[0].path);
        let open_canon = fs::canonicalize(&open).unwrap();
        let real_canon = fs::canonicalize(&real).unwrap();
        assert_eq!(open_canon, real_canon);

        let via_junction = list_directory(&link.to_string_lossy(), true).unwrap();
        let via_canon = fs::canonicalize(PathBuf::from(&via_junction.path)).unwrap();
        assert_eq!(via_canon, real_canon);

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn hidden_entries_are_omitted_until_show_hidden() {
        let dir = std::env::temp_dir().join(format!(
            "a-file-manager-hidden-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let visible = dir.join("visible.txt");
        let hidden = dir.join("secret.txt");
        File::create(&visible).unwrap();
        File::create(&hidden).unwrap();
        let status = std::process::Command::new("attrib")
            .arg("+H")
            .arg(&hidden)
            .status()
            .expect("attrib");
        assert!(status.success());

        let concealed = list_directory(&dir.to_string_lossy(), false).unwrap();
        assert_eq!(concealed.entries.len(), 1);
        assert_eq!(concealed.entries[0].name, "visible.txt");

        let shown = list_directory(&dir.to_string_lossy(), true).unwrap();
        assert_eq!(shown.entries.len(), 2);

        let _ = fs::remove_dir_all(&dir);
    }
}
