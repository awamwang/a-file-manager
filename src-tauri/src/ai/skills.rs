// SPDX-License-Identifier: GPL-3.0-or-later
//! Public Skill library (AIA-216 / A3 · ADR 0005).
//!
//! Default root: `<config_dir>\.agents\skills` (APPDATA). Override via `ai-config.json`
//! `skillsLibraryRoot`. Import requires root-level `SKILL.md`. No Junction deploy (A4).

use super::agents::{AiConfigDocument, AI_CONFIG_SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SKILL_MANIFEST_FILE: &str = "SKILL.md";

pub const ERR_SKILL_SOURCE_INVALID: &str = "AI_SKILL_SOURCE_INVALID";
pub const ERR_SKILL_MANIFEST_MISSING: &str = "AI_SKILL_MANIFEST_MISSING";
pub const ERR_SKILL_NAME_INVALID: &str = "AI_SKILL_NAME_INVALID";
pub const ERR_SKILL_EXISTS: &str = "AI_SKILL_EXISTS";
pub const ERR_SKILL_UNKNOWN: &str = "AI_SKILL_UNKNOWN";
pub const ERR_SKILL_ROOT_INVALID: &str = "AI_SKILL_ROOT_INVALID";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySkill {
    pub name: String,
    pub path: String,
    pub manifest_path: String,
    pub imported_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillsLibraryStatus {
    pub root: String,
    pub skills: Vec<LibrarySkill>,
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn normalize_path_string(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}

pub fn default_skills_library_root(config_dir: &Path) -> PathBuf {
    config_dir.join(".agents").join("skills")
}

pub fn resolve_skills_library_root(config_dir: &Path, document: &AiConfigDocument) -> PathBuf {
    let trimmed = document.skills_library_root.trim();
    if trimmed.is_empty() {
        default_skills_library_root(config_dir)
    } else {
        PathBuf::from(trimmed)
    }
}

pub fn ensure_library_root(root: &Path) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|err| format!("Failed to create skills library: {err}"))?;
    Ok(())
}

pub fn validate_skill_name_pub(name: &str) -> Result<String, String> {
    validate_skill_name(name)
}

fn validate_skill_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty()
        || name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
        || name == "."
        || name == ".."
    {
        return Err(ERR_SKILL_NAME_INVALID.to_string());
    }
    Ok(name.to_string())
}

fn require_skill_md(dir: &Path) -> Result<PathBuf, String> {
    let manifest = dir.join(SKILL_MANIFEST_FILE);
    if !manifest.is_file() {
        return Err(ERR_SKILL_MANIFEST_MISSING.to_string());
    }
    Ok(manifest)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|err| format!("Failed to create skill dir: {err}"))?;
    for entry in fs::read_dir(src).map_err(|err| format!("Failed to read skill source: {err}"))? {
        let entry = entry.map_err(|err| format!("Failed to read skill entry: {err}"))?;
        let ty = entry
            .file_type()
            .map_err(|err| format!("Failed to stat skill entry: {err}"))?;
        // 不跟随 junction/symlink：源树若回链到 Skill 库根（项目部署常见），跟随会递归炸盘并卡死导入。
        if ty.is_symlink() {
            continue;
        }
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else if ty.is_file() {
            fs::copy(entry.path(), &to)
                .map_err(|err| format!("Failed to copy skill file: {err}"))?;
        }
    }
    Ok(())
}

pub fn list_skills(root: &Path) -> Result<Vec<LibrarySkill>, String> {
    ensure_library_root(root)?;
    let mut out = Vec::new();
    let read = fs::read_dir(root).map_err(|err| format!("Failed to list skills library: {err}"))?;
    for entry in read {
        let entry = entry.map_err(|err| format!("Failed to read skills entry: {err}"))?;
        if !entry
            .file_type()
            .map(|t| t.is_dir())
            .unwrap_or(false)
        {
            continue;
        }
        let path = entry.path();
        let Ok(manifest) = require_skill_md(&path) else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().to_string();
        let meta = fs::metadata(&path).ok();
        let imported_at = meta
            .and_then(|m| m.created().or_else(|_| m.modified()).ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        out.push(LibrarySkill {
            name,
            path: normalize_path_string(&path),
            manifest_path: normalize_path_string(&manifest),
            imported_at,
        });
    }
    out.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    Ok(out)
}

pub fn library_status(config_dir: &Path, document: &AiConfigDocument) -> Result<SkillsLibraryStatus, String> {
    let root = resolve_skills_library_root(config_dir, document);
    ensure_library_root(&root)?;
    Ok(SkillsLibraryStatus {
        root: normalize_path_string(&root),
        skills: list_skills(&root)?,
    })
}

pub fn set_library_root(
    config_dir: &Path,
    document: &AiConfigDocument,
    root: Option<String>,
) -> Result<(AiConfigDocument, SkillsLibraryStatus), String> {
    let mut next = document.clone();
    next.schema_version = AI_CONFIG_SCHEMA_VERSION;
    match root {
        None => next.skills_library_root = String::new(),
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                next.skills_library_root = String::new();
            } else {
                let path = PathBuf::from(trimmed);
                // Allow non-existent yet — ensure on use.
                if path.exists() && !path.is_dir() {
                    return Err(ERR_SKILL_ROOT_INVALID.to_string());
                }
                next.skills_library_root = normalize_path_string(&path);
            }
        }
    }
    let status = library_status(config_dir, &next)?;
    Ok((next, status))
}

pub fn import_skill(
    config_dir: &Path,
    document: &AiConfigDocument,
    source_path: &str,
    name_override: Option<String>,
) -> Result<(SkillsLibraryStatus, LibrarySkill), String> {
    let source = PathBuf::from(source_path.trim());
    if !source.is_dir() {
        return Err(ERR_SKILL_SOURCE_INVALID.to_string());
    }
    require_skill_md(&source)?;

    let name = if let Some(raw) = name_override {
        validate_skill_name(&raw)?
    } else {
        let base = source
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        validate_skill_name(&base)?
    };

    let root = resolve_skills_library_root(config_dir, document);
    ensure_library_root(&root)?;
    let dest = root.join(&name);
    if dest.exists() {
        return Err(ERR_SKILL_EXISTS.to_string());
    }
    copy_dir_recursive(&source, &dest)?;
    // Re-validate after copy.
    let manifest = require_skill_md(&dest)?;
    let skill = LibrarySkill {
        name,
        path: normalize_path_string(&dest),
        manifest_path: normalize_path_string(&manifest),
        imported_at: now_millis(),
    };
    let status = library_status(config_dir, document)?;
    Ok((status, skill))
}

pub fn remove_skill(
    config_dir: &Path,
    document: &AiConfigDocument,
    name: &str,
) -> Result<SkillsLibraryStatus, String> {
    let name = validate_skill_name(name)?;
    let root = resolve_skills_library_root(config_dir, document);
    let dest = root.join(&name);
    if !dest.is_dir() {
        return Err(ERR_SKILL_UNKNOWN.to_string());
    }
    // Only remove if it looks like a skill (has SKILL.md) — safety.
    require_skill_md(&dest)?;
    remove_dir_all_robust(&dest).map_err(|err| format!("Failed to remove skill: {err}"))?;
    library_status(config_dir, document)
}

fn remove_dir_all_robust(path: &Path) -> io::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

/// Save helper used by IPC after mutating ai-config root override.
pub fn save_ai_config(config_dir: &Path, document: &AiConfigDocument) -> Result<(), String> {
    super::agents::save_to_path(config_dir, document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::agents::{load_from_path, AiConfigDocument};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "afm-ai-skills-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_skill(dir: &Path, name: &str, body: &str) -> PathBuf {
        let skill = dir.join(name);
        fs::create_dir_all(&skill).unwrap();
        fs::write(skill.join(SKILL_MANIFEST_FILE), body).unwrap();
        skill
    }

    #[test]
    fn default_root_under_agents_skills() {
        let dir = temp_dir("root");
        let doc = AiConfigDocument::empty();
        let root = resolve_skills_library_root(&dir, &doc);
        assert_eq!(root, dir.join(".agents").join("skills"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn import_requires_skill_md() {
        let dir = temp_dir("manifest");
        let doc = load_from_path(&dir).unwrap();
        let bad = dir.join("src-bad");
        fs::create_dir_all(&bad).unwrap();
        let err = import_skill(&dir, &doc, bad.to_str().unwrap(), None).unwrap_err();
        assert_eq!(err, ERR_SKILL_MANIFEST_MISSING);

        let good_src = write_skill(&dir, "src-good", "# Hello\n");
        let (status, skill) =
            import_skill(&dir, &doc, good_src.to_str().unwrap(), Some("hello-skill".into()))
                .unwrap();
        assert_eq!(skill.name, "hello-skill");
        assert!(status.skills.iter().any(|s| s.name == "hello-skill"));
        assert!(Path::new(&skill.path).join(SKILL_MANIFEST_FILE).is_file());

        let again = import_skill(
            &dir,
            &doc,
            good_src.to_str().unwrap(),
            Some("hello-skill".into()),
        )
        .unwrap_err();
        assert_eq!(again, ERR_SKILL_EXISTS);

        let status = remove_skill(&dir, &doc, "hello-skill").unwrap();
        assert!(!status.skills.iter().any(|s| s.name == "hello-skill"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn custom_root_override() {
        let dir = temp_dir("custom-root");
        let custom = dir.join("lib");
        let doc = load_from_path(&dir).unwrap();
        let (next, status) =
            set_library_root(&dir, &doc, Some(custom.to_string_lossy().into())).unwrap();
        assert_eq!(next.skills_library_root, normalize_path_string(&custom));
        assert_eq!(status.root, normalize_path_string(&custom));
        assert!(custom.is_dir());
        let _ = fs::remove_dir_all(&dir);
    }

    /// FIL-30：源树 junction 回链到库根时，导入必须跳过 reparse point，否则会无限递归卡死。
    #[cfg(windows)]
    #[test]
    fn import_skips_junction_back_into_library() {
        let dir = temp_dir("junction-skip");
        let lib = dir.join("lib");
        fs::create_dir_all(&lib).unwrap();
        let mut doc = load_from_path(&dir).unwrap();
        doc.skills_library_root = normalize_path_string(&lib);
        save_ai_config(&dir, &doc).unwrap();

        let src = dir.join("src-skill");
        fs::create_dir_all(src.join("nested")).unwrap();
        fs::write(src.join(SKILL_MANIFEST_FILE), "# Junction probe\n").unwrap();
        let link = src.join("nested").join("back-to-lib");
        let status = std::process::Command::new("cmd")
            .args([
                "/C",
                "mklink",
                "/J",
                &link.to_string_lossy(),
                &lib.to_string_lossy(),
            ])
            .status()
            .expect("mklink");
        assert!(status.success(), "mklink /J failed");

        let (_status, skill) =
            import_skill(&dir, &doc, src.to_str().unwrap(), Some("junction-probe".into()))
                .expect("import must finish without following junction");
        let dest = PathBuf::from(&skill.path);
        assert!(dest.join(SKILL_MANIFEST_FILE).is_file());
        assert!(dest.join("nested").is_dir());
        // 跳过 junction：目标侧不应出现回链目录，更不应把整个库拷进去。
        assert!(!dest.join("nested").join("back-to-lib").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
