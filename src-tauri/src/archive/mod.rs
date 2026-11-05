// SPDX-License-Identifier: GPL-3.0-or-later
//! ZIP compress / extract v1 (deflate).

use serde::Serialize;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveResult {
    pub ok: bool,
    pub message: String,
    pub output_path: Option<String>,
}

fn fail(msg: impl Into<String>) -> ArchiveResult {
    ArchiveResult {
        ok: false,
        message: msg.into(),
        output_path: None,
    }
}

fn ok(msg: impl Into<String>, output: Option<String>) -> ArchiveResult {
    ArchiveResult {
        ok: true,
        message: msg.into(),
        output_path: output,
    }
}

fn sanitize_zip_entry_path(name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::Normal(s) => out.push(s),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

fn default_zip_name(first: &Path) -> String {
    let stem = first
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("archive");
    format!("{stem}.zip")
}

/// Compress one or more paths into a ZIP next to the first path (or `dest_zip` if given).
#[tauri::command]
pub fn archive_compress_zip(paths: Vec<String>, dest_zip: Option<String>) -> ArchiveResult {
    let cleaned: Vec<PathBuf> = paths
        .into_iter()
        .map(|p| PathBuf::from(p.trim()))
        .filter(|p| !p.as_os_str().is_empty())
        .collect();
    if cleaned.is_empty() {
        return fail("No paths to compress");
    }
    for p in &cleaned {
        if !p.exists() {
            return fail(format!("Path not found: {}", p.display()));
        }
    }

    let out_path = if let Some(dest) = dest_zip.filter(|s| !s.trim().is_empty()) {
        PathBuf::from(dest.trim())
    } else {
        let first = &cleaned[0];
        let parent = first.parent().unwrap_or_else(|| Path::new("."));
        parent.join(default_zip_name(first))
    };

    if out_path.exists() {
        return fail(format!("Target already exists: {}", out_path.display()));
    }
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            if let Err(err) = fs::create_dir_all(parent) {
                return fail(format!("Failed to create parent: {err}"));
            }
        }
    }

    let file = match File::create(&out_path) {
        Ok(f) => f,
        Err(err) => return fail(format!("Failed to create zip: {err}")),
    };
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    for src in &cleaned {
        let root_name = src
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("item"));
        if src.is_file() {
            if let Err(err) = add_file_to_zip(&mut zip, src, &root_name, options) {
                let _ = fs::remove_file(&out_path);
                return fail(err);
            }
        } else if src.is_dir() {
            for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                let rel = match path.strip_prefix(src) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                let name_in_zip = root_name.join(rel);
                if path.is_dir() {
                    let dir_name = format!("{}/", name_in_zip.to_string_lossy().replace('\\', "/"));
                    if let Err(err) = zip.add_directory(dir_name, options) {
                        let _ = fs::remove_file(&out_path);
                        return fail(format!("Failed to add directory: {err}"));
                    }
                } else if path.is_file() {
                    if let Err(err) = add_file_to_zip(&mut zip, path, &name_in_zip, options) {
                        let _ = fs::remove_file(&out_path);
                        return fail(err);
                    }
                }
            }
        }
    }

    if let Err(err) = zip.finish() {
        let _ = fs::remove_file(&out_path);
        return fail(format!("Failed to finish zip: {err}"));
    }

    ok(
        "Compressed".to_string(),
        Some(out_path.to_string_lossy().into_owned()),
    )
}

fn add_file_to_zip(
    zip: &mut ZipWriter<File>,
    path: &Path,
    name_in_zip: &Path,
    options: SimpleFileOptions,
) -> Result<(), String> {
    let name = name_in_zip.to_string_lossy().replace('\\', "/");
    zip.start_file(name, options)
        .map_err(|e| format!("Failed to start zip entry: {e}"))?;
    let mut f = File::open(path).map_err(|e| format!("Failed to open {}: {e}", path.display()))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    zip.write_all(&buf)
        .map_err(|e| format!("Failed to write zip entry: {e}"))?;
    Ok(())
}

/// Extract a ZIP into `dest_dir` (or beside the zip if omitted).
#[tauri::command]
pub fn archive_extract_zip(zip_path: String, dest_dir: Option<String>) -> ArchiveResult {
    let zip_pb = PathBuf::from(zip_path.trim());
    if !zip_pb.is_file() {
        return fail(format!("ZIP not found: {}", zip_pb.display()));
    }
    let out_dir = if let Some(d) = dest_dir.filter(|s| !s.trim().is_empty()) {
        PathBuf::from(d.trim())
    } else {
        let stem = zip_pb
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("extracted");
        zip_pb
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(stem)
    };

    if let Err(err) = fs::create_dir_all(&out_dir) {
        return fail(format!("Failed to create dest dir: {err}"));
    }

    let file = match File::open(&zip_pb) {
        Ok(f) => f,
        Err(err) => return fail(format!("Failed to open zip: {err}")),
    };
    let mut archive = match ZipArchive::new(file) {
        Ok(a) => a,
        Err(err) => return fail(format!("Invalid zip: {err}")),
    };

    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(err) => return fail(format!("Failed to read entry {i}: {err}")),
        };
        let raw_name = entry.name().to_string();
        let Some(rel) = sanitize_zip_entry_path(&raw_name) else {
            return fail(format!("Unsafe zip entry path: {raw_name}"));
        };
        let out_path = out_dir.join(&rel);
        if entry.is_dir() || raw_name.ends_with('/') {
            if let Err(err) = fs::create_dir_all(&out_path) {
                return fail(format!("Failed to create dir {}: {err}", out_path.display()));
            }
            continue;
        }
        if let Some(parent) = out_path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                return fail(format!("Failed to create parent {}: {err}", parent.display()));
            }
        }
        let mut outfile = match File::create(&out_path) {
            Ok(f) => f,
            Err(err) => {
                return fail(format!("Failed to create {}: {err}", out_path.display()));
            }
        };
        if let Err(err) = std::io::copy(&mut entry, &mut outfile) {
            return fail(format!("Failed to extract {}: {err}", out_path.display()));
        }
    }

    ok(
        "Extracted".to_string(),
        Some(out_dir.to_string_lossy().into_owned()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn compress_and_extract_roundtrip() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let root = std::env::temp_dir().join(format!("afm-zip-{stamp}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let src = root.join("hello.txt");
        fs::write(&src, b"zip-v1").unwrap();
        let zip_path = root.join("hello.zip");
        let c = archive_compress_zip(
            vec![src.to_string_lossy().into_owned()],
            Some(zip_path.to_string_lossy().into_owned()),
        );
        assert!(c.ok, "{}", c.message);
        let out = root.join("out");
        let e = archive_extract_zip(
            zip_path.to_string_lossy().into_owned(),
            Some(out.to_string_lossy().into_owned()),
        );
        assert!(e.ok, "{}", e.message);
        let extracted = out.join("hello.txt");
        assert_eq!(fs::read_to_string(extracted).unwrap(), "zip-v1");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_zip_slip() {
        assert!(sanitize_zip_entry_path("../evil.txt").is_none());
        assert!(sanitize_zip_entry_path("ok/a.txt").is_some());
    }
}
