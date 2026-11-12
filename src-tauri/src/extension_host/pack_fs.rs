// SPDX-License-Identifier: GPL-3.0-or-later

use super::types::{FeaturePackManifest, FEATURE_PACK_API_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};
use zip::ZipArchive;

const STATE_FILE_NAME: &str = "state.json";
const PACK_MANIFEST_FILE: &str = "pack.json";

/// Feature Pack 启用状态（`{root}/state.json`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PackStateFile {
    #[serde(default)]
    pub enabled: BTreeMap<String, bool>,
}

/// `{app_data_dir}/feature-packs`
pub fn packs_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("feature-packs")
}

/// `{root}/{id}`
pub fn pack_dir(root: &Path, id: &str) -> PathBuf {
    root.join(id)
}

/// 读取并校验目录下的 `pack.json`。
pub fn read_pack_json(dir: &Path) -> Result<FeaturePackManifest, String> {
    let path = dir.join(PACK_MANIFEST_FILE);
    let text = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read pack.json at {}: {err}", path.display()))?;
    let manifest: FeaturePackManifest =
        serde_json::from_str(&text).map_err(|err| format!("Failed to parse pack.json: {err}"))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

/// 写入 `{root}/state.json` 中指定 Pack 的启用标记。
pub fn write_enabled_state(root: &Path, id: &str, enabled: bool) -> Result<(), String> {
    validate_pack_id(id)?;
    let mut state = load_state(root)?;
    state.enabled.insert(id.to_string(), enabled);
    save_state(root, &state)
}

/// 读取 `{root}/state.json`；文件不存在时返回空状态。
pub fn load_state(root: &Path) -> Result<PackStateFile, String> {
    let path = root.join(STATE_FILE_NAME);
    if !path.exists() {
        return Ok(PackStateFile::default());
    }
    let text = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read pack state at {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("Failed to parse pack state: {err}"))
}

/// 校验 `pack.json` 后将源目录拷贝到 `{app_data}/feature-packs/{id}`（先写临时目录再 rename）。
pub fn install_from_directory(
    app_data_dir: &Path,
    source_dir: &Path,
) -> Result<FeaturePackManifest, String> {
    let manifest = read_pack_json(source_dir)?;
    let root = packs_root(app_data_dir);
    fs::create_dir_all(&root)
        .map_err(|err| format!("Failed to create feature-packs directory: {err}"))?;

    let dest = pack_dir(&root, &manifest.id);
    let staging = root.join(format!(".{}.staging-{}", manifest.id, std::process::id()));
    if staging.exists() {
        remove_path(&staging)?;
    }
    if let Err(err) = copy_pack_tree(source_dir, &staging) {
        let _ = remove_path(&staging);
        return Err(err);
    }
    if let Err(err) = commit_staging(&staging, &dest) {
        let _ = remove_path(&staging);
        return Err(err);
    }
    read_pack_json(&dest)
}

/// 将 zip 安全解压到目标目录（拒绝 zip-slip）。
pub fn extract_pack_zip(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dest_dir)
        .map_err(|err| format!("Failed to create zip extract directory: {err}"))?;
    let file = File::open(zip_path)
        .map_err(|err| format!("Failed to open zip {}: {err}", zip_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|err| format!("Invalid Feature Pack zip {}: {err}", zip_path.display()))?;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|err| format!("Failed to read zip entry {i}: {err}"))?;
        let raw_name = entry.name().to_string();
        let Some(rel) = sanitize_zip_entry_path(&raw_name) else {
            return Err(format!("Unsafe zip entry path: {raw_name}"));
        };
        let out_path = dest_dir.join(&rel);
        if entry.is_dir() || raw_name.ends_with('/') || raw_name.ends_with('\\') {
            fs::create_dir_all(&out_path)
                .map_err(|err| format!("Failed to create zip dir {}: {err}", out_path.display()))?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("Failed to create zip parent {}: {err}", parent.display())
            })?;
        }
        let mut outfile = File::create(&out_path)
            .map_err(|err| format!("Failed to create {}: {err}", out_path.display()))?;
        io::copy(&mut entry, &mut outfile)
            .map_err(|err| format!("Failed to extract {}: {err}", out_path.display()))?;
    }
    Ok(())
}

/// 定位解压目录中的 `pack.json` 根（支持 zip 内单层包裹目录）。
pub fn find_pack_root(extracted: &Path) -> Result<PathBuf, String> {
    if extracted.join(PACK_MANIFEST_FILE).is_file() {
        return Ok(extracted.to_path_buf());
    }
    let mut found = Vec::new();
    let entries = fs::read_dir(extracted)
        .map_err(|err| format!("Failed to read extracted pack: {err}"))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("Failed to read extracted entry: {err}"))?;
        let path = entry.path();
        if path.is_dir() && path.join(PACK_MANIFEST_FILE).is_file() {
            found.push(path);
        }
    }
    match found.len() {
        1 => Ok(found.remove(0)),
        0 => Err("pack.json not found in zip".to_string()),
        _ => Err("zip contains multiple pack.json roots".to_string()),
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

/// 删除本机 Pack 目录并清除启用状态。
pub fn uninstall_pack(app_data_dir: &Path, id: &str) -> Result<(), String> {
    validate_pack_id(id)?;
    let root = packs_root(app_data_dir);
    let dest = pack_dir(&root, id);
    if dest.exists() {
        remove_path(&dest)?;
    }
    let mut state = load_state(&root)?;
    if state.enabled.remove(id).is_some() {
        save_state(&root, &state)?;
    }
    Ok(())
}

fn validate_manifest(manifest: &FeaturePackManifest) -> Result<(), String> {
    if manifest.api_version != FEATURE_PACK_API_VERSION {
        return Err(format!(
            "Unsupported Feature Pack apiVersion '{}' (expected {FEATURE_PACK_API_VERSION})",
            manifest.api_version
        ));
    }
    validate_pack_id(&manifest.id)
}

fn validate_pack_id(id: &str) -> Result<(), String> {
    if is_valid_pack_id(id) {
        Ok(())
    } else {
        Err(format!(
            "Invalid Feature Pack id '{id}' (must match ^[A-Za-z0-9._-]+$)"
        ))
    }
}

fn is_valid_pack_id(id: &str) -> bool {
    if id == "." || id == ".." {
        return false;
    }
    !id.is_empty()
        && id.bytes().all(|b| {
            matches!(
                b,
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-'
            )
        })
}

fn save_state(root: &Path, state: &PackStateFile) -> Result<(), String> {
    fs::create_dir_all(root)
        .map_err(|err| format!("Failed to create feature-packs directory: {err}"))?;
    let path = root.join(STATE_FILE_NAME);
    let text = serde_json::to_string_pretty(state)
        .map_err(|err| format!("Failed to serialize pack state: {err}"))?;
    write_text_atomic(&path, &text)
}

fn write_text_atomic(path: &Path, text: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Pack state path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("Failed to create pack state directory: {err}"))?;
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| STATE_FILE_NAME.to_string());
    let tmp = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));
    fs::write(&tmp, text).map_err(|err| {
        let _ = fs::remove_file(&tmp);
        format!("Failed to write pack state staging file: {err}")
    })?;
    if path.exists() {
        let parked = parent.join(format!(".{file_name}.{}.parked", std::process::id()));
        if let Err(err) = fs::rename(path, &parked) {
            let _ = fs::remove_file(&tmp);
            return Err(format!("Failed to park existing pack state: {err}"));
        }
        match fs::rename(&tmp, path) {
            Ok(()) => {
                let _ = fs::remove_file(&parked);
                Ok(())
            }
            Err(err) => {
                let _ = fs::rename(&parked, path);
                let _ = fs::remove_file(&tmp);
                Err(format!("Failed to replace pack state: {err}"))
            }
        }
    } else {
        fs::rename(&tmp, path).map_err(|err| {
            let _ = fs::remove_file(&tmp);
            format!("Failed to commit pack state: {err}")
        })
    }
}

/// 只拷贝普通文件与目录，跳过符号链接与重解析点，避免逃出 Pack 源目录。
fn copy_pack_tree(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|err| format!("Failed to create pack directory: {err}"))?;
    for entry in fs::read_dir(src).map_err(|err| format!("Failed to read pack source: {err}"))? {
        let entry = entry.map_err(|err| format!("Failed to read pack entry: {err}"))?;
        let from = entry.path();
        let meta = fs::symlink_metadata(&from)
            .map_err(|err| format!("Failed to stat pack entry {}: {err}", from.display()))?;
        if is_unsafe_entry(&meta) {
            continue;
        }
        let to = dst.join(entry.file_name());
        if meta.is_dir() {
            copy_pack_tree(&from, &to)?;
        } else if meta.is_file() {
            fs::copy(&from, &to)
                .map_err(|err| format!("Failed to copy pack file {}: {err}", from.display()))?;
        }
    }
    Ok(())
}

fn is_unsafe_entry(meta: &fs::Metadata) -> bool {
    if meta.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

fn commit_staging(staging: &Path, dest: &Path) -> Result<(), String> {
    if dest.exists() {
        let file_name = dest
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "pack".to_string());
        let parent = dest
            .parent()
            .ok_or_else(|| "Pack destination has no parent directory".to_string())?;
        let parked = parent.join(format!(".{file_name}.parked-{}", std::process::id()));
        if parked.exists() {
            remove_path(&parked)?;
        }
        fs::rename(dest, &parked)
            .map_err(|err| format!("Failed to park existing pack directory: {err}"))?;
        match fs::rename(staging, dest) {
            Ok(()) => {
                let _ = remove_path(&parked);
                Ok(())
            }
            Err(err) => {
                let _ = fs::rename(&parked, dest);
                Err(format!("Failed to commit pack directory: {err}"))
            }
        }
    } else {
        fs::rename(staging, dest).map_err(|err| format!("Failed to commit pack directory: {err}"))
    }
}

fn remove_path(path: &Path) -> Result<(), String> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(format!("Failed to stat path {}: {err}", path.display())),
    };
    if meta.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|err| format!("Failed to remove directory {}: {err}", path.display()))
    } else {
        fs::remove_file(path)
            .map_err(|err| format!("Failed to remove file {}: {err}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_pack_json(dir: &Path, json: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(PACK_MANIFEST_FILE), json).unwrap();
    }

    #[test]
    fn install_from_directory_copies_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src-pack");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("pack.json"),
            r#"{"id":"demo.pack","name":"Demo","version":"0.1.0","apiVersion":"feature-pack.1","permissions":[],"contributions":{}}"#,
        )
        .unwrap();
        let app = tmp.path().join("appdata");
        let man = install_from_directory(&app, &src).unwrap();
        assert_eq!(man.id, "demo.pack");
        assert!(pack_dir(&packs_root(&app), "demo.pack")
            .join("pack.json")
            .is_file());
    }

    #[test]
    fn rejects_bad_api_version() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src-pack");
        write_pack_json(
            &src,
            r#"{"id":"demo.pack","name":"Demo","version":"0.1.0","apiVersion":"vnext.1","permissions":[],"contributions":{}}"#,
        );
        let app = tmp.path().join("appdata");
        let err = install_from_directory(&app, &src).unwrap_err();
        assert!(err.contains("apiVersion"));
        assert!(err.contains("feature-pack.1"));
        assert!(!pack_dir(&packs_root(&app), "demo.pack").exists());
    }

    #[test]
    fn rejects_invalid_pack_id() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src-pack");
        write_pack_json(
            &src,
            r#"{"id":"../escape","name":"Demo","version":"0.1.0","apiVersion":"feature-pack.1","permissions":[],"contributions":{}}"#,
        );
        let app = tmp.path().join("appdata");
        let err = install_from_directory(&app, &src).unwrap_err();
        assert!(err.contains("Invalid Feature Pack id"));
        assert!(!packs_root(&app).join("escape").exists());
    }

    #[test]
    fn rejects_dot_segment_pack_id() {
        for bad_id in [".", ".."] {
            let tmp = tempfile::tempdir().unwrap();
            let src = tmp.path().join("src-pack");
            let json = format!(
                r#"{{"id":"{bad_id}","name":"Demo","version":"0.1.0","apiVersion":"feature-pack.1","permissions":[],"contributions":{{}}}}"#,
            );
            write_pack_json(&src, &json);
            let app = tmp.path().join("appdata");
            let err = install_from_directory(&app, &src).unwrap_err();
            assert!(
                err.contains("Invalid Feature Pack id"),
                "id={bad_id}: {err}"
            );
            assert!(!pack_dir(&packs_root(&app), bad_id).exists());
        }
    }

    #[test]
    fn write_enabled_state_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let root = packs_root(tmp.path());
        write_enabled_state(&root, "demo.pack", true).unwrap();
        let state = load_state(&root).unwrap();
        assert_eq!(state.enabled.get("demo.pack"), Some(&true));
        write_enabled_state(&root, "demo.pack", false).unwrap();
        let state = load_state(&root).unwrap();
        assert_eq!(state.enabled.get("demo.pack"), Some(&false));
    }

    #[test]
    fn uninstall_pack_removes_directory_and_state() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src-pack");
        write_pack_json(
            &src,
            r#"{"id":"demo.pack","name":"Demo","version":"0.1.0","apiVersion":"feature-pack.1","permissions":[],"contributions":{}}"#,
        );
        let app = tmp.path().join("appdata");
        install_from_directory(&app, &src).unwrap();
        let root = packs_root(&app);
        write_enabled_state(&root, "demo.pack", true).unwrap();
        uninstall_pack(&app, "demo.pack").unwrap();
        assert!(!pack_dir(&root, "demo.pack").exists());
        assert!(!load_state(&root).unwrap().enabled.contains_key("demo.pack"));
    }

    #[test]
    fn copy_skips_symlinks() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src-pack");
        write_pack_json(
            &src,
            r#"{"id":"demo.pack","name":"Demo","version":"0.1.0","apiVersion":"feature-pack.1","permissions":[],"contributions":{}}"#,
        );
        let secret = tmp.path().join("secret.txt");
        std::fs::write(&secret, "secret").unwrap();
        let link = src.join("escaped.txt");
        let created = {
            #[cfg(windows)]
            {
                std::os::windows::fs::symlink_file(&secret, &link).is_ok()
            }
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&secret, &link).is_ok()
            }
            #[cfg(not(any(windows, unix)))]
            {
                false
            }
        };
        if !created {
            return;
        }
        let app = tmp.path().join("appdata");
        install_from_directory(&app, &src).unwrap();
        assert!(!pack_dir(&packs_root(&app), "demo.pack")
            .join("escaped.txt")
            .exists());
        assert!(pack_dir(&packs_root(&app), "demo.pack")
            .join("pack.json")
            .is_file());
    }

    fn write_pack_zip(path: &Path, json: &str, nested_dir: Option<&str>) {
        use std::io::Write;
        use zip::write::SimpleFileOptions;
        use zip::CompressionMethod;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let entry_name = match nested_dir {
            Some(dir) => format!("{dir}/{PACK_MANIFEST_FILE}"),
            None => PACK_MANIFEST_FILE.to_string(),
        };
        zip.start_file(entry_name, options).unwrap();
        zip.write_all(json.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn extract_nested_zip_then_install_from_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("demo.zip");
        write_pack_zip(
            &zip_path,
            r#"{"id":"demo.pack","name":"Demo","version":"0.1.0","apiVersion":"feature-pack.1","permissions":[],"contributions":{}}"#,
            Some("demo.pack"),
        );
        let extracted = tmp.path().join("extracted");
        extract_pack_zip(&zip_path, &extracted).unwrap();
        let pack_root = find_pack_root(&extracted).unwrap();
        let app = tmp.path().join("appdata");
        let man = install_from_directory(&app, &pack_root).unwrap();
        assert_eq!(man.id, "demo.pack");
        assert!(pack_dir(&packs_root(&app), "demo.pack")
            .join("pack.json")
            .is_file());
    }

    #[test]
    fn extract_pack_zip_rejects_zip_slip() {
        assert!(sanitize_zip_entry_path("../evil.txt").is_none());
        assert!(sanitize_zip_entry_path("ok/a.txt").is_some());
    }
}
