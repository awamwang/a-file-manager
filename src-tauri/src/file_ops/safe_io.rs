// SPDX-License-Identifier: GPL-3.0-or-later
//! Disk-safe copy/move primitives (AIA-275).
//!
//! Invariants:
//! - Prefer same-volume `rename` (atomic on NTFS for a single entry).
//! - Cross-volume: copy → verify → commit staging → only then delete source.
//! - Never delete the destination until a verified staging copy exists.
//! - On interrupt/failure before commit: leave source intact; scrub staging.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use walkdir::WalkDir;

static STAGING_SEQ: AtomicU64 = AtomicU64::new(1);

/// Volume identity for same-volume decisions (drive letter or `\\server\share`).
pub fn volume_id(path: &Path) -> Option<String> {
    let raw = path.to_string_lossy();
    let normalized = raw.replace('/', "\\");
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return None;
    }

    let bytes = trimmed.as_bytes();
    // `\\server\share\...`
    if trimmed.starts_with("\\\\") {
        let rest = &trimmed[2..];
        let mut parts = rest.split('\\').filter(|p| !p.is_empty());
        let server = parts.next()?;
        let share = parts.next()?;
        return Some(format!("\\\\{}\\{}", server.to_ascii_lowercase(), share.to_ascii_lowercase()));
    }

    // `C:\...` or `C:...`
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return Some(format!("{}:", bytes[0].to_ascii_lowercase() as char));
    }

    // Relative / unresolved — treat as unknown (force copy+verify path via rename probe).
    None
}

pub fn same_volume(a: &Path, b: &Path) -> bool {
    match (volume_id(a), volume_id(b)) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

pub fn refuse_volume_root(path: &Path) -> Result<(), String> {
    let parent = path.parent();
    if parent.map(|item| item.as_os_str().is_empty()).unwrap_or(true) {
        return Err(format!(
            "Refusing destructive op on a volume root: {}",
            display_path(path)
        ));
    }
    // `C:\` style after normalize — also catch paths whose components are only a prefix root.
    let comps: Vec<_> = path.components().collect();
    if comps.len() <= 1 {
        return Err(format!(
            "Refusing destructive op on a volume root: {}",
            display_path(path)
        ));
    }
    if comps.len() == 2 {
        if let (Component::Prefix(_), Component::RootDir) = (&comps[0], &comps[1]) {
            return Err(format!(
                "Refusing destructive op on a volume root: {}",
                display_path(path)
            ));
        }
    }
    Ok(())
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}

pub fn remove_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|err| format!("Failed to remove directory: {err}"))
    } else {
        fs::remove_file(path).map_err(|err| format!("Failed to remove file: {err}"))
    }
}

/// Hidden-ish staging sibling next to the final destination name.
pub fn staging_path(parent: &Path, final_name: &str) -> PathBuf {
    let seq = STAGING_SEQ.fetch_add(1, Ordering::Relaxed);
    let safe_name = final_name.replace(['\\', '/', ':'], "_");
    parent.join(format!(
        ".afm-staging-{}-{}-{}",
        std::process::id(),
        seq,
        safe_name
    ))
}

pub fn copy_entry(src: &Path, dest: &Path) -> Result<(), String> {
    if src.is_dir() {
        copy_dir_recursive(src, dest)
    } else {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("Failed to create parent directory: {err}"))?;
        }
        fs::copy(src, dest).map_err(|err| format!("Failed to copy file: {err}"))?;
        Ok(())
    }
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|err| format!("Failed to create directory: {err}"))?;
    for item in fs::read_dir(src).map_err(|err| format!("Failed to read directory: {err}"))? {
        let entry = item.map_err(|err| format!("Failed to read entry: {err}"))?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        copy_entry(&from, &to)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TreeFingerprint {
    files: u64,
    dirs: u64,
    bytes: u64,
}

fn fingerprint(path: &Path) -> Result<TreeFingerprint, String> {
    if path.is_file() {
        let meta = fs::metadata(path).map_err(|err| format!("Failed to stat file: {err}"))?;
        return Ok(TreeFingerprint {
            files: 1,
            dirs: 0,
            bytes: meta.len(),
        });
    }
    if !path.is_dir() {
        return Err(format!("Not a file or directory: {}", display_path(path)));
    }
    let mut files = 0u64;
    let mut dirs = 0u64;
    let mut bytes = 0u64;
    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if p == path {
            dirs += 1;
            continue;
        }
        let ft = entry.file_type();
        if ft.is_dir() {
            dirs += 1;
        } else if ft.is_file() {
            files += 1;
            bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    Ok(TreeFingerprint { files, dirs, bytes })
}

/// Byte-count / entry-count mirror check (durable enough before deleting source).
pub fn verify_mirror(src: &Path, dest: &Path) -> Result<(), String> {
    if !dest.exists() {
        return Err(format!(
            "Verification failed: destination missing: {}",
            display_path(dest)
        ));
    }
    let left = fingerprint(src)?;
    let right = fingerprint(dest)?;
    if left != right {
        return Err(format!(
            "Verification failed: source {:?}, dest {:?}",
            left, right
        ));
    }
    Ok(())
}

/// Commit staging into `dest`. If `replace`, swap out the existing dest only after staging is verified.
fn commit_staging(staging: &Path, dest: &Path, replace: bool) -> Result<(), String> {
    let parent = dest
        .parent()
        .ok_or_else(|| "Destination has no parent".to_string())?;

    if replace && dest.exists() {
        let backup = staging_path(
            parent,
            &format!(
                "replaced-{}",
                dest.file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "item".into())
            ),
        );
        fs::rename(dest, &backup)
            .map_err(|err| format!("Failed to park existing destination: {err}"))?;
        match fs::rename(staging, dest) {
            Ok(()) => {
                let _ = remove_path(&backup);
                Ok(())
            }
            Err(err) => {
                let _ = fs::rename(&backup, dest);
                let _ = remove_path(staging);
                Err(format!("Failed to commit staged move: {err}"))
            }
        }
    } else if dest.exists() {
        let _ = remove_path(staging);
        Err(format!(
            "Destination already exists: {}",
            display_path(dest)
        ))
    } else {
        match fs::rename(staging, dest) {
            Ok(()) => Ok(()),
            Err(err) => {
                let _ = remove_path(staging);
                Err(format!("Failed to commit staged copy: {err}"))
            }
        }
    }
}

fn same_volume_move(src: &Path, dest: &Path, replace: bool) -> Result<(), String> {
    if src == dest {
        return Ok(());
    }
    if replace && dest.exists() {
        let parent = dest
            .parent()
            .ok_or_else(|| "Destination has no parent".to_string())?;
        let backup = staging_path(
            parent,
            &format!(
                "replaced-{}",
                dest.file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "item".into())
            ),
        );
        fs::rename(dest, &backup)
            .map_err(|err| format!("Failed to park existing destination: {err}"))?;
        match fs::rename(src, dest) {
            Ok(()) => {
                let _ = remove_path(&backup);
                Ok(())
            }
            Err(err) => {
                let _ = fs::rename(&backup, dest);
                Err(format!("Failed to move: {err}"))
            }
        }
    } else {
        fs::rename(src, dest).map_err(|err| format!("Failed to move: {err}"))
    }
}

/// Cross-volume move with optional early stop for interrupt-invariant tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossVolumeStop {
    /// Full successful move.
    None,
    /// After verified staging copy — source still intact (crash/cancel before commit).
    AfterVerify,
    /// After dest commit — both may exist briefly (crash before source delete).
    AfterCommit,
}

pub fn cross_volume_move(
    src: &Path,
    dest: &Path,
    replace: bool,
    stop: CrossVolumeStop,
) -> Result<(), String> {
    let parent = dest
        .parent()
        .ok_or_else(|| "Destination has no parent".to_string())?;
    let name = dest
        .file_name()
        .ok_or_else(|| "Destination has no file name".to_string())?
        .to_string_lossy()
        .into_owned();
    let staging = staging_path(parent, &name);
    if staging.exists() {
        remove_path(&staging)?;
    }

    if let Err(err) = copy_entry(src, &staging) {
        let _ = remove_path(&staging);
        return Err(err);
    }
    if let Err(err) = verify_mirror(src, &staging) {
        let _ = remove_path(&staging);
        return Err(err);
    }
    if stop == CrossVolumeStop::AfterVerify {
        // Simulate cancel/crash: scrub staging, leave source untouched.
        let _ = remove_path(&staging);
        return Err("Interrupted after verify (source left intact)".to_string());
    }

    commit_staging(&staging, dest, replace)?;
    if stop == CrossVolumeStop::AfterCommit {
        return Err("Interrupted after commit (destination durable; source retained)".to_string());
    }

    remove_path(src).map_err(|err| {
        format!("Moved copy ok but failed to remove source (destination is durable): {err}")
    })
}

pub fn safe_move_entry(src: &Path, dest: &Path, replace: bool) -> Result<(), String> {
    if src == dest {
        return Ok(());
    }
    refuse_volume_root(src)?;
    refuse_volume_root(dest)?;

    if same_volume(src, dest) {
        match same_volume_move(src, dest, replace) {
            Ok(()) => Ok(()),
            // Rename can still fail on same volume (locks, permissions) — fall back safely.
            Err(_) => cross_volume_move(src, dest, replace, CrossVolumeStop::None),
        }
    } else {
        // Different / unknown volumes: try rename once, then staging copy+verify.
        if !replace && dest.exists() {
            return Err(format!(
                "Destination already exists: {}",
                display_path(dest)
            ));
        }
        match fs::rename(src, dest) {
            Ok(()) => Ok(()),
            Err(_) => cross_volume_move(src, dest, replace, CrossVolumeStop::None),
        }
    }
}

pub fn safe_copy_entry(src: &Path, dest: &Path, replace: bool) -> Result<(), String> {
    if src == dest {
        return Ok(());
    }
    refuse_volume_root(dest)?;
    let parent = dest
        .parent()
        .ok_or_else(|| "Destination has no parent".to_string())?;
    let name = dest
        .file_name()
        .ok_or_else(|| "Destination has no file name".to_string())?
        .to_string_lossy()
        .into_owned();
    let staging = staging_path(parent, &name);
    if staging.exists() {
        remove_path(&staging)?;
    }
    if let Err(err) = copy_entry(src, &staging) {
        let _ = remove_path(&staging);
        return Err(err);
    }
    if let Err(err) = verify_mirror(src, &staging) {
        let _ = remove_path(&staging);
        return Err(err);
    }
    commit_staging(&staging, dest, replace)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_workspace(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "afm-safe-io-{}-{}",
            label,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn volume_id_drive_and_unc() {
        assert_eq!(volume_id(Path::new(r"C:\Users\x")), Some("c:".into()));
        assert_eq!(volume_id(Path::new(r"d:/data")), Some("d:".into()));
        assert_eq!(
            volume_id(Path::new(r"\\Server\Share\folder")),
            Some(r"\\server\share".into())
        );
        assert!(same_volume(Path::new(r"C:\a"), Path::new(r"C:\b")));
        assert!(!same_volume(Path::new(r"C:\a"), Path::new(r"D:\b")));
    }

    #[test]
    fn refuse_volume_roots() {
        assert!(refuse_volume_root(Path::new(r"C:\")).is_err());
        assert!(refuse_volume_root(Path::new(r"D:\")).is_err());
        assert!(refuse_volume_root(Path::new(r"C:\Users")).is_ok());
    }

    #[test]
    fn same_volume_move_leaves_no_source() {
        let dir = temp_workspace("svm");
        let a = dir.join("a");
        let b = dir.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let src = a.join("note.txt");
        fs::write(&src, b"hello").unwrap();
        let dest = b.join("note.txt");
        safe_move_entry(&src, &dest, false).unwrap();
        assert!(!src.exists());
        assert_eq!(fs::read(&dest).unwrap(), b"hello");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn interrupt_after_verify_keeps_source() {
        let dir = temp_workspace("interrupt");
        let a = dir.join("a");
        let b = dir.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let src = a.join("tree");
        fs::create_dir_all(src.join("nested")).unwrap();
        fs::write(src.join("nested").join("x.bin"), b"payload").unwrap();
        let dest = b.join("tree");

        let err = cross_volume_move(&src, &dest, false, CrossVolumeStop::AfterVerify).unwrap_err();
        assert!(err.contains("Interrupted"));
        assert!(src.exists(), "source must survive interrupt");
        assert!(src.join("nested").join("x.bin").is_file());
        assert!(!dest.exists(), "final dest must not appear after scrub");
        // No leftover staging dirs.
        let leftovers: Vec<_> = fs::read_dir(&b)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(".afm-staging-")
            })
            .collect();
        assert!(leftovers.is_empty(), "staging scrubbed on interrupt");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn interrupt_after_commit_keeps_both_complete() {
        let dir = temp_workspace("after-commit");
        let a = dir.join("a");
        let b = dir.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let src = a.join("file.dat");
        let mut f = fs::File::create(&src).unwrap();
        write!(f, "durable").unwrap();
        drop(f);
        let dest = b.join("file.dat");

        let err = cross_volume_move(&src, &dest, false, CrossVolumeStop::AfterCommit).unwrap_err();
        assert!(err.contains("Interrupted"));
        assert!(src.exists(), "source retained when delete skipped");
        assert!(dest.is_file(), "destination durable before source delete");
        assert_eq!(fs::read(&dest).unwrap(), b"durable");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_move_does_not_drop_dest_on_failure_prep() {
        let dir = temp_workspace("replace");
        let a = dir.join("a");
        let b = dir.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(a.join("note.txt"), b"new").unwrap();
        fs::write(b.join("note.txt"), b"old").unwrap();
        safe_move_entry(&a.join("note.txt"), &b.join("note.txt"), true).unwrap();
        assert!(!a.join("note.txt").exists());
        assert_eq!(fs::read(b.join("note.txt")).unwrap(), b"new");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn safe_copy_replace_uses_staging() {
        let dir = temp_workspace("copy-replace");
        let src = dir.join("src.txt");
        let dest = dir.join("dest.txt");
        fs::write(&src, b"fresh").unwrap();
        fs::write(&dest, b"stale").unwrap();
        safe_copy_entry(&src, &dest, true).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"fresh");
        assert!(src.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
